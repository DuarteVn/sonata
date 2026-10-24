//! Shared helpers for core integration tests.
#![allow(dead_code)]

pub mod fake_bin;
pub mod fake_device;

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use sonata_core::paths::DataRoot;
use tempfile::TempDir;

/// A throwaway data root plus a throwaway "projects" folder, both removed on drop.
pub struct Sandbox {
    pub tmp: TempDir,
    pub root: DataRoot,
}

impl Sandbox {
    pub fn new() -> Self {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = DataRoot::at(tmp.path().join("sonata")).expect("data root");
        Self { tmp, root }
    }

    /// Creates `<tmp>/projects/<name>` and returns its path.
    pub fn project(&self, name: &str) -> PathBuf {
        let dir = self.tmp.path().join("projects").join(name);
        std::fs::create_dir_all(&dir).expect("project dir");
        dir
    }
}

pub fn mode(path: &Path) -> u32 {
    std::fs::metadata(path)
        .expect("metadata")
        .permissions()
        .mode()
        & 0o777
}

pub fn inode(path: &Path) -> u64 {
    use std::os::unix::fs::MetadataExt;
    std::fs::metadata(path).expect("metadata").ino()
}

use std::sync::Arc;
use std::time::{Duration, Instant};

use sonata_core::App;
use sonata_core::app::AppOptions;
use sonata_core::events::RecordingSink;

/// Polls `check` until it returns true or `timeout` passes.
pub fn wait_until(timeout: Duration, mut check: impl FnMut() -> bool) -> bool {
    let start = Instant::now();
    while start.elapsed() < timeout {
        if check() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(25));
    }
    check()
}

pub const WAIT: Duration = Duration::from_secs(5);

/// An app with `/bin/sh` as the shell, so tests do not depend on the user's shell config.
pub fn test_app(sb: &Sandbox) -> (Arc<App>, Arc<RecordingSink>) {
    let sink = Arc::new(RecordingSink::default());
    let opts = AppOptions {
        shell: Some("/bin/sh".into()),
        ..AppOptions::default()
    };
    let app = App::start_with(sb.root.clone(), sink.clone(), opts).expect("app");
    (app, sink)
}

/// Path of the built `sonata` CLI (built on first use).
pub fn cli_bin() -> PathBuf {
    static BUILT: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();
    BUILT
        .get_or_init(|| {
            let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
            let status = std::process::Command::new(env!("CARGO"))
                .args(["build", "-q", "-p", "sonata-cli"])
                .current_dir(&root)
                .status()
                .expect("cargo build sonata-cli");
            assert!(status.success());
            root.join("target/debug/sonata")
                .canonicalize()
                .expect("sonata binary")
        })
        .clone()
}

/// An app with the CLI socket on and the CLI folder on every terminal's PATH.
pub fn messaging_app(sb: &Sandbox) -> (Arc<App>, Arc<RecordingSink>) {
    let sink = Arc::new(RecordingSink::default());
    let cli_dir = cli_bin().parent().unwrap().to_path_buf();
    let opts = AppOptions {
        shell: Some("/bin/sh".into()),
        cli_dir: Some(cli_dir),
        socket: true,
        ..AppOptions::default()
    };
    let app = App::start_with(sb.root.clone(), sink.clone(), opts).expect("app");
    (app, sink)
}

pub struct CliOut {
    pub code: i32,
    pub stdout: String,
    pub stderr: String,
}

/// Runs `sonata <args>` as terminal `id` would (same identity environment).
pub fn sonata_as(app: &App, id: uuid::Uuid, args: &[&str]) -> CliOut {
    sonata_with(app, id, args, None)
}

pub fn sonata_with(app: &App, id: uuid::Uuid, args: &[&str], stdin: Option<&str>) -> CliOut {
    use std::io::Write;
    let mut cmd = std::process::Command::new(cli_bin());
    cmd.args(args)
        .env("SONATA_SOCKET", app.socket_path().expect("socket"))
        .env("SONATA_TERMINAL_ID", id.to_string())
        .env(
            "SONATA_TOKEN",
            app.terminals().session_token(id).expect("token"),
        )
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    let mut child = cmd.spawn().expect("spawn sonata");
    if let Some(input) = stdin {
        child
            .stdin
            .take()
            .unwrap()
            .write_all(input.as_bytes())
            .unwrap();
    }
    drop(child.stdin.take());
    let out = child.wait_with_output().expect("wait sonata");
    CliOut {
        code: out.status.code().unwrap_or(-1),
        stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
    }
}

/// Creates an open workspace with named shell terminals, all ready at a prompt.
pub fn shells(app: &App, sb: &Sandbox, names: &[&str]) -> (uuid::Uuid, Vec<uuid::Uuid>) {
    use sonata_core::app::NewTerminal;
    use sonata_core::workspace::manager::CreateWorkspace;
    let ws = app
        .create_workspace(CreateWorkspace {
            name: format!("W{}", names.join("")),
            working_directory: sb.project("p"),
        })
        .unwrap()
        .id;
    app.open_workspace(ws).unwrap();
    let ids: Vec<uuid::Uuid> = names
        .iter()
        .enumerate()
        .map(|(i, n)| {
            let frame = sonata_core::model::node::Frame {
                x: i as f64 * 700.0,
                y: 0.0,
                width: 640.0,
                height: 400.0,
            };
            app.create_terminal(
                ws,
                NewTerminal {
                    frame,
                    name: Some(n.to_string()),
                    ..Default::default()
                },
            )
            .unwrap()
            .id
        })
        .collect();
    for id in &ids {
        assert!(wait_until(WAIT, || app.terminal_ready(*id, Duration::from_millis(300))));
    }
    (ws, ids)
}

/// True when `line` is the output `word`, possibly after a shell prompt that some shells print
/// before the output of input typed early (e.g. dash on Linux: "$ word").
pub fn is_output(line: &str, word: &str) -> bool {
    let line = line.trim_matches(|c: char| c == '\r' || c.is_whitespace());
    line == word || line.rsplit("$ ").next() == Some(word) || line.rsplit("% ").next() == Some(word)
}

/// Runs `git` in `dir` for test setup, isolated from the user's global config. Panics on failure.
pub fn git(dir: &Path, args: &[&str]) -> String {
    let out = std::process::Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .output()
        .expect("run git");
    assert!(
        out.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

/// A throwaway repository at `<tmp>/projects/<name>` on branch `main` with one commit
/// (`a.txt` = "one"). Identity and signing are set locally so the app's own git calls work too.
pub fn git_repo(sb: &Sandbox, name: &str) -> PathBuf {
    let dir = sb.project(name);
    git(&dir, &["init", "-q", "-b", "main"]);
    git(&dir, &["config", "user.name", "Test"]);
    git(&dir, &["config", "user.email", "test@example.com"]);
    git(&dir, &["config", "commit.gpgsign", "false"]);
    commit_file(&dir, "a.txt", "one\n", "init");
    dir
}

/// Writes `file` (relative to `dir`), stages everything, and commits.
pub fn commit_file(dir: &Path, file: &str, content: &str, message: &str) {
    let path = dir.join(file);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("parent dir");
    }
    std::fs::write(&path, content).expect("write file");
    git(dir, &["add", "-A"]);
    git(dir, &["commit", "-q", "-m", message]);
}

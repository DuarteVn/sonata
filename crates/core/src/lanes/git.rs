//! Runs the system `git` (research R1, R2) and parses its machine-readable output.

use std::ffi::OsStr;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// `merge-tree --write-tree` needs 2.38 (research R6).
pub const MIN_VERSION: (u32, u32) = (2, 38);
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum GitError {
    #[error("could not run git: {0}")]
    Spawn(String),
    #[error("git did not finish within {0:?}")]
    Timeout(Duration),
    #[error("{0}")]
    Failed(String),
}

#[derive(Debug, Clone)]
pub struct Output {
    pub code: Option<i32>,
    pub stdout: Vec<u8>,
    pub stderr: String,
}

impl Output {
    pub fn success(&self) -> bool {
        self.code == Some(0)
    }

    pub fn text(&self) -> String {
        String::from_utf8_lossy(&self.stdout).trim().to_string()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Git {
    bin: PathBuf,
}

impl Git {
    pub fn new(bin: impl Into<PathBuf>) -> Self {
        Self { bin: bin.into() }
    }

    pub fn bin(&self) -> &Path {
        &self.bin
    }

    /// `git -C <dir> <args>` with an environment that cannot redirect or prompt.
    pub fn command<S: AsRef<OsStr>>(&self, dir: &Path, args: &[S]) -> Command {
        let mut cmd = Command::new(&self.bin);
        crate::platform::child_env().apply_std(&mut cmd);
        cmd.arg("-C").arg(dir).args(args);
        for var in [
            "GIT_DIR",
            "GIT_WORK_TREE",
            "GIT_INDEX_FILE",
            "GIT_COMMON_DIR",
            "GIT_OBJECT_DIRECTORY",
            "GIT_NAMESPACE",
        ] {
            cmd.env_remove(var);
        }
        cmd.env("GIT_TERMINAL_PROMPT", "0")
            .env("GIT_OPTIONAL_LOCKS", "0")
            .env("LC_ALL", "C");
        cmd
    }

    pub fn run<S: AsRef<OsStr>>(&self, dir: &Path, args: &[S]) -> Result<Output, GitError> {
        self.run_with(dir, args, DEFAULT_TIMEOUT)
    }

    pub fn run_with<S: AsRef<OsStr>>(
        &self,
        dir: &Path,
        args: &[S],
        timeout: Duration,
    ) -> Result<Output, GitError> {
        let mut child = self
            .command(dir, args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| GitError::Spawn(e.to_string()))?;
        let reader = |mut pipe: Box<dyn Read + Send>| {
            std::thread::spawn(move || {
                let mut buf = Vec::new();
                let _ = pipe.read_to_end(&mut buf);
                buf
            })
        };
        let out = reader(Box::new(child.stdout.take().expect("piped stdout")));
        let err = reader(Box::new(child.stderr.take().expect("piped stderr")));
        let deadline = Instant::now() + timeout;
        let status = loop {
            match child.try_wait() {
                Ok(Some(status)) => break status,
                Ok(None) if Instant::now() >= deadline => {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(GitError::Timeout(timeout));
                }
                Ok(None) => std::thread::sleep(Duration::from_millis(5)),
                Err(e) => return Err(GitError::Spawn(e.to_string())),
            }
        };
        Ok(Output {
            code: status.code(),
            stdout: out.join().unwrap_or_default(),
            stderr: String::from_utf8_lossy(&err.join().unwrap_or_default()).into_owned(),
        })
    }

    /// Trimmed stdout on success; git's own message otherwise.
    pub fn ok<S: AsRef<OsStr>>(&self, dir: &Path, args: &[S]) -> Result<String, GitError> {
        let out = self.run(dir, args)?;
        if out.success() {
            Ok(out.text())
        } else {
            Err(GitError::Failed(out.stderr.trim().to_string()))
        }
    }

    pub fn version(&self) -> Result<(u32, u32), GitError> {
        let out = self.ok(Path::new("/"), &["--version"])?;
        parse_version(&out).ok_or_else(|| GitError::Failed(format!("unknown git version: {out}")))
    }
}

pub fn parse_version(text: &str) -> Option<(u32, u32)> {
    let rest = text.trim().strip_prefix("git version ")?;
    let mut parts = rest.split(|c: char| !c.is_ascii_digit());
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next()?.parse().ok()?;
    Some((major, minor))
}

/// Finds a usable git without triggering macOS's install prompt (research R2).
pub fn discover(shell: &Path) -> Result<Git, String> {
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(out) = Git::new(shell).run_with_raw(&["-l", "-c", "command -v git"]) {
        let found = out.text();
        if !found.is_empty() {
            candidates.push(PathBuf::from(found));
        }
    }
    candidates.extend(crate::platform::git_candidates().iter().map(PathBuf::from));
    let shim = crate::platform::git_shim();
    let chosen = candidates
        .into_iter()
        .find(|p| p.is_absolute() && shim.is_none_or(|(s, _)| p != s) && p.is_file())
        .or_else(|| {
            shim.filter(|(s, usable)| *usable && s.is_file())
                .map(|(s, _)| s.to_path_buf())
        })
        .ok_or_else(|| crate::platform::git_install_hint().to_string())?;
    let git = Git::new(chosen);
    let version = git
        .version()
        .map_err(|e| format!("Git could not run: {e}"))?;
    if version < MIN_VERSION {
        return Err(format!(
            "Lanes need git {}.{} or newer; this {} has {}.{}.",
            MIN_VERSION.0,
            MIN_VERSION.1,
            crate::platform::machine_name(),
            version.0,
            version.1
        ));
    }
    Ok(git)
}

impl Git {
    /// Runs the binary itself (no `-C`); used only to ask the login shell where git is.
    fn run_with_raw(&self, args: &[&str]) -> Result<Output, GitError> {
        let mut cmd = Command::new(&self.bin);
        crate::platform::child_env().apply_std(&mut cmd);
        let out = cmd
            .args(args)
            .stdin(Stdio::null())
            .stderr(Stdio::null())
            .output()
            .map_err(|e| GitError::Spawn(e.to_string()))?;
        Ok(Output {
            code: out.status.code(),
            stdout: out.stdout,
            stderr: String::new(),
        })
    }
}

/// Paths from `status --porcelain=v2 -z`: changed, unmerged, and untracked entries.
pub fn parse_status_z(bytes: &[u8]) -> Vec<String> {
    let mut paths = Vec::new();
    let mut fields = bytes.split(|b| *b == 0).filter(|f| !f.is_empty());
    while let Some(field) = fields.next() {
        let text = String::from_utf8_lossy(field);
        let path = match text.as_bytes().first() {
            Some(b'1') => text.splitn(9, ' ').nth(8),
            Some(b'2') => {
                let p = text.splitn(10, ' ').nth(9).map(str::to_string);
                fields.next(); // original path of a rename or copy
                paths.extend(p);
                continue;
            }
            Some(b'u') => text.splitn(11, ' ').nth(10),
            Some(b'?') => text.get(2..),
            _ => None,
        };
        paths.extend(path.map(str::to_string));
    }
    paths
}

/// Worktree paths from `worktree list --porcelain -z`, main checkout first.
pub fn parse_worktrees_z(bytes: &[u8]) -> Vec<PathBuf> {
    bytes
        .split(|b| *b == 0)
        .filter_map(|f| f.strip_prefix(b"worktree "))
        .map(|p| PathBuf::from(String::from_utf8_lossy(p).into_owned()))
        .collect()
}

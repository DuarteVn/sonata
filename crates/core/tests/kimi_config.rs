//! Sonata's block in Kimi's config file (feature 010, research R4, contracts/kimi-config.md).
//! Every test uses a temporary Kimi folder and a fake login shell; `~/.kimi-code` is never read.

mod common;

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use common::{fake_bin::fake_bin, mode};
use sonata_core::ErrorCode;
use sonata_core::agents::kimi::{self, KimiStatus};

fn fixture(name: &str) -> String {
    std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/kimi")
            .join(name),
    )
    .unwrap()
}

/// A temporary Kimi folder plus a fake login shell. The shell runs its `-c` script with `bin`
/// first on `PATH`, like `$SHELL -l -c` would with the user's PATH.
struct Env {
    tmp: tempfile::TempDir,
    home: PathBuf,
    bin: PathBuf,
    shell: PathBuf,
}

impl Env {
    fn new() -> Self {
        let tmp = tempfile::tempdir().unwrap();
        let home = tmp.path().join("kimi-home");
        let bin = tmp.path().join("bin");
        std::fs::create_dir_all(&home).unwrap();
        std::fs::create_dir_all(&bin).unwrap();
        let shell = fake_bin(
            &tmp.path().join("shell"),
            "sh",
            &format!(
                "shift 2\nPATH=\"{}:/usr/bin:/bin\" exec /bin/sh -c \"$1\"",
                bin.display()
            ),
        );
        Self {
            tmp,
            home,
            bin,
            shell,
        }
    }

    fn config(&self) -> PathBuf {
        self.home.join("config.toml")
    }

    fn write(&self, text: &str, mode: u32) {
        std::fs::write(self.config(), text).unwrap();
        std::fs::set_permissions(self.config(), std::fs::Permissions::from_mode(mode)).unwrap();
    }

    fn read(&self) -> String {
        std::fs::read_to_string(self.config()).unwrap()
    }

    /// A fake `kimi` whose `doctor` follows the word in `<tmp>/doctor-mode`:
    /// `ok`, `fail`, `fail-after` (fails once Sonata's lines are in the file), or `hang`.
    fn fake_kimi(&self, doctor_mode: &str) {
        let mode_file = self.tmp.path().join("doctor-mode");
        std::fs::write(&mode_file, doctor_mode).unwrap();
        fake_bin(
            &self.bin,
            "kimi",
            &format!(
                r#"[ "$1" = doctor ] || exit 2
case $(cat "{m}") in
  ok) echo "config ok"; exit 0;;
  fail) echo "error: config.toml line 3: expected '='"; exit 1;;
  fail-after) if grep -q "sonata _hook kimi" "$KIMI_CODE_HOME/config.toml"; then echo "error: hooks rejected"; exit 1; fi; exit 0;;
  hang) sleep 30;;
esac"#,
                m = mode_file.display()
            ),
        );
    }

    fn backups(&self) -> Vec<PathBuf> {
        let mut v: Vec<PathBuf> = std::fs::read_dir(&self.home)
            .unwrap()
            .map(|e| e.unwrap().path())
            .filter(|p| {
                p.file_name()
                    .unwrap()
                    .to_string_lossy()
                    .starts_with("config.toml.sonata-backup-")
            })
            .collect();
        v.sort();
        v
    }
}

// ---- Status (T024) ----

#[test]
fn status_of_every_fixture() {
    for (name, status) in [
        ("empty.toml", KimiStatus::Off),
        ("user.toml", KimiStatus::Off),
        ("on-v1.toml", KimiStatus::On),
        ("rewritten.toml", KimiStatus::On),
        ("outdated.toml", KimiStatus::Outdated),
        ("inline-hooks.toml", KimiStatus::Off),
        ("hooks-table.toml", KimiStatus::Off),
    ] {
        assert_eq!(kimi::status(&fixture(name)), status, "{name}");
    }
    for name in ["inline-hooks.toml", "hooks-table.toml"] {
        let msg = kimi::conflict(&fixture(name)).expect(name);
        assert!(msg.contains("line 2") || msg.contains("line 3"), "{msg}");
    }
    for name in ["user.toml", "on-v1.toml", "outdated.toml", "empty.toml"] {
        assert!(kimi::conflict(&fixture(name)).is_none(), "{name}");
    }
}

/// The block's commands behave as Kimi runs them: TOML string, then `/bin/sh -c` (G1).
#[test]
fn block_commands_run_as_kimi_runs_them() {
    let tmp = tempfile::tempdir().unwrap();
    let log = tmp.path().join("sonata.log");
    let bin = tmp.path().join("bin");
    fake_bin(
        &bin,
        "sonata",
        &format!(
            "echo \"$@\" >> \"{l}\"; cat >> \"{l}\"; echo >> \"{l}\"; exit \"${{FAKE_EXIT:-0}}\"",
            l = log.display()
        ),
    );
    let commands = kimi::block_commands();
    assert_eq!(commands.len(), 5);
    let run = |command: &str, terminal: Option<&str>, exit: &str| {
        let mut cmd = Command::new("/bin/sh");
        cmd.args(["-c", command])
            .env("PATH", format!("{}:/usr/bin:/bin", bin.display()))
            .env("FAKE_EXIT", exit)
            .env_remove("SONATA_TERMINAL_ID")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        if let Some(t) = terminal {
            cmd.env("SONATA_TERMINAL_ID", t);
        }
        let mut child = cmd.spawn().unwrap();
        use std::io::Write;
        child
            .stdin
            .take()
            .unwrap()
            .write_all(br#"{"session_id":"session_x"}"#)
            .unwrap();
        child.wait_with_output().unwrap()
    };
    for (event, command) in &commands {
        let out = run(command, None, "0");
        assert!(out.status.success(), "{event}");
        assert!(
            out.stdout.is_empty() && out.stderr.is_empty(),
            "{event}: {out:?}"
        );
        assert!(!log.exists(), "{event}: sonata called outside Sonata");
    }
    for (event, command) in &commands {
        let _ = std::fs::remove_file(&log);
        let out = run(command, Some("t1"), "1");
        assert!(
            out.status.success(),
            "{event}: a failing report must not block Kimi"
        );
        assert!(out.stdout.is_empty(), "{event}");
        let logged = std::fs::read_to_string(&log).unwrap();
        assert!(logged.starts_with("_hook kimi "), "{event}: {logged}");
        assert!(logged.contains("session_x"), "{event}: {logged}");
    }
}

// ---- Path (T025) ----

#[test]
fn kimi_home_from_the_login_shell() {
    let tmp = tempfile::tempdir().unwrap();
    let custom = tmp.path().join("custom");
    let shell = fake_bin(
        tmp.path(),
        "sh-custom",
        &format!(
            "shift 2\nKIMI_CODE_HOME=\"{}\" exec /bin/sh -c \"$1\"",
            custom.display()
        ),
    );
    assert_eq!(kimi::kimi_home(&shell), custom);
    let shell = fake_bin(
        tmp.path(),
        "sh-plain",
        "shift 2\nunset KIMI_CODE_HOME\nexec /bin/sh -c \"$1\"",
    );
    let home = std::env::var_os("HOME").map(PathBuf::from).unwrap();
    assert_eq!(kimi::kimi_home(&shell), home.join(".kimi-code"));
}

#[test]
fn unavailable_when_the_folder_is_missing_or_unreadable() {
    let env = Env::new();
    let missing = env.tmp.path().join("nope");
    let i = kimi::integration(&missing, &env.shell);
    assert_eq!(i.status, KimiStatus::Unavailable);
    assert!(i.message.unwrap().contains("Run Kimi once first"));
    assert!(!i.can_apply);
    let err = kimi::apply(&missing, &env.shell).unwrap_err();
    assert_eq!(err.code, ErrorCode::KimiUnavailable);
    assert!(!missing.exists());

    std::fs::create_dir_all(env.config()).unwrap(); // a folder where the file should be
    let i = kimi::integration(&env.home, &env.shell);
    assert_eq!(i.status, KimiStatus::Unavailable);
    assert!(i.message.is_some());
}

// ---- Apply and remove (T027, T028) ----

#[test]
fn apply_appends_only_the_block() {
    let env = Env::new();
    let user = fixture("user.toml");
    env.write(&user, 0o600);
    let i = kimi::apply(&env.home, &env.shell).unwrap();
    assert_eq!(i.status, KimiStatus::On);
    assert_eq!(env.read(), fixture("on-v1.toml"));
    assert!(env.read().starts_with(&user));
    assert_eq!(mode(&env.config()), 0o600);
    assert_eq!(i.config_path, env.config());
    assert!(!i.validated, "no kimi on PATH");
}

#[test]
fn apply_replaces_an_outdated_block_and_keeps_the_mode() {
    let env = Env::new();
    env.write(&fixture("outdated.toml"), 0o644);
    kimi::apply(&env.home, &env.shell).unwrap();
    let text = env.read();
    assert_eq!(kimi::status(&text), KimiStatus::On);
    assert!(!text.contains("timeout = 30"));
    assert!(text.starts_with(&fixture("user.toml")));
    assert_eq!(mode(&env.config()), 0o644);
}

#[test]
fn apply_refuses_a_conflicting_hooks_definition() {
    for name in ["inline-hooks.toml", "hooks-table.toml"] {
        let env = Env::new();
        env.write(&fixture(name), 0o600);
        let err = kimi::apply(&env.home, &env.shell).unwrap_err();
        assert_eq!(err.code, ErrorCode::KimiHooksConflict, "{name}");
        assert_eq!(env.read(), fixture(name));
        assert!(env.backups().is_empty());
        assert!(!kimi::integration(&env.home, &env.shell).can_apply);
    }
}

#[test]
fn apply_creates_a_private_file_when_the_folder_exists() {
    let env = Env::new();
    kimi::apply(&env.home, &env.shell).unwrap();
    assert_eq!(env.read(), kimi::BLOCK_V1);
    assert_eq!(mode(&env.config()), 0o600);
    assert!(env.backups().is_empty(), "nothing to back up");
}

#[test]
fn remove_takes_out_only_sonata_lines() {
    for name in ["on-v1.toml", "rewritten.toml"] {
        let env = Env::new();
        env.write(&fixture(name), 0o600);
        let i = kimi::remove(&env.home, &env.shell).unwrap();
        assert_eq!(i.status, KimiStatus::Off, "{name}");
        assert_eq!(env.read(), format!("{}\n", fixture("user.toml")), "{name}");
        assert!(env.read().contains("event = \"Notification\""));
    }
}

// ---- Backups (T029) ----

#[test]
fn every_change_writes_a_backup_and_three_are_kept() {
    let env = Env::new();
    env.write(&fixture("user.toml"), 0o640);
    std::fs::write(env.home.join("config.toml.bak"), "mine").unwrap();
    let i = kimi::apply(&env.home, &env.shell).unwrap();
    let backups = env.backups();
    assert_eq!(backups.len(), 1);
    assert_eq!(
        std::fs::read_to_string(&backups[0]).unwrap(),
        fixture("user.toml")
    );
    assert_eq!(mode(&backups[0]), 0o640);
    assert_eq!(i.last_backup.as_deref(), Some(backups[0].as_path()));
    let name = backups[0]
        .file_name()
        .unwrap()
        .to_string_lossy()
        .into_owned();
    let stamp = name.trim_start_matches("config.toml.sonata-backup-");
    assert!(
        stamp.len() >= 15 && stamp.as_bytes()[8] == b'-',
        "YYYYMMDD-HHMMSS: {name}"
    );

    for _ in 0..4 {
        kimi::remove(&env.home, &env.shell).unwrap();
        kimi::apply(&env.home, &env.shell).unwrap();
    }
    let backups = env.backups();
    assert_eq!(backups.len(), 3, "{backups:?}");
    let i = kimi::integration(&env.home, &env.shell);
    let newest = i.last_backup.unwrap();
    // The newest backup holds the file as it was before the last change (block removed).
    assert_eq!(
        kimi::status(&std::fs::read_to_string(&newest).unwrap()),
        KimiStatus::Off
    );
    assert_eq!(
        std::fs::read_to_string(env.home.join("config.toml.bak")).unwrap(),
        "mine",
        "other files are never touched"
    );
}

// ---- kimi doctor (T030) ----

#[test]
fn doctor_failure_before_any_change_stops_apply_and_remove() {
    let env = Env::new();
    env.write(&fixture("user.toml"), 0o600);
    env.fake_kimi("fail");
    let err = kimi::apply(&env.home, &env.shell).unwrap_err();
    assert_eq!(err.code, ErrorCode::KimiConfigInvalid);
    assert!(err.message.contains("expected '='"), "{}", err.message);
    assert_eq!(env.read(), fixture("user.toml"));
    assert!(env.backups().is_empty());

    env.write(&fixture("on-v1.toml"), 0o600);
    let err = kimi::remove(&env.home, &env.shell).unwrap_err();
    assert_eq!(err.code, ErrorCode::KimiConfigInvalid);
    assert_eq!(env.read(), fixture("on-v1.toml"));
}

#[test]
fn doctor_failure_after_writing_restores_the_backup() {
    let env = Env::new();
    env.write(&fixture("user.toml"), 0o600);
    env.fake_kimi("fail-after");
    let err = kimi::apply(&env.home, &env.shell).unwrap_err();
    assert_eq!(err.code, ErrorCode::KimiCheckFailed);
    assert!(err.message.contains("hooks rejected"), "{}", err.message);
    assert_eq!(env.read(), fixture("user.toml"));
    assert_eq!(mode(&env.config()), 0o600);
}

#[test]
fn doctor_success_keeps_the_change_and_marks_it_validated() {
    let env = Env::new();
    env.write(&fixture("user.toml"), 0o600);
    env.fake_kimi("ok");
    let i = kimi::apply(&env.home, &env.shell).unwrap();
    assert_eq!(i.status, KimiStatus::On);
    assert!(i.validated);
}

#[test]
fn hanging_doctor_counts_as_failure() {
    let env = Env::new();
    env.write(&fixture("user.toml"), 0o600);
    env.fake_kimi("hang");
    let start = Instant::now();
    let err = kimi::apply_with(&env.home, &env.shell, Duration::from_millis(500)).unwrap_err();
    assert!(start.elapsed() < Duration::from_secs(5));
    assert_eq!(err.code, ErrorCode::KimiConfigInvalid);
    assert_eq!(env.read(), fixture("user.toml"));
    assert_eq!(kimi::DOCTOR_TIMEOUT, Duration::from_secs(10));
}

// ---- App (T034) ----

fn kimi_app(
    env: &Env,
) -> (
    std::sync::Arc<sonata_core::App>,
    std::sync::Arc<sonata_core::events::RecordingSink>,
    common::Sandbox,
) {
    use sonata_core::app::AppOptions;
    let sb = common::Sandbox::new();
    let sink = std::sync::Arc::new(sonata_core::events::RecordingSink::default());
    let opts = AppOptions {
        shell: Some(env.shell.clone()),
        kimi_home: Some(env.home.clone()),
        ..AppOptions::default()
    };
    let app = sonata_core::App::start_with(sb.root.clone(), sink.clone(), opts).unwrap();
    (app, sink, sb)
}

fn kimi_events(sink: &sonata_core::events::RecordingSink) -> Vec<KimiStatus> {
    use sonata_core::events::CoreEvent;
    sink.take()
        .into_iter()
        .filter_map(|e| match e {
            CoreEvent::KimiIntegrationChanged(i) => Some(i.status),
            _ => None,
        })
        .collect()
}

#[test]
fn app_applies_removes_and_reports_each_change_once() {
    let env = Env::new();
    env.write(&fixture("user.toml"), 0o600);
    let (app, sink, _sb) = kimi_app(&env);
    assert_eq!(app.kimi_integration().status, KimiStatus::Off);
    sink.take();
    assert_eq!(app.kimi_integration_apply().unwrap().status, KimiStatus::On);
    assert_eq!(kimi_events(&sink), [KimiStatus::On]);
    assert_eq!(
        app.kimi_integration_remove().unwrap().status,
        KimiStatus::Off
    );
    assert_eq!(kimi_events(&sink), [KimiStatus::Off]);
    app.quit();
}

#[test]
fn app_reports_outside_edits_only_while_watching() {
    use common::wait_until;
    let env = Env::new();
    env.write(&fixture("user.toml"), 0o600);
    let (app, sink, _sb) = kimi_app(&env);
    app.kimi_integration();
    sink.take();

    // Not watched: no Settings window and no Kimi terminal.
    env.write(&fixture("on-v1.toml"), 0o600);
    std::thread::sleep(Duration::from_millis(1200));
    assert!(kimi_events(&sink).is_empty());

    app.kimi_watch(true);
    std::thread::sleep(Duration::from_millis(300));
    sink.take();
    env.write(&fixture("outdated.toml"), 0o600);
    let mut seen = Vec::new();
    assert!(wait_until(Duration::from_secs(3), || {
        seen.extend(kimi_events(&sink));
        seen.contains(&KimiStatus::Outdated)
    }));

    app.kimi_watch(false);
    std::thread::sleep(Duration::from_millis(300));
    sink.take();
    env.write(&fixture("user.toml"), 0o600);
    std::thread::sleep(Duration::from_millis(1200));
    assert!(kimi_events(&sink).is_empty());
    app.quit();
}

#[test]
fn app_saves_the_notice_dismissal() {
    use sonata_core::model::preferences::KimiNoticeDismissed;
    use sonata_core::persistence::store::Store;
    let env = Env::new();
    let (app, _sink, sb) = kimi_app(&env);
    app.dismiss_kimi_notice(KimiNoticeDismissed::Outdated)
        .unwrap();
    assert_eq!(
        Store::new(sb.root.clone())
            .load_preferences()
            .value
            .kimi_notice_dismissed,
        Some(KimiNoticeDismissed::Outdated)
    );
    app.quit();
}

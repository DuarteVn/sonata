mod common;

use std::path::PathBuf;
use std::sync::Arc;

use common::{Sandbox, WAIT, wait_until};
use sonata_core::events::NullSink;
use sonata_core::terminal::manager::{RunStatus, SpawnSpec, TerminalManager};
use uuid::Uuid;

fn manager() -> TerminalManager {
    TerminalManager::new(Arc::new(NullSink))
}

fn spec(cwd: PathBuf) -> SpawnSpec {
    SpawnSpec {
        node_id: Uuid::new_v4(),
        cwd,
        launch_command: None,
        rows: 30,
        cols: 100,
        history_max_rows: 10_000,
        shell: Some("/bin/sh".into()),
        restored_history: None,
    }
}

fn screen_has(m: &TerminalManager, id: Uuid, needle: &str) -> bool {
    wait_until(WAIT, || {
        m.screen_text(id).is_some_and(|t| t.contains(needle))
    })
}

#[test]
fn starts_in_working_directory() {
    let sb = Sandbox::new();
    let dir = sb.project("here").canonicalize().unwrap();
    let m = manager();
    let s = spec(dir.clone());
    let id = s.node_id;
    m.start(s).unwrap();
    m.write(id, b"pwd -P\r").unwrap();
    assert!(
        screen_has(&m, id, dir.to_str().unwrap()),
        "{:?}",
        m.screen_text(id)
    );
    m.stop(id);
}

#[test]
fn missing_directory_falls_back_to_home_with_warning() {
    let sb = Sandbox::new();
    let m = manager();
    let s = spec(sb.tmp.path().join("gone"));
    let id = s.node_id;
    let info = m.start(s).unwrap();
    assert!(info.warning.as_deref().unwrap_or_default().contains("gone"));
    let home = PathBuf::from(std::env::var("HOME").unwrap())
        .canonicalize()
        .unwrap();
    m.write(id, b"pwd -P\r").unwrap();
    assert!(screen_has(&m, id, home.to_str().unwrap()));
    m.stop(id);
}

#[test]
fn launch_command_runs_once_and_shell_stays_open() {
    let sb = Sandbox::new();
    let m = manager();
    let mut s = spec(sb.project("a"));
    s.launch_command = Some("echo hello-from-launch".into());
    let id = s.node_id;
    m.start(s).unwrap();
    assert!(
        screen_has(&m, id, "\nhello-from-launch"),
        "{:?}",
        m.screen_text(id)
    );
    std::thread::sleep(std::time::Duration::from_millis(200));
    assert!(matches!(m.status(id), Some(RunStatus::Running { .. })));
    let text = m.screen_text(id).unwrap();
    assert_eq!(text.matches("\nhello-from-launch").count(), 1, "{text}");
    m.stop(id);
}

#[test]
fn environment_is_set() {
    let sb = Sandbox::new();
    let m = manager();
    let s = spec(sb.project("a"));
    let id = s.node_id;
    m.start(s).unwrap();
    m.write(id, b"echo \"[$TERM|$COLORTERM|$TERM_PROGRAM|$SONATA_TERMINAL_ID|${TERM_PROGRAM_VERSION:+v}]\"\r")
        .unwrap();
    let expected = format!("[xterm-256color|truecolor|Sonata|{id}|v]");
    assert!(screen_has(&m, id, &expected), "{:?}", m.screen_text(id));
    m.stop(id);
}

#[test]
fn resize_reaches_the_program() {
    let sb = Sandbox::new();
    let m = manager();
    let s = spec(sb.project("a"));
    let id = s.node_id;
    m.start(s).unwrap();
    m.write(id, b"stty size\r").unwrap();
    assert!(screen_has(&m, id, "30 100"));
    m.resize(id, 40, 120).unwrap();
    m.write(id, b"stty size\r").unwrap();
    assert!(screen_has(&m, id, "40 120"), "{:?}", m.screen_text(id));
    m.stop(id);
}

#[test]
fn exit_code_is_reported_and_restart_starts_new_shell() {
    let sb = Sandbox::new();
    let m = manager();
    let s = spec(sb.project("a"));
    let id = s.node_id;
    m.start(s).unwrap();
    m.write(id, b"exit 3\r").unwrap();
    assert!(
        wait_until(WAIT, || m.status(id) == Some(RunStatus::Exited { code: 3 })),
        "{:?}",
        m.status(id)
    );
    m.restart(id).unwrap();
    assert!(wait_until(WAIT, || matches!(
        m.status(id),
        Some(RunStatus::Running { .. })
    )));
    m.write(id, b"echo again\r").unwrap();
    assert!(screen_has(&m, id, "\nagain"));
    m.stop(id);
}

#[test]
fn busy_while_a_program_runs() {
    let sb = Sandbox::new();
    let m = manager();
    let s = spec(sb.project("a"));
    let id = s.node_id;
    m.start(s).unwrap();
    assert!(wait_until(WAIT, || m
        .status(id)
        .is_some_and(|s| matches!(s, RunStatus::Running { .. }))));
    std::thread::sleep(std::time::Duration::from_millis(300));
    assert!(!m.is_busy(id), "idle prompt reported busy");
    m.write(id, b"sleep 5\r").unwrap();
    assert!(wait_until(WAIT, || m.is_busy(id)), "sleep not detected");
    m.write(id, b"\x03").unwrap();
    assert!(
        wait_until(WAIT, || !m.is_busy(id)),
        "still busy after Ctrl-C"
    );
    m.stop(id);
}

#[test]
fn login_shell_resolves_to_an_executable() {
    let shell = sonata_core::terminal::pty::login_shell();
    assert!(shell.is_absolute() && shell.exists(), "{shell:?}");
}

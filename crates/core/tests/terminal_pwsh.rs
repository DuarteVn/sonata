//! PowerShell and Command Prompt terminals in ConPTY (feature 012, US2, research R7, R10).
#![cfg(windows)]
mod common;

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use common::{Sandbox, WAIT, wait_until};
use sonata_core::events::NullSink;
use sonata_core::terminal::manager::{SpawnSpec, TerminalManager};
use uuid::Uuid;

fn windows_powershell() -> PathBuf {
    PathBuf::from(std::env::var_os("SystemRoot").unwrap())
        .join(r"System32\WindowsPowerShell\v1.0\powershell.exe")
}

fn spec(cwd: PathBuf, shell: PathBuf) -> SpawnSpec {
    SpawnSpec {
        node_id: Uuid::new_v4(),
        cwd,
        launch_command: None,
        rows: 30,
        cols: 100,
        history_max_rows: 10_000,
        shell: Some(shell),
        restored_history: None,
        session_env: Vec::new(),
        wrap_agents: false,
        shell_helpers: None,
        resume: None,
        shell_choice: None,
    }
}

fn screen_has(m: &TerminalManager, id: Uuid, needle: &str, wait: Duration) -> bool {
    wait_until(wait, || {
        m.screen_text(id).is_some_and(|t| t.contains(needle))
    })
}

#[test]
fn powershell_runs_commands_in_the_working_folder() {
    let sb = Sandbox::new();
    let dir = sb.project("here");
    let m = TerminalManager::new(Arc::new(NullSink));
    let s = spec(dir.clone(), windows_powershell());
    let id = s.node_id;
    m.start(s).unwrap();
    m.write(
        id,
        b"Write-Output \"hello-$(40+2)\"; $env:SONATA_TERMINAL_ID; (Get-Location).Path\r",
    )
    .unwrap();
    assert!(
        screen_has(&m, id, "hello-42", Duration::from_secs(30)),
        "{:?}",
        m.screen_text(id)
    );
    assert!(
        screen_has(&m, id, &id.to_string(), WAIT),
        "{:?}",
        m.screen_text(id)
    );
    assert!(screen_has(&m, id, "here", WAIT), "{:?}", m.screen_text(id));
    m.resize(id, 40, 120).unwrap();
    m.stop(id);
}

#[test]
fn launch_command_runs_in_command_prompt() {
    let sb = Sandbox::new();
    let m = TerminalManager::new(Arc::new(NullSink));
    let mut s = spec(sb.project("a"), common::test_shell());
    s.launch_command = Some("echo hello-from-launch".into());
    let id = s.node_id;
    m.start(s).unwrap();
    assert!(
        screen_has(&m, id, "hello-from-launch", Duration::from_secs(20)),
        "{:?}",
        m.screen_text(id)
    );
    m.stop(id);
}

#[test]
fn busy_while_a_child_program_runs() {
    let sb = Sandbox::new();
    let m = TerminalManager::new(Arc::new(NullSink));
    let s = spec(sb.project("b"), common::test_shell());
    let id = s.node_id;
    m.start(s).unwrap();
    assert!(wait_until(WAIT, || m.output_seq(id).unwrap_or(0) > 0));
    std::thread::sleep(Duration::from_millis(500));
    assert!(!m.is_busy(id), "idle shell reported busy");
    m.write(id, b"ping -n 6 127.0.0.1\r").unwrap();
    assert!(wait_until(WAIT, || m.is_busy(id)), "running ping not busy");
    assert!(
        wait_until(Duration::from_secs(15), || !m.is_busy(id)),
        "still busy after ping"
    );
    m.stop(id);
}

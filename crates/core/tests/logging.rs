//! Terminal content must never reach the log files.

mod common;

use common::{Sandbox, WAIT, test_app, wait_until};
use sonata_core::app::NewTerminal;
use sonata_core::workspace::manager::CreateWorkspace;

#[test]
fn terminal_output_is_not_logged() {
    let sb = Sandbox::new();
    let guard = sonata_core::logging::init(&sb.root).unwrap();
    let (app, _) = test_app(&sb);
    let ws = app
        .create_workspace(CreateWorkspace {
            name: "W".into(),
            working_directory: sb.project("p"),
        })
        .unwrap()
        .id;
    app.open_workspace(ws).unwrap();
    let t = app
        .create_terminal(
            ws,
            NewTerminal {
                // The shell computes 42, so the typed command alone never matches.
                launch_command: Some(if cfg!(windows) {
                    "echo TOKEN-4^2-SECRET".into()
                } else {
                    "echo TOKEN-$((6*7))-SECRET".into()
                }),
                ..Default::default()
            },
        )
        .unwrap();
    assert!(wait_until(WAIT, || app
        .terminals()
        .screen_text(t.id)
        .unwrap()
        .contains("TOKEN-42-SECRET")));
    app.terminals().write(t.id, b"exit 4\r").unwrap();
    assert!(wait_until(WAIT, || app
        .terminals()
        .status(t.id)
        .is_some_and(|s| matches!(
            s,
            sonata_core::terminal::manager::RunStatus::Exited { .. }
        ))));
    app.save_dirty();
    app.quit();
    drop(guard);

    let logs: String = std::fs::read_dir(sb.root.logs_dir())
        .unwrap()
        .flatten()
        .map(|e| std::fs::read_to_string(e.path()).unwrap())
        .collect();
    assert!(
        logs.contains("workspace created"),
        "logging not active: {logs}"
    );
    assert!(!logs.contains("TOKEN-42-SECRET"));
    assert!(!logs.contains("6*7"), "launch command logged");
}

mod common;

use std::time::Duration;

use common::{Sandbox, WAIT, test_app, wait_until};
use sonata_core::app::{NewTerminal, TerminalSettings};
use sonata_core::events::{CoreEvent, RecordingSink, RunState};
use sonata_core::workspace::manager::CreateWorkspace;

fn busy_events(sink: &RecordingSink, id: uuid::Uuid, seen: &mut Vec<bool>) {
    for e in sink.take() {
        if let CoreEvent::TerminalState(s) = e
            && s.node_id == id
            && s.run == RunState::Running
        {
            seen.push(s.busy);
        }
    }
}

#[test]
fn monitored_terminals_report_busy_changes() {
    let sb = Sandbox::new();
    let (app, sink) = test_app(&sb);
    let ws = app
        .create_workspace(CreateWorkspace {
            name: "W".into(),
            working_directory: sb.project("p"),
        })
        .unwrap()
        .id;
    app.open_workspace(ws).unwrap();
    let watched = app.create_terminal(ws, NewTerminal::default()).unwrap().id;
    let quiet = app
        .create_terminal(
            ws,
            NewTerminal {
                settings: TerminalSettings {
                    monitor_activity: Some(false),
                    ..Default::default()
                },
                ..Default::default()
            },
        )
        .unwrap()
        .id;
    for id in [watched, quiet] {
        assert!(wait_until(WAIT, || app
            .terminals()
            .screen_text(id)
            .is_some_and(|s| s.contains('$'))));
    }
    std::thread::sleep(Duration::from_millis(700));
    sink.take();
    for id in [watched, quiet] {
        app.terminals().write(id, b"sleep 1.5\r").unwrap();
    }
    let mut seen = Vec::new();
    let mut quiet_seen = Vec::new();
    assert!(
        wait_until(Duration::from_secs(5), || {
            let events = sink.take();
            for e in &events {
                if let CoreEvent::TerminalState(s) = e {
                    if s.node_id == watched && s.run == RunState::Running {
                        seen.push(s.busy);
                    }
                    if s.node_id == quiet {
                        quiet_seen.push(s.busy);
                    }
                }
            }
            seen.contains(&true) && seen.last() == Some(&false)
        }),
        "{seen:?}"
    );
    assert!(
        quiet_seen.is_empty(),
        "unmonitored terminal emitted {quiet_seen:?}"
    );
    let _ = busy_events;
    app.quit();
}

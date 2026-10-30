// Drives a POSIX shell loop; Windows terminals are covered by terminal_pwsh.rs.
#![cfg(unix)]
mod common;

use std::time::{Duration, Instant};

use common::{Sandbox, WAIT, test_app, wait_until};
use sonata_core::app::NewTerminal;
use sonata_core::events::CoreEvent;
use sonata_core::workspace::manager::CreateWorkspace;

#[test]
fn only_subscribed_terminals_emit_previews_at_most_once_per_second() {
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
    let a = app.create_terminal(ws, NewTerminal::default()).unwrap().id;
    let b = app.create_terminal(ws, NewTerminal::default()).unwrap().id;
    for id in [a, b] {
        assert!(wait_until(WAIT, || app
            .terminals()
            .screen_text(id)
            .is_some_and(|s| s.contains(common::PROMPT))));
        app.terminals()
            .write(id, b"while :; do echo tick; sleep 0.05; done\r")
            .unwrap();
    }
    sink.take();
    app.subscribe_previews(vec![a]);

    let start = Instant::now();
    let mut events = Vec::new();
    while start.elapsed() < Duration::from_millis(3100) {
        events.extend(sink.take().into_iter().filter_map(|e| match e {
            CoreEvent::TerminalPreview(p) => Some((start.elapsed(), p)),
            _ => None,
        }));
        std::thread::sleep(Duration::from_millis(20));
    }
    assert!(!events.is_empty(), "no previews");
    assert!(
        events.iter().all(|(_, p)| p.node_id == a),
        "unsubscribed terminal emitted"
    );
    assert!(
        events.len() <= 4,
        "more than ~1 per second: {}",
        events.len()
    );
    let last = &events.last().unwrap().1;
    assert!(
        last.lines.iter().any(|l| l.trim_end() == "tick"),
        "{:?}",
        last.lines
    );
    assert_eq!(
        last.lines.len(),
        app.terminals().preview(a).unwrap().0.len()
    );

    app.subscribe_previews(vec![]);
    std::thread::sleep(Duration::from_millis(1200));
    sink.take();
    std::thread::sleep(Duration::from_millis(1500));
    assert!(
        !sink
            .take()
            .iter()
            .any(|e| matches!(e, CoreEvent::TerminalPreview(_)))
    );
    app.quit();
}

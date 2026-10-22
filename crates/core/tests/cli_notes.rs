mod common;

use std::time::Duration;

use common::{Sandbox, messaging_app, shells, sonata_as, sonata_with};
use sonata_core::events::CoreEvent;
use sonata_core::messaging::commands::short_id;
use sonata_core::model::node::Frame;

#[test]
fn linked_notes_and_notifications() {
    let sb = Sandbox::new();
    let (app, sink) = messaging_app(&sb);
    let (ws, ids) = shells(&app, &sb, &["A"]);
    let a = ids[0];
    let note = app
        .create_note(
            ws,
            Frame {
                x: 0.0,
                y: 600.0,
                width: 300.0,
                height: 200.0,
            },
        )
        .unwrap()
        .id;
    app.note_write(note, "# Plan\n- start", true).unwrap();

    assert_eq!(
        sonata_as(&app, a, &["note", "read", "Plan"]).code,
        3,
        "unlinked note readable"
    );
    app.create_link(ws, a, note).unwrap();
    let list = sonata_as(&app, a, &["list"]);
    assert!(
        list.stdout.contains("note")
            && list.stdout.contains("Plan")
            && list.stdout.contains(&short_id(note)),
        "{}",
        list.stdout
    );

    let read = sonata_as(&app, a, &["note", "read", "plan"]);
    assert_eq!(read.stdout.trim(), "# Plan\n- start");
    sink.take();
    let app_ = sonata_as(&app, a, &["note", "write", "Plan", "- done", "--append"]);
    assert_eq!(app_.code, 0, "{}", app_.stderr);
    assert_eq!(app.note_read(note).unwrap().text, "# Plan\n- start\n- done");
    assert!(
        sink.take()
            .iter()
            .any(|e| matches!(e, CoreEvent::NoteChanged(c) if c.node_id == note))
    );
    let by_id = sonata_with(
        &app,
        a,
        &["note", "write", &short_id(note), "--stdin"],
        Some("# Plan\nreplaced"),
    );
    assert_eq!(by_id.code, 0, "{}", by_id.stderr);
    assert_eq!(app.note_read(note).unwrap().text, "# Plan\nreplaced");

    // While the user edits (unsaved), the agent's text goes to a conflict copy.
    app.note_write(note, "# Plan\nuser typing", false).unwrap();
    sonata_as(&app, a, &["note", "write", "Plan", "agent text"]);
    assert_eq!(app.note_read(note).unwrap().text, "# Plan\nuser typing");
    assert!(
        sink.take()
            .iter()
            .any(|e| matches!(e, CoreEvent::NoteConflict(_)))
    );

    let n = sonata_as(&app, a, &["notify", "all", "done"]);
    assert_eq!(n.code, 0, "{}", n.stderr);
    std::thread::sleep(Duration::from_millis(100));
    assert!(
        sink.take().iter().any(
            |e| matches!(e, CoreEvent::Notify(x) if x.message == "all done" && x.title == "A")
        )
    );
    app.quit();
}

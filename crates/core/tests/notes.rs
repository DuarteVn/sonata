mod common;

use std::time::Duration;

use common::{Sandbox, test_app, wait_until};
use sonata_core::events::{CoreEvent, RecordingSink};
use sonata_core::model::node::Frame;
use sonata_core::notes::store::NoteSync;
use sonata_core::workspace::manager::CreateWorkspace;

const TWO_SECONDS: Duration = Duration::from_secs(2);

fn setup(
    sb: &Sandbox,
) -> (
    std::sync::Arc<sonata_core::App>,
    std::sync::Arc<RecordingSink>,
    uuid::Uuid,
) {
    let (app, sink) = test_app(sb);
    let ws = app
        .create_workspace(CreateWorkspace {
            name: "W".into(),
            working_directory: sb.project("p"),
        })
        .unwrap()
        .id;
    app.open_workspace(ws).unwrap();
    (app, sink, ws)
}

fn frame() -> Frame {
    Frame {
        x: 0.0,
        y: 0.0,
        width: 10.0,
        height: 10.0,
    }
}

fn saw(sink: &RecordingSink, seen: &mut Vec<CoreEvent>, f: impl Fn(&CoreEvent) -> bool) -> bool {
    seen.extend(sink.take());
    seen.iter().any(f)
}

#[test]
fn create_write_save_delete() {
    let sb = Sandbox::new();
    let (app, _, ws) = setup(&sb);
    let note = app.create_note(ws, frame()).unwrap();
    assert_eq!((note.frame.width, note.frame.height), (120.0, 80.0));
    let path = sb.root.notes_dir(ws).join(format!("{}.md", note.id));
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "");

    app.note_write(note.id, "# Plan\n- one", false).unwrap();
    assert_eq!(app.note_read(note.id).unwrap().sync, NoteSync::Dirty);
    assert_eq!(
        std::fs::read_to_string(&path).unwrap(),
        "",
        "written before save"
    );
    app.save_dirty();
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "# Plan\n- one");
    assert_eq!(app.note_read(note.id).unwrap().sync, NoteSync::Clean);

    app.note_write(note.id, "flushed", true).unwrap();
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "flushed");

    app.delete_node(ws, note.id, false).unwrap();
    assert!(!path.exists());
    app.quit();
}

#[test]
fn external_edit_reloads_when_clean() {
    let sb = Sandbox::new();
    let (app, sink, ws) = setup(&sb);
    let note = app.create_note(ws, frame()).unwrap();
    app.note_write(note.id, "mine", true).unwrap();
    std::thread::sleep(Duration::from_millis(500));
    sink.take();
    let path = sb.root.notes_dir(ws).join(format!("{}.md", note.id));
    std::fs::write(&path, "from an agent").unwrap();
    let mut seen = Vec::new();
    assert!(wait_until(TWO_SECONDS, || saw(
        &sink,
        &mut seen,
        |e| matches!(e,
        CoreEvent::NoteChanged(c) if c.node_id == note.id && c.text == "from an agent")
    )));
    assert_eq!(app.note_read(note.id).unwrap().text, "from an agent");
    app.quit();
}

#[test]
fn own_writes_do_not_trigger_reloads() {
    let sb = Sandbox::new();
    let (app, sink, ws) = setup(&sb);
    let note = app.create_note(ws, frame()).unwrap();
    sink.take();
    app.note_write(note.id, "typed in the app", true).unwrap();
    std::thread::sleep(Duration::from_millis(1200));
    assert!(
        !sink
            .take()
            .iter()
            .any(|e| matches!(e, CoreEvent::NoteChanged(_) | CoreEvent::NoteConflict(_)))
    );
    app.quit();
}

#[test]
fn external_edit_while_dirty_creates_conflict_copy() {
    let sb = Sandbox::new();
    let (app, sink, ws) = setup(&sb);
    let note = app.create_note(ws, frame()).unwrap();
    std::thread::sleep(Duration::from_millis(500));
    app.note_write(note.id, "unsaved in app", false).unwrap();
    let dir = sb.root.notes_dir(ws);
    std::fs::write(dir.join(format!("{}.md", note.id)), "external text").unwrap();
    let mut seen = Vec::new();
    assert!(wait_until(TWO_SECONDS, || saw(
        &sink,
        &mut seen,
        |e| matches!(e,
        CoreEvent::NoteConflict(c) if c.node_id == note.id)
    )));
    let conflict = dir.join(format!("{} (conflict).md", note.id));
    assert_eq!(std::fs::read_to_string(&conflict).unwrap(), "external text");
    let state = app.note_read(note.id).unwrap();
    assert_eq!(state.text, "unsaved in app");
    assert!(matches!(state.sync, NoteSync::Conflict { .. }));
    app.save_dirty();
    assert_eq!(
        std::fs::read_to_string(dir.join(format!("{}.md", note.id))).unwrap(),
        "unsaved in app"
    );
    app.quit();
}

#[test]
fn deleted_file_is_rewritten_on_next_save() {
    let sb = Sandbox::new();
    let (app, sink, ws) = setup(&sb);
    let note = app.create_note(ws, frame()).unwrap();
    app.note_write(note.id, "keep me", true).unwrap();
    std::thread::sleep(Duration::from_millis(500));
    let path = sb.root.notes_dir(ws).join(format!("{}.md", note.id));
    std::fs::remove_file(&path).unwrap();
    let mut seen = Vec::new();
    assert!(wait_until(TWO_SECONDS, || saw(&sink, &mut seen, |_| false)
        || app.note_read(note.id).unwrap().sync
            == NoteSync::FileMissing));
    assert_eq!(app.note_read(note.id).unwrap().text, "keep me");
    app.save_dirty();
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "keep me");
    app.quit();
}

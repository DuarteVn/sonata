mod common;

use common::{Sandbox, test_app};
use sonata_core::ErrorCode;
use sonata_core::app::NewTerminal;
use sonata_core::model::node::Frame;
use sonata_core::persistence::store::Store;
use sonata_core::workspace::manager::{CreateWorkspace, WorkspaceManager};

fn frame() -> Frame {
    Frame {
        x: 0.0,
        y: 0.0,
        width: 300.0,
        height: 200.0,
    }
}

#[test]
fn create_validate_delete_and_persist() {
    let sb = Sandbox::new();
    let (app, _) = test_app(&sb);
    let ws = app
        .create_workspace(CreateWorkspace {
            name: "W".into(),
            working_directory: sb.project("p"),
        })
        .unwrap()
        .id;
    app.open_workspace(ws).unwrap();
    let t1 = app.create_terminal(ws, NewTerminal::default()).unwrap().id;
    let t2 = app.create_terminal(ws, NewTerminal::default()).unwrap().id;
    let note = app.create_note(ws, frame()).unwrap().id;
    app.save_dirty();

    let l1 = app.create_link(ws, t1, t2).unwrap();
    assert_eq!(app.workspaces().dirty_ids(), vec![ws]);
    assert!(l1.a < l1.b);
    app.create_link(ws, note, t1).unwrap();
    assert_eq!(
        app.create_link(ws, t2, t1).unwrap_err().code,
        ErrorCode::LinkExists
    );
    assert_eq!(
        app.create_link(ws, t1, t1).unwrap_err().code,
        ErrorCode::LinkSelf
    );
    assert_eq!(
        app.create_link(ws, t1, uuid::Uuid::new_v4())
            .unwrap_err()
            .code,
        ErrorCode::NodeNotFound
    );
    assert_eq!(app.links_of(ws, t1).len(), 2);

    app.delete_link(ws, l1.id).unwrap();
    assert_eq!(app.links_of(ws, t1).len(), 1);
    app.save_dirty();
    let reloaded = WorkspaceManager::load(Store::new(sb.root.clone())).unwrap();
    assert_eq!(reloaded.workspace(ws).unwrap().links.len(), 1);

    app.delete_node(ws, t1, true).unwrap();
    assert!(
        app.workspaces().workspace(ws).unwrap().links.is_empty(),
        "node deletion kept its links"
    );
    app.quit();
}

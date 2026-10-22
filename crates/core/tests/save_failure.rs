mod common;

use common::{Sandbox, test_app};
use sonata_core::events::CoreEvent;
use sonata_core::workspace::manager::CreateWorkspace;

#[test]
fn failed_save_warns_keeps_data_and_retries() {
    let sb = Sandbox::new();
    let (app, sink) = test_app(&sb);
    let ws = app
        .create_workspace(CreateWorkspace {
            name: "W".into(),
            working_directory: sb.project("p"),
        })
        .unwrap()
        .id;
    let dir = sb.root.workspace_dir(ws);
    app.set_camera(ws, 7.0, 8.0, 2.0).unwrap();

    // A non-empty folder where the file should be makes the atomic rename fail.
    let file = dir.join("workspace.json");
    std::fs::remove_file(&file).unwrap();
    std::fs::create_dir_all(file.join("blocker")).unwrap();
    let report = app.save_dirty();
    assert_eq!(report.errors.len(), 1);
    assert!(
        sink.take()
            .iter()
            .any(|e| matches!(e, CoreEvent::PersistenceWarning(_)))
    );
    assert_eq!(
        app.workspaces().dirty_ids(),
        vec![ws],
        "change dropped after failed save"
    );

    std::fs::remove_dir_all(&file).unwrap();
    let report = app.save_dirty();
    assert!(report.errors.is_empty());
    assert_eq!(report.saved, vec![ws]);
}

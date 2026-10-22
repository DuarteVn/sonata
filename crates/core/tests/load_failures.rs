mod common;

use common::{Sandbox, test_app};
use serde_json::json;
use sonata_core::persistence::store::LoadFailureReason;
use sonata_core::workspace::manager::CreateWorkspace;
use uuid::Uuid;

fn write_raw(sb: &Sandbox, folder: &str, content: &str) -> std::path::PathBuf {
    let dir = sb.root.workspaces_dir().join(folder);
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("workspace.json");
    std::fs::write(&file, content).unwrap();
    file
}

fn valid_payload(id: Uuid, dir: &std::path::Path) -> serde_json::Value {
    json!({
        "id": id, "name": "X", "icon": "code", "color": "#3B82F6",
        "workingDirectory": dir, "createdAt": "2026-10-21T00:00:00Z",
        "lastOpenedAt": "2026-10-21T00:00:00Z"
    })
}

#[test]
fn each_failure_reason_is_reported_and_files_are_never_rewritten() {
    let sb = Sandbox::new();
    let dir = sb.project("p");
    let ids: Vec<Uuid> = (0..6).map(|_| Uuid::new_v4()).collect();
    let cases = [
        write_raw(&sb, &ids[0].to_string(), "{"),
        write_raw(&sb, &ids[1].to_string(), &json!({"type": "note", "schemaVersion": 1, "payload": {}}).to_string()),
        write_raw(&sb, &ids[2].to_string(), &json!({"type": "workspace", "schemaVersion": 99, "payload": {}}).to_string()),
        write_raw(&sb, &ids[3].to_string(), &json!({"type": "workspace", "schemaVersion": 0, "payload": {}}).to_string()),
        write_raw(&sb, &ids[4].to_string(), &json!({"type": "workspace", "schemaVersion": 1, "payload": {"id": ids[4]}}).to_string()),
        write_raw(
            &sb,
            &ids[5].to_string(),
            &json!({"type": "workspace", "schemaVersion": 1, "payload": valid_payload(Uuid::new_v4(), &dir)}).to_string(),
        ),
    ];
    write_raw(&sb, "not-a-uuid", "{");
    let before: Vec<Vec<u8>> = cases.iter().map(|p| std::fs::read(p).unwrap()).collect();

    let (app, _) = test_app(&sb);
    app.create_workspace(CreateWorkspace {
        name: "Good".into(),
        working_directory: dir,
    })
    .unwrap();
    let boot = app.bootstrap();
    assert_eq!(boot.workspaces.len(), 1);
    assert_eq!(boot.load_failures.len(), 6);
    let reason = |i: usize| {
        boot.load_failures
            .iter()
            .find(|f| f.folder_name == ids[i].to_string())
            .unwrap()
            .reason
            .clone()
    };
    assert!(matches!(reason(0), LoadFailureReason::InvalidJson { .. }));
    assert!(matches!(reason(1), LoadFailureReason::WrongType { .. }));
    assert_eq!(
        reason(2),
        LoadFailureReason::NewerVersion {
            found: 99,
            supported: 1
        }
    );
    assert!(matches!(
        reason(3),
        LoadFailureReason::MigrationFailed { step: 0, .. }
    ));
    assert!(matches!(reason(4), LoadFailureReason::DecodeFailed { .. }));
    assert!(matches!(reason(5), LoadFailureReason::IdMismatch { .. }));
    assert_eq!(
        boot.load_failures[0].path.file_name().unwrap(),
        "workspace.json"
    );

    app.save_dirty();
    app.quit();
    let after: Vec<Vec<u8>> = cases.iter().map(|p| std::fs::read(p).unwrap()).collect();
    assert_eq!(before, after, "a failed workspace file was rewritten");
}

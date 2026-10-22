mod common;

use common::{Sandbox, test_app};
use serde_json::{Value, json};
use sonata_core::persistence::store::LoadFailureReason;
use uuid::Uuid;

fn write(sb: &Sandbox, id: Uuid, value: &Value) -> std::path::PathBuf {
    let dir = sb.root.workspace_dir(id);
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("workspace.json");
    std::fs::write(&file, serde_json::to_vec_pretty(value).unwrap()).unwrap();
    file
}

fn payload(id: Uuid, dir: &std::path::Path, nodes: Value) -> Value {
    json!({
        "id": id, "name": "Old", "icon": "code", "color": "#3B82F6", "workingDirectory": dir,
        "createdAt": "2026-10-21T00:00:00Z", "lastOpenedAt": "2026-10-21T00:00:00Z", "nodes": nodes
    })
}

#[test]
fn v1_file_loads_and_saves_as_v2() {
    let sb = Sandbox::new();
    let id = Uuid::new_v4();
    let file = write(
        &sb,
        id,
        &json!({"type": "workspace", "schemaVersion": 1, "payload": payload(id, &sb.project("p"), json!([]))}),
    );
    let (app, _) = test_app(&sb);
    assert!(app.bootstrap().load_failures.is_empty());
    assert!(app.workspaces().workspace(id).unwrap().links.is_empty());
    app.set_camera(id, 1.0, 1.0, 1.0).unwrap();
    app.save_dirty();
    let saved: Value = serde_json::from_slice(&std::fs::read(&file).unwrap()).unwrap();
    assert_eq!(saved["schemaVersion"], 2);
    assert_eq!(saved["payload"]["links"], json!([]));
    app.quit();
}

#[test]
fn dangling_links_are_dropped_with_warning() {
    let sb = Sandbox::new();
    let id = Uuid::new_v4();
    let node = Uuid::new_v4();
    let nodes = json!([{
        "id": node, "frame": {"x": 0, "y": 0, "width": 300, "height": 200}, "z": 1,
        "createdAt": "2026-10-21T00:00:00Z", "modifiedAt": "2026-10-21T00:00:00Z", "content": {"note": {}}
    }]);
    let mut p = payload(id, &sb.project("p"), nodes);
    p["links"] = json!([{"id": Uuid::new_v4(), "a": node, "b": Uuid::new_v4(), "createdAt": "2026-10-21T00:00:00Z"}]);
    write(
        &sb,
        id,
        &json!({"type": "workspace", "schemaVersion": 2, "payload": p}),
    );
    let (app, _) = test_app(&sb);
    let boot = app.bootstrap();
    assert!(app.workspaces().workspace(id).unwrap().links.is_empty());
    assert!(
        boot.warnings.iter().any(|w| w.contains("link")),
        "{:?}",
        boot.warnings
    );
    app.quit();
}

#[test]
fn v3_file_is_refused_and_untouched() {
    let sb = Sandbox::new();
    let id = Uuid::new_v4();
    let file = write(
        &sb,
        id,
        &json!({"type": "workspace", "schemaVersion": 3, "payload": payload(id, &sb.project("p"), json!([]))}),
    );
    let before = std::fs::read(&file).unwrap();
    let (app, _) = test_app(&sb);
    let boot = app.bootstrap();
    assert_eq!(
        boot.load_failures[0].reason,
        LoadFailureReason::NewerVersion {
            found: 3,
            supported: 2
        }
    );
    app.quit();
    assert_eq!(std::fs::read(&file).unwrap(), before);
}

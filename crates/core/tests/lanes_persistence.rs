mod common;

use common::{Sandbox, test_app};
use serde_json::{Value, json};
use sonata_core::model::node::{Lane, TerminalContent};
use uuid::Uuid;

fn terminal_node(lane: Value) -> Value {
    json!({
        "id": Uuid::new_v4(), "frame": {"x": 0, "y": 0, "width": 640, "height": 400}, "z": 1,
        "createdAt": "2026-10-23T00:00:00Z", "modifiedAt": "2026-10-23T00:00:00Z",
        "content": {"terminal": {"name": "Api", "saveHistory": true, "monitorActivity": true,
                                 "orchestrator": false, "lane": lane}}
    })
}

fn write_workspace(sb: &Sandbox, version: u32, nodes: Value) -> (Uuid, std::path::PathBuf) {
    let id = Uuid::new_v4();
    let dir = sb.root.workspace_dir(id);
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("workspace.json");
    let payload = json!({
        "id": id, "name": "W", "icon": "code", "color": "#3B82F6",
        "workingDirectory": sb.project("p"), "createdAt": "2026-10-23T00:00:00Z",
        "lastOpenedAt": "2026-10-23T00:00:00Z", "nodes": nodes, "links": []
    });
    let env = json!({"type": "workspace", "schemaVersion": version, "payload": payload});
    std::fs::write(&file, serde_json::to_vec_pretty(&env).unwrap()).unwrap();
    (id, file)
}

#[test]
fn lane_round_trips_with_sorted_keys_and_omitted_empties() {
    let sb = Sandbox::new();
    let lane = json!({
        "branch": "lane/api", "baseBranch": "main", "mainCheckout": "/repo",
        "checkoutPath": "/data/lanes/x/api", "createdAt": "2026-10-23T00:00:00Z"
    });
    let (id, file) = write_workspace(&sb, 3, json!([terminal_node(lane.clone())]));
    let (app, _) = test_app(&sb);
    assert!(app.bootstrap().load_failures.is_empty());
    let ws = app.workspaces().workspace(id).cloned().unwrap();
    let t = ws.nodes[0].terminal().unwrap();
    let got: &Lane = t.lane.as_ref().expect("lane loaded");
    assert_eq!(got.branch, "lane/api");
    assert_eq!(got.subdirectory, None);
    assert_eq!(got.landed_at, None);
    app.set_camera(id, 2.0, 2.0, 1.0).unwrap();
    app.save_dirty();
    app.quit();
    let text = std::fs::read_to_string(&file).unwrap();
    let saved: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(saved["schemaVersion"], 3);
    assert_eq!(
        saved["payload"]["nodes"][0]["content"]["terminal"]["lane"],
        lane
    );
    let lane_text = &text[text.find("\"lane\"").unwrap()..];
    let order: Vec<usize> = ["baseBranch", "branch\"", "checkoutPath", "mainCheckout"]
        .iter()
        .map(|k| lane_text.find(k).unwrap())
        .collect();
    assert!(
        order.windows(2).all(|w| w[0] < w[1]),
        "lane keys sorted: {order:?}"
    );
}

#[test]
fn v2_terminal_without_lane_loads_and_saves_as_v3() {
    let sb = Sandbox::new();
    let mut node = terminal_node(Value::Null);
    node["content"]["terminal"]
        .as_object_mut()
        .unwrap()
        .remove("lane");
    let (id, file) = write_workspace(&sb, 2, json!([node]));
    let (app, _) = test_app(&sb);
    assert!(app.bootstrap().load_failures.is_empty());
    let ws = app.workspaces().workspace(id).cloned().unwrap();
    assert!(ws.nodes[0].terminal().unwrap().lane.is_none());
    app.set_camera(id, 2.0, 2.0, 1.0).unwrap();
    app.save_dirty();
    app.quit();
    let saved: Value = serde_json::from_slice(&std::fs::read(&file).unwrap()).unwrap();
    assert_eq!(saved["schemaVersion"], 3);
    assert!(
        saved["payload"]["nodes"][0]["content"]["terminal"]
            .get("lane")
            .is_none()
    );
}

#[test]
fn new_terminal_has_no_lane() {
    assert!(TerminalContent::new("x".into()).lane.is_none());
}

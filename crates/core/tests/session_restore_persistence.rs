//! The terminal's remembered conversation in the workspace file (feature 009, research R6).

mod common;

use common::{Sandbox, test_app};
use serde_json::{Value, json};
use sonata_core::model::node::ConversationAgent;
use uuid::Uuid;

fn terminal_node(conversation: Option<Value>) -> Value {
    let mut terminal = json!({"name": "Api", "saveHistory": true, "monitorActivity": true,
                              "orchestrator": false});
    if let Some(c) = conversation {
        terminal["conversation"] = c;
    }
    json!({
        "id": Uuid::new_v4(), "frame": {"x": 0, "y": 0, "width": 640, "height": 400}, "z": 1,
        "createdAt": "2026-10-23T00:00:00Z", "modifiedAt": "2026-10-23T00:00:00Z",
        "content": {"terminal": terminal}
    })
}

fn write_workspace(sb: &Sandbox, nodes: Value) -> (Uuid, std::path::PathBuf) {
    let id = Uuid::new_v4();
    let dir = sb.root.workspace_dir(id);
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("workspace.json");
    let payload = json!({
        "id": id, "name": "W", "icon": "code", "color": "#3B82F6",
        "workingDirectory": sb.project("p"), "createdAt": "2026-10-23T00:00:00Z",
        "lastOpenedAt": "2026-10-23T00:00:00Z", "nodes": nodes, "links": []
    });
    let env = json!({"type": "workspace", "schemaVersion": 3, "payload": payload});
    std::fs::write(&file, serde_json::to_vec_pretty(&env).unwrap()).unwrap();
    (id, file)
}

#[test]
fn conversation_round_trips_with_sorted_keys() {
    let sb = Sandbox::new();
    let conversation = json!({
        "agent": "claude", "id": "7294d08d-ac3c-4e7e-925e-16750c04c269",
        "recordedAt": "2026-10-24T13:05:00Z"
    });
    let (id, file) = write_workspace(&sb, json!([terminal_node(Some(conversation.clone()))]));
    let (app, _) = test_app(&sb);
    assert!(app.bootstrap().load_failures.is_empty());
    let ws = app.workspaces().workspace(id).cloned().unwrap();
    let c = ws.nodes[0]
        .terminal()
        .unwrap()
        .conversation
        .clone()
        .unwrap();
    assert_eq!(c.agent, ConversationAgent::Claude);
    assert_eq!(c.id, "7294d08d-ac3c-4e7e-925e-16750c04c269");
    app.set_camera(id, 2.0, 2.0, 1.0).unwrap();
    app.save_dirty();
    app.quit();
    let text = std::fs::read_to_string(&file).unwrap();
    let saved: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(saved["schemaVersion"], 3, "no schema bump");
    assert_eq!(
        saved["payload"]["nodes"][0]["content"]["terminal"]["conversation"],
        conversation
    );
    let c_text = &text[text.find("\"conversation\"").unwrap()..];
    let order: Vec<usize> = ["\"agent\"", "\"id\"", "\"recordedAt\""]
        .iter()
        .map(|k| c_text.find(k).unwrap())
        .collect();
    assert!(
        order.windows(2).all(|w| w[0] < w[1]),
        "keys sorted: {order:?}"
    );
}

#[test]
fn file_from_before_the_feature_loads_without_conversation() {
    let sb = Sandbox::new();
    let (id, file) = write_workspace(&sb, json!([terminal_node(None)]));
    let (app, _) = test_app(&sb);
    assert!(app.bootstrap().load_failures.is_empty());
    let ws = app.workspaces().workspace(id).cloned().unwrap();
    assert!(ws.nodes[0].terminal().unwrap().conversation.is_none());
    app.set_camera(id, 2.0, 2.0, 1.0).unwrap();
    app.save_dirty();
    app.quit();
    let saved: Value = serde_json::from_slice(&std::fs::read(&file).unwrap()).unwrap();
    assert!(
        saved["payload"]["nodes"][0]["content"]["terminal"]
            .get("conversation")
            .is_none()
    );
}

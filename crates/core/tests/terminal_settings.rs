mod common;

use common::{Sandbox, WAIT, test_app, wait_until};
use serde_json::json;
use sonata_core::ErrorCode;
use sonata_core::app::{NewTerminal, TerminalSettings};
use sonata_core::model::node::{Node, NoteColor, RoleRef};
use sonata_core::workspace::manager::CreateWorkspace;

#[test]
fn defaults_for_001_nodes() {
    let terminal: Node = serde_json::from_value(json!({
        "id": uuid::Uuid::new_v4(), "frame": {"x": 0, "y": 0, "width": 300, "height": 200}, "z": 1,
        "createdAt": "2026-10-21T00:00:00Z", "modifiedAt": "2026-10-21T00:00:00Z",
        "content": {"terminal": {"name": "T", "saveHistory": true}}
    }))
    .unwrap();
    let t = terminal.terminal().unwrap();
    assert!(t.monitor_activity);
    assert!(!t.orchestrator);
    assert!(t.accent_color.is_none() && t.font_size.is_none() && t.role.is_none());
    let out = serde_json::to_value(&terminal).unwrap();
    let keys: Vec<&String> = out["content"]["terminal"]
        .as_object()
        .unwrap()
        .keys()
        .collect();
    assert!(
        !keys
            .iter()
            .any(|k| *k == "accentColor" || *k == "fontSize" || *k == "role"),
        "{keys:?}"
    );

    let note: Node = serde_json::from_value(json!({
        "id": uuid::Uuid::new_v4(), "frame": {"x": 0, "y": 0, "width": 300, "height": 200}, "z": 1,
        "createdAt": "2026-10-21T00:00:00Z", "modifiedAt": "2026-10-21T00:00:00Z", "content": {"note": {}}
    }))
    .unwrap();
    let style = note.note_style().unwrap();
    assert_eq!(style.color, NoteColor::Yellow);
    assert_eq!(style.text_size, 3);
}

fn setup(sb: &Sandbox) -> (std::sync::Arc<sonata_core::App>, uuid::Uuid) {
    let (app, _) = test_app(sb);
    let ws = app
        .create_workspace(CreateWorkspace {
            name: "W".into(),
            working_directory: sb.project("p"),
        })
        .unwrap()
        .id;
    app.open_workspace(ws).unwrap();
    (app, ws)
}

#[test]
fn update_settings_applies_and_validates() {
    let sb = Sandbox::new();
    let (app, ws) = setup(&sb);
    let node = app.create_terminal(ws, NewTerminal::default()).unwrap().id;
    assert!(wait_until(WAIT, || app
        .terminals()
        .screen_text(node)
        .is_some_and(|s| s.contains('$'))));

    let res = app
        .update_terminal_settings(
            node,
            TerminalSettings {
                name: Some("Lead".into()),
                accent_color: Some(Some("#8B5CF6".into())),
                font_size: Some(Some(16)),
                role: Some(Some(RoleRef {
                    name: Some("Coordinator".into()),
                    text: "Coordinate.".into(),
                })),
                orchestrator: Some(true),
                monitor_activity: Some(false),
                ..Default::default()
            },
        )
        .unwrap();
    assert!(!res.applies_on_restart);
    let t = res.node.terminal().unwrap().clone();
    assert_eq!(t.name, "Lead");
    assert_eq!(t.accent_color.as_deref(), Some("#8B5CF6"));
    assert_eq!(t.font_size, Some(16));
    assert!(t.orchestrator && !t.monitor_activity);
    assert_eq!(app.workspaces().dirty_ids(), vec![ws]);

    let res = app
        .update_terminal_settings(
            node,
            TerminalSettings {
                launch_command: Some(Some("echo restarted-with-new-command".into())),
                ..Default::default()
            },
        )
        .unwrap();
    assert!(res.applies_on_restart);
    app.restart_terminal(node, true).unwrap();
    assert!(wait_until(WAIT, || app
        .terminals()
        .screen_text(node)
        .is_some_and(|s| s.contains("\nrestarted-with-new-command"))));

    let bad_font = TerminalSettings {
        font_size: Some(Some(40)),
        ..Default::default()
    };
    assert!(app.update_terminal_settings(node, bad_font).is_err());
    let bad_color = TerminalSettings {
        accent_color: Some(Some("#123456".into())),
        ..Default::default()
    };
    assert!(app.update_terminal_settings(node, bad_color).is_err());
    let busy = app.terminals().write(node, b"sleep 10\r");
    assert!(busy.is_ok());
    assert!(wait_until(WAIT, || app.terminals().is_busy(node)));
    assert_eq!(
        app.restart_terminal(node, false).unwrap_err().code,
        ErrorCode::TerminalBusy
    );
    app.quit();
}

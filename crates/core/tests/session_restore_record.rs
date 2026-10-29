//! Recording, clearing, and forgetting a terminal's conversation (feature 009).

mod common;

use std::time::Duration;

use common::{Sandbox, messaging_app, shells, sonata_as, test_app};
use serde_json::Value;
use sonata_core::App;
use sonata_core::events::{CoreEvent, RecordingSink};
use sonata_core::model::node::{Conversation, ConversationAgent, Frame};
use uuid::Uuid;

const ID: &str = "7294d08d-ac3c-4e7e-925e-16750c04c269";
const CLAUDE: ConversationAgent = ConversationAgent::Claude;
const CODEX: ConversationAgent = ConversationAgent::Codex;

fn conversation(app: &App, ws: Uuid, node: Uuid) -> Option<Conversation> {
    app.workspaces()
        .workspace(ws)
        .and_then(|w| w.node(node).cloned())
        .and_then(|n| n.terminal().and_then(|t| t.conversation.clone()))
}

fn on_disk(sb: &Sandbox, ws: Uuid, node: Uuid) -> Value {
    let saved: Value =
        serde_json::from_slice(&std::fs::read(sb.root.workspace_file(ws)).unwrap()).unwrap();
    saved["payload"]["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|n| n["id"] == node.to_string())
        .map(|n| n["content"]["terminal"]["conversation"].clone())
        .unwrap()
}

fn mtime(sb: &Sandbox, ws: Uuid) -> std::time::SystemTime {
    std::fs::metadata(sb.root.workspace_file(ws))
        .unwrap()
        .modified()
        .unwrap()
}

fn nodes_changed(sink: &RecordingSink) -> usize {
    sink.take()
        .iter()
        .filter(|e| matches!(e, CoreEvent::NodesChanged(_)))
        .count()
}

#[test]
fn record_stores_saves_at_once_and_ignores_repeats() {
    let sb = Sandbox::new();
    let (app, sink) = test_app(&sb);
    let (ws, ids) = shells(&app, &sb, &["A"]);
    let a = ids[0];
    sink.take();

    assert!(app.record_conversation(a, CLAUDE, Some(ID)).unwrap());
    let c = conversation(&app, ws, a).expect("stored");
    assert_eq!((c.agent, c.id.as_str()), (CLAUDE, ID));
    let disk = on_disk(&sb, ws, a);
    assert_eq!(disk["agent"], "claude");
    assert_eq!(disk["id"], ID);
    assert_eq!(nodes_changed(&sink), 1);

    let before = mtime(&sb, ws);
    std::thread::sleep(Duration::from_millis(20));
    assert!(!app.record_conversation(a, CLAUDE, Some(ID)).unwrap());
    assert_eq!(
        mtime(&sb, ws),
        before,
        "same value must not rewrite the file"
    );
    assert_eq!(nodes_changed(&sink), 0);

    // "No conversation" from another agent leaves it.
    assert!(!app.record_conversation(a, CODEX, None).unwrap());
    assert!(conversation(&app, ws, a).is_some());
    assert_eq!(nodes_changed(&sink), 0);

    // From the same agent it removes it, on disk too.
    assert!(app.record_conversation(a, CLAUDE, None).unwrap());
    assert!(conversation(&app, ws, a).is_none());
    assert!(on_disk(&sb, ws, a).is_null());
    assert_eq!(nodes_changed(&sink), 1);
    app.quit();
}

#[test]
fn another_agent_replaces_the_conversation() {
    let sb = Sandbox::new();
    let (app, _) = test_app(&sb);
    let (ws, ids) = shells(&app, &sb, &["A"]);
    app.record_conversation(ids[0], CLAUDE, Some(ID)).unwrap();
    assert!(app.record_conversation(ids[0], CODEX, Some("c3")).unwrap());
    let c = conversation(&app, ws, ids[0]).unwrap();
    assert_eq!((c.agent, c.id.as_str()), (CODEX, "c3"));
    app.quit();
}

#[test]
fn invalid_ids_and_non_terminals_are_refused() {
    let sb = Sandbox::new();
    let (app, sink) = test_app(&sb);
    let (ws, ids) = shells(&app, &sb, &["A"]);
    sink.take();
    for bad in ["", "a b", "a;b", "../x", "$(x)", "a\nb"] {
        assert!(app.record_conversation(ids[0], CLAUDE, Some(bad)).is_err());
    }
    let long = "x".repeat(129);
    assert!(
        app.record_conversation(ids[0], CLAUDE, Some(&long))
            .is_err()
    );
    assert!(conversation(&app, ws, ids[0]).is_none());
    assert_eq!(nodes_changed(&sink), 0);

    let frame = Frame {
        x: 0.0,
        y: 900.0,
        width: 200.0,
        height: 200.0,
    };
    let note = app.create_note(ws, frame).unwrap().id;
    assert!(app.record_conversation(note, CLAUDE, Some(ID)).is_err());
    assert!(
        app.record_conversation(Uuid::new_v4(), CLAUDE, Some(ID))
            .is_err()
    );
    app.quit();
}

#[test]
fn conversation_command_records_for_the_calling_terminal_only() {
    let sb = Sandbox::new();
    let (app, _) = messaging_app(&sb);
    let (ws, ids) = shells(&app, &sb, &["A", "B"]);
    let (a, b) = (ids[0], ids[1]);

    let out = sonata_as(&app, a, &["_conversation", "claude", ID]);
    assert_eq!(out.code, 0, "{}", out.stderr);
    assert_eq!(conversation(&app, ws, a).unwrap().id, ID);
    assert!(conversation(&app, ws, b).is_none());

    let out = sonata_as(&app, b, &["_conversation", "codex", "c3"]);
    assert_eq!(out.code, 0, "{}", out.stderr);
    let out = sonata_as(&app, b, &["_conversation", "codex", "none"]);
    assert_eq!(out.code, 0, "{}", out.stderr);
    assert!(conversation(&app, ws, b).is_none());
    assert_eq!(conversation(&app, ws, a).unwrap().id, ID);

    for args in [
        ["_conversation", "gemini", "x"],
        ["_conversation", "claude", "a;b"],
        ["_conversation", "claude", ""],
    ] {
        let out = sonata_as(&app, b, &args);
        assert_eq!(out.code, 2, "{args:?}: {}", out.stderr);
    }
    assert!(conversation(&app, ws, b).is_none());
    app.quit();
}

#[test]
fn conversation_command_refuses_callers_that_are_not_terminals() {
    let sb = Sandbox::new();
    let (app, _) = messaging_app(&sb);
    let (ws, ids) = shells(&app, &sb, &["A"]);
    let frame = Frame {
        x: 0.0,
        y: 900.0,
        width: 200.0,
        height: 200.0,
    };
    let note = app.create_note(ws, frame).unwrap().id;
    let out = std::process::Command::new(common::cli_bin())
        .args(["_conversation", "claude", ID])
        .env("SONATA_SOCKET", app.socket_path().unwrap())
        .env("SONATA_TERMINAL_ID", note.to_string())
        .env(
            "SONATA_TOKEN",
            app.terminals().session_token(ids[0]).unwrap(),
        )
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(3));
    assert!(conversation(&app, ws, ids[0]).is_none());
    app.quit();
}

#[test]
fn start_fresh_forgets_the_conversation_and_restarts() {
    use common::{WAIT, wait_until};
    use sonata_core::error::ErrorCode;
    let sb = Sandbox::new();
    let (app, _) = messaging_app(&sb);
    let (ws, ids) = shells(&app, &sb, &["A"]);
    let a = ids[0];
    // Stands for the agent's transcript: Start fresh must not touch it.
    let transcript = sb.tmp.path().join(format!("{ID}.jsonl"));
    std::fs::write(&transcript, "{}\n").unwrap();
    app.record_conversation(a, CLAUDE, Some(ID)).unwrap();

    app.terminals().write(a, b"sleep 10\r").unwrap();
    assert!(wait_until(WAIT, || app.terminals().is_busy(a)));
    let err = app.start_fresh_conversation(a, false).unwrap_err();
    assert_eq!(err.code, ErrorCode::TerminalBusy);
    assert_eq!(
        err.message,
        app.restart_terminal(a, false).unwrap_err().message
    );
    assert!(conversation(&app, ws, a).is_some());

    let token = app.terminals().session_token(a);
    app.start_fresh_conversation(a, true).unwrap();
    assert!(conversation(&app, ws, a).is_none());
    assert!(on_disk(&sb, ws, a).is_null());
    assert_ne!(
        app.terminals().session_token(a),
        token,
        "terminal restarted"
    );
    assert!(wait_until(WAIT, || !app.terminals().is_busy(a)));
    let hook = common::cli_bin().display().to_string();
    let out = sonata_as(&app, a, &["_session", "claude", &hook, "--json"]);
    let v: Value = serde_json::from_str(&out.stdout).unwrap();
    assert!(v["data"].get("resume").is_none(), "{v}");
    assert!(transcript.exists());

    // An idle terminal needs no confirmation. The restarted shell may still run startup
    // commands in the foreground, so retry until it is idle; a refusal changes nothing.
    app.record_conversation(a, CLAUDE, Some(ID)).unwrap();
    assert!(wait_until(WAIT, || app
        .start_fresh_conversation(a, false)
        .is_ok()));
    assert!(conversation(&app, ws, a).is_none());

    assert_eq!(
        app.start_fresh_conversation(Uuid::new_v4(), true)
            .unwrap_err()
            .code,
        ErrorCode::NodeNotFound
    );
    app.quit();
}

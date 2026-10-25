//! Agent hooks report the terminal's conversation (feature 009, contracts/cli.md hooks table).

mod common;

use std::io::Write;
use std::process::{Command, Stdio};

use common::{Sandbox, messaging_app, shells};
use sonata_core::App;
use sonata_core::model::node::{Conversation, ConversationAgent};
use uuid::Uuid;

/// Runs `sonata _hook <args>` as terminal `node`'s agent would, and returns its exit code.
fn hook(app: &App, node: Uuid, args: &[&str], stdin: &str, off: bool) -> i32 {
    let mut cmd = Command::new(common::cli_bin());
    cmd.arg("_hook")
        .args(args)
        .env("SONATA_SOCKET", app.socket_path().unwrap())
        .env("SONATA_TERMINAL_ID", node.to_string())
        .env("SONATA_TOKEN", app.terminals().session_token(node).unwrap())
        .env_remove("SONATA_CONVERSATION")
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    if off {
        cmd.env("SONATA_CONVERSATION", "off");
    }
    let mut child = cmd.spawn().unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(stdin.as_bytes())
        .unwrap();
    child.wait().unwrap().code().unwrap_or(-1)
}

fn stored(app: &App, ws: Uuid, node: Uuid) -> Option<(ConversationAgent, String)> {
    app.workspaces()
        .workspace(ws)
        .and_then(|w| w.node(node).cloned())
        .and_then(|n| n.terminal().and_then(|t| t.conversation.clone()))
        .map(|Conversation { agent, id, .. }| (agent, id))
}

const CLAUDE: ConversationAgent = ConversationAgent::Claude;
const CODEX: ConversationAgent = ConversationAgent::Codex;

#[test]
fn claude_session_start_records_resumes_and_forgets_new_sessions() {
    let sb = Sandbox::new();
    let (app, _) = messaging_app(&sb);
    let (ws, ids) = shells(&app, &sb, &["A"]);
    let a = ids[0];
    let start = |source: &str| {
        let payload = format!(r#"{{"session_id":"a1","source":"{source}"}}"#);
        assert_eq!(hook(&app, a, &["session-start"], &payload, false), 0);
    };

    start("resume");
    assert_eq!(stored(&app, ws, a), Some((CLAUDE, "a1".into())));
    start("compact");
    assert_eq!(stored(&app, ws, a), Some((CLAUDE, "a1".into())));
    start("startup");
    assert_eq!(stored(&app, ws, a), None);
    start("resume");
    start("clear");
    assert_eq!(stored(&app, ws, a), None);
    app.quit();
}

#[test]
fn turn_hooks_record_the_conversation() {
    let sb = Sandbox::new();
    let (app, _) = messaging_app(&sb);
    let (ws, ids) = shells(&app, &sb, &["A"]);
    let a = ids[0];

    assert_eq!(
        hook(&app, a, &["turn-start"], r#"{"session_id":"b2"}"#, false),
        0
    );
    assert_eq!(stored(&app, ws, a), Some((CLAUDE, "b2".into())));

    let notify = r#"{"type":"agent-turn-complete","thread-id":"c3","last-assistant-message":"x"}"#;
    assert_eq!(hook(&app, a, &["codex-notify", notify], "", false), 0);
    assert_eq!(stored(&app, ws, a), Some((CODEX, "c3".into())));
    app.quit();
}

#[test]
fn print_mode_and_bad_payloads_change_nothing() {
    let sb = Sandbox::new();
    let (app, _) = messaging_app(&sb);
    let (ws, ids) = shells(&app, &sb, &["A"]);
    let a = ids[0];
    app.record_conversation(a, CLAUDE, Some("keep")).unwrap();
    let keep = Some((CLAUDE, "keep".to_string()));

    let notify = r#"{"type":"agent-turn-complete","thread-id":"c3"}"#;
    let cases: [(&[&str], &str); 3] = [
        (
            &["session-start"],
            r#"{"session_id":"x","source":"startup"}"#,
        ),
        (&["turn-start"], r#"{"session_id":"y"}"#),
        (&["codex-notify", notify], ""),
    ];
    for (args, stdin) in cases {
        assert_eq!(hook(&app, a, args, stdin, true), 0, "{args:?}");
        assert_eq!(
            stored(&app, ws, a),
            keep,
            "{args:?} with SONATA_CONVERSATION=off"
        );
    }

    let bad: [(&[&str], &str); 6] = [
        (&["session-start"], "not json"),
        (&["session-start"], r#"{"source":"resume"}"#),
        (
            &["session-start"],
            r#"{"session_id":"a b","source":"resume"}"#,
        ),
        (&["turn-start"], "{"),
        (&["turn-start"], r#"{"session_id":42}"#),
        (&["codex-notify", "{"], ""),
    ];
    for (args, stdin) in bad {
        assert_eq!(hook(&app, a, args, stdin, false), 0, "{args:?} {stdin}");
        assert_eq!(stored(&app, ws, a), keep, "{args:?} {stdin}");
    }
    app.quit();
}

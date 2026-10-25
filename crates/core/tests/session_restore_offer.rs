//! The one-time resume offer in the `_session` reply (feature 009, research R3). Agents are
//! fake scripts found through `SONATA_AGENT_BIN_DIR`, set once before any test starts an app.

mod common;

use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::Duration;

use common::{Sandbox, WAIT, messaging_app, shells, sonata_as, wait_until};
use serde_json::{Value, json};
use sonata_core::App;
use sonata_core::app::NewTerminal;
use sonata_core::model::node::{ConversationAgent, Frame};
use uuid::Uuid;

const ID: &str = "7294d08d-ac3c-4e7e-925e-16750c04c269";

/// Folder of fake agents, put in the environment before any test touches it. Every test calls
/// this first, so no thread reads the environment while it changes.
fn agents() -> &'static Path {
    static DIR: OnceLock<PathBuf> = OnceLock::new();
    DIR.get_or_init(|| {
        let dir = std::env::temp_dir().join(format!("sonata-009-agents-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        // A Codex that logs its arguments and every line typed into it.
        common::fake_bin::fake_bin(
            &dir,
            "codex",
            &format!(
                "echo \"$*\" >> '{0}/codex.args'\nwhile IFS= read -r line; do echo \"$line\" >> '{0}/codex.lines'; done",
                dir.display()
            ),
        );
        // SAFETY: runs once, before any test thread starts an app or reads the environment.
        unsafe { std::env::set_var("SONATA_AGENT_BIN_DIR", &dir) };
        dir
    })
}

fn session(app: &App, node: Uuid, agent: &str) -> Value {
    let hook = common::cli_bin().display().to_string();
    let out = sonata_as(app, node, &["_session", agent, &hook, "--json"]);
    assert_eq!(out.code, 0, "{}", out.stderr);
    let v: Value = serde_json::from_str(&out.stdout).unwrap();
    v["data"].clone()
}

fn restart(app: &App, node: Uuid) {
    app.restart_terminal(node, true).unwrap();
    assert!(wait_until(WAIT, || app
        .terminal_ready(node, std::time::Duration::from_millis(300))));
}

#[test]
fn offer_comes_once_per_start_and_only_for_the_same_agent() {
    agents();
    let sb = Sandbox::new();
    let (app, _) = messaging_app(&sb);
    let (_, ids) = shells(&app, &sb, &["A", "B"]);
    let (a, b) = (ids[0], ids[1]);
    app.record_conversation(a, ConversationAgent::Claude, Some(ID))
        .unwrap();

    // Recorded while running: the current process gets no offer.
    assert!(session(&app, a, "claude").get("resume").is_none());

    restart(&app, a);
    let first = session(&app, a, "claude");
    assert_eq!(first["resume"], json!({"agent": "claude", "id": ID}));
    assert!(first["settingsFile"].is_string());
    assert!(session(&app, a, "claude").get("resume").is_none());

    // Another agent gets nothing and uses up the offer.
    restart(&app, a);
    assert!(session(&app, a, "codex").get("resume").is_none());
    assert!(session(&app, a, "claude").get("resume").is_none());

    // A terminal without a conversation never gets an offer.
    restart(&app, b);
    assert!(session(&app, b, "claude").get("resume").is_none());
    app.quit();
}

#[test]
fn reopening_the_app_offers_the_stored_conversation() {
    agents();
    let sb = Sandbox::new();
    let (ws, a) = {
        let (app, _) = messaging_app(&sb);
        let (ws, ids) = shells(&app, &sb, &["A"]);
        app.record_conversation(ids[0], ConversationAgent::Codex, Some("c3"))
            .unwrap();
        app.quit();
        (ws, ids[0])
    };
    let (app, _) = messaging_app(&sb);
    app.open_workspace(ws).unwrap();
    assert!(wait_until(WAIT, || app
        .terminal_ready(a, std::time::Duration::from_millis(300))));
    assert_eq!(
        session(&app, a, "codex")["resume"],
        json!({"agent": "codex", "id": "c3"})
    );
    app.quit();
}

fn lines(path: &Path) -> Vec<String> {
    std::fs::read_to_string(path)
        .map(|s| s.lines().map(str::to_string).collect())
        .unwrap_or_default()
}

#[test]
fn resumed_session_gets_the_same_files_and_typed_instructions_once() {
    let dir = agents();
    let sb = Sandbox::new();
    let (app, _) = messaging_app(&sb);
    let (ws, ids) = shells(&app, &sb, &["A"]);

    // Claude: the same session files with or without a resume offer.
    let fresh = session(&app, ids[0], "claude");
    app.record_conversation(ids[0], ConversationAgent::Claude, Some(ID))
        .unwrap();
    restart(&app, ids[0]);
    let resumed = session(&app, ids[0], "claude");
    assert!(resumed.get("resume").is_some());
    for key in ["instructionsFile", "settingsFile"] {
        assert_eq!(resumed[key], fresh[key], "{key}");
    }

    // Codex: instructions are typed once per start, resumed or not (003 FR-019).
    let frame = Frame {
        x: 0.0,
        y: 500.0,
        width: 640.0,
        height: 400.0,
    };
    let codex = app
        .create_terminal(
            ws,
            NewTerminal {
                frame,
                name: Some("Codex".into()),
                launch_command: Some("codex".into()),
                ..Default::default()
            },
        )
        .unwrap()
        .id;
    let typed = |n: usize| {
        wait_until(Duration::from_secs(15), || {
            lines(&dir.join("codex.lines"))
                .iter()
                .filter(|l| l.contains("running inside Sonata"))
                .count()
                == n
        })
    };
    assert!(typed(1), "screen: {:?}", app.terminals().screen_text(codex));
    app.record_conversation(codex, ConversationAgent::Codex, Some("c3"))
        .unwrap();
    app.restart_terminal(codex, true).unwrap();
    assert!(wait_until(Duration::from_secs(10), || lines(
        &dir.join("codex.args")
    )
    .len()
        == 2));
    assert!(lines(&dir.join("codex.args"))[1].starts_with("resume c3 "));
    assert!(typed(2), "screen: {:?}", app.terminals().screen_text(codex));
    std::thread::sleep(Duration::from_secs(3));
    assert!(typed(2), "typed more than once");
    app.quit();
}

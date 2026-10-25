//! `sonata run` and end-of-turn hooks with fake `claude` and `codex` programs. One test function
//! because it changes the process PATH and HOME.

mod common;

use std::os::unix::fs::PermissionsExt;
use std::time::Duration;

use common::{Sandbox, messaging_app, shells, sonata_as, wait_until};
use sonata_core::app::{NewTerminal, TerminalSettings};
use sonata_core::model::node::{Frame, RoleRef};

const FAKE_CLAUDE: &str = r#"#!/bin/sh
echo "$@" > "$FAKE_LOG/claude.args"
dir=$(mktemp -d)
while IFS= read -r line; do
  echo "$line" >> "$FAKE_LOG/claude.lines"
  printf '{"type":"user","message":{"content":"x"}}\n{"type":"assistant","message":{"content":[{"type":"text","text":"echo:%s"}]}}\n' "$line" > "$dir/t.jsonl"
  printf '{"transcript_path":"%s"}' "$dir/t.jsonl" | sonata _hook claude-stop
done
"#;

const FAKE_CODEX: &str = r#"#!/bin/sh
echo "$@" > "$FAKE_LOG/codex.args"
while IFS= read -r line; do
  echo "$line" >> "$FAKE_LOG/codex.lines"
  sonata _hook codex-notify "{\"type\":\"agent-turn-complete\",\"thread-id\":\"t1\",\"last-assistant-message\":\"codex:$line\"}"
done
"#;

fn install(dir: &std::path::Path, name: &str, body: &str) {
    let p = dir.join(name);
    std::fs::write(&p, body).unwrap();
    std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o755)).unwrap();
}

#[test]
fn agents_get_session_instructions_and_report_turns() {
    let sb = Sandbox::new();
    let fake_bin = sb.tmp.path().join("fakebin");
    let log = sb.tmp.path().join("log");
    let home = sb.tmp.path().join("home");
    for d in [&fake_bin, &log, &home] {
        std::fs::create_dir_all(d).unwrap();
    }
    install(&fake_bin, "claude", FAKE_CLAUDE);
    install(&fake_bin, "codex", FAKE_CODEX);
    // SAFETY: this test binary has a single test, so nothing reads the environment concurrently.
    unsafe {
        std::env::set_var("SONATA_AGENT_BIN_DIR", &fake_bin);
        std::env::set_var("HOME", &home);
        std::env::set_var("FAKE_LOG", &log);
    }

    let (app, _) = messaging_app(&sb);
    let (ws, ids) = shells(&app, &sb, &["Asker"]);
    let frame = |x: f64| Frame {
        x,
        y: 500.0,
        width: 640.0,
        height: 400.0,
    };
    let claude = app
        .create_terminal(
            ws,
            NewTerminal {
                frame: frame(0.0),
                name: Some("Claude".into()),
                launch_command: Some("claude --model test".into()),
                settings: TerminalSettings {
                    role: Some(Some(RoleRef {
                        name: None,
                        text: "You review diffs.".into(),
                    })),
                    ..Default::default()
                },
                ..Default::default()
            },
        )
        .unwrap()
        .id;
    let codex = app
        .create_terminal(
            ws,
            NewTerminal {
                frame: frame(700.0),
                name: Some("Codex".into()),
                launch_command: Some("codex".into()),
                ..Default::default()
            },
        )
        .unwrap()
        .id;

    // Claude Code: session-only options, nothing in the user's config.
    let started = wait_until(Duration::from_secs(10), || log.join("claude.args").exists());
    assert!(started, "screen: {:?}", app.terminals().screen_text(claude));
    let args = std::fs::read_to_string(log.join("claude.args")).unwrap();
    assert!(
        args.contains("--append-system-prompt-file")
            && args.contains("--settings")
            && args.ends_with("--model test\n"),
        "{args}"
    );
    let words: Vec<&str> = args.split_whitespace().collect();
    let file_after =
        |flag: &str| words[words.iter().position(|w| *w == flag).unwrap() + 1].to_string();
    let instructions = std::fs::read_to_string(file_after("--append-system-prompt-file")).unwrap();
    assert!(
        instructions.contains("\"Claude\"")
            && instructions.contains("You review diffs.")
            && instructions.contains("sonata ask"),
        "{instructions}"
    );
    let settings: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(file_after("--settings")).unwrap()).unwrap();
    let hooks: Vec<&String> = settings["hooks"].as_object().unwrap().keys().collect();
    assert_eq!(hooks, ["SessionStart", "Stop", "UserPromptSubmit"]);
    assert_eq!(
        settings.as_object().unwrap().len(),
        1,
        "settings must contain hooks only"
    );
    assert!(!home.join(".claude").exists(), "user config touched");

    // Codex: notify override; typed instructions arrive once it is ready.
    assert!(wait_until(Duration::from_secs(10), || log
        .join("codex.args")
        .exists()));
    let cargs = std::fs::read_to_string(log.join("codex.args")).unwrap();
    assert!(
        cargs.contains("notify=[") && cargs.contains("codex-notify"),
        "{cargs}"
    );
    assert!(wait_until(Duration::from_secs(15), || {
        std::fs::read_to_string(log.join("codex.lines"))
            .is_ok_and(|l| l.contains("running inside Sonata"))
    }));

    // Answers come from the agents' end-of-turn signals.
    app.create_link(ws, ids[0], claude).unwrap();
    app.create_link(ws, ids[0], codex).unwrap();
    let out = sonata_as(&app, ids[0], &["ask", "Claude", "ping"]);
    assert_eq!(out.code, 0, "{}", out.stderr);
    assert_eq!(out.stdout.trim(), "echo:ping");
    let out = sonata_as(&app, ids[0], &["ask", "Codex", "pong"]);
    assert_eq!(out.code, 0, "{}", out.stderr);
    assert_eq!(out.stdout.trim(), "codex:pong");
    app.quit();
}

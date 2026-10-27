//! Kimi terminals running a fake `kimi` (feature 010): typed instructions and how asks end.
//! One test function because it changes the process PATH-related environment and HOME.

mod common;

use std::time::{Duration, Instant};

use common::{Sandbox, install_fake_kimi, messaging_app, shells, sonata_as, wait_until};
use sonata_core::App;
use sonata_core::app::{NewTerminal, TerminalSettings};
use sonata_core::model::node::{ConversationAgent, Frame};
use uuid::Uuid;

fn kimi_terminal(app: &App, ws: Uuid, name: &str, x: f64, send: Option<bool>) -> Uuid {
    app.create_terminal(
        ws,
        NewTerminal {
            frame: Frame {
                x,
                y: 500.0,
                width: 640.0,
                height: 400.0,
            },
            name: Some(name.into()),
            launch_command: Some("kimi".into()),
            settings: TerminalSettings {
                send_instructions: send.map(Some),
                ..Default::default()
            },
            ..Default::default()
        },
    )
    .unwrap()
    .id
}

#[test]
fn kimi_terminals_get_instructions_and_end_turns() {
    let sb = Sandbox::new();
    let bin = sb.tmp.path().join("fakebin");
    let log = sb.tmp.path().join("log");
    let home = sb.tmp.path().join("home");
    for d in [&log, &home] {
        std::fs::create_dir_all(d).unwrap();
    }
    install_fake_kimi(&bin);
    // SAFETY: this test binary has a single test, so nothing reads the environment concurrently.
    unsafe {
        std::env::set_var("SONATA_AGENT_BIN_DIR", &bin);
        std::env::set_var("HOME", &home);
        std::env::set_var("FAKE_LOG", &log);
    }
    let (app, _) = messaging_app(&sb);
    let (ws, ids) = shells(&app, &sb, &["Asker"]);
    let asker = ids[0];

    // Typed instructions are off by default for Kimi: the agent file carries them (T018).
    let kimi = kimi_terminal(&app, ws, "Kimi", 0.0, None);
    assert!(wait_until(Duration::from_secs(10), || log
        .join("kimi.args")
        .exists()));
    let args = std::fs::read_to_string(log.join("kimi.args")).unwrap();
    assert!(args.starts_with("--agent-file "), "{args}");
    std::thread::sleep(Duration::from_secs(6));
    let lines = std::fs::read_to_string(log.join("kimi.lines")).unwrap_or_default();
    assert!(!lines.contains("running inside Sonata"), "{lines}");

    // T055: a restart resumes the stored Kimi conversation through the shell helper; Start
    // fresh forgets it, and the new start gets the agent file and no `--session`.
    let restarted_with = |restart: &dyn Fn()| {
        let _ = std::fs::remove_file(log.join("kimi.args"));
        restart();
        assert!(wait_until(Duration::from_secs(10), || log
            .join("kimi.args")
            .exists()));
        std::fs::read_to_string(log.join("kimi.args")).unwrap()
    };
    app.record_conversation(kimi, ConversationAgent::Kimi, Some("session_abc"))
        .unwrap();
    let args = restarted_with(&|| app.restart_terminal(kimi, true).unwrap());
    assert_eq!(args.trim(), "--session session_abc");
    let args = restarted_with(&|| app.start_fresh_conversation(kimi, true).unwrap());
    assert!(
        args.starts_with("--agent-file ") && !args.contains("--session"),
        "{args}"
    );
    assert!(
        app.workspaces()
            .workspace(ws)
            .and_then(|w| w.node(kimi).cloned())
            .and_then(|n| n.terminal().and_then(|t| t.conversation.clone()))
            .is_none()
    );

    let typed = kimi_terminal(&app, ws, "Typed", 700.0, Some(true));
    assert!(wait_until(Duration::from_secs(15), || {
        std::fs::read_to_string(log.join("kimi.lines"))
            .is_ok_and(|l| l.contains("running inside Sonata"))
    }));
    app.delete_node(ws, typed, true).unwrap();

    app.create_link(ws, asker, kimi).unwrap();
    let out = sonata_as(&app, asker, &["ask", "Kimi", "ping"]);
    assert_eq!(out.code, 0, "{}", out.stderr);
    assert!(out.stdout.contains("kimi:ping"), "{}", out.stdout);

    // T046: a hooked turn with a 25 s silence (longer than the 20 s hooked fallback) ends only on
    // Kimi's stop hook, with the full answer. This terminal's first ask starts as `Probing` and
    // switches when the prompt hook arrives.
    unsafe { std::env::set_var("FAKE_KIMI_PAUSE", "25") };
    let slow = kimi_terminal(&app, ws, "Slow", 1400.0, None);
    app.create_link(ws, asker, slow).unwrap();
    assert!(wait_until(Duration::from_secs(10), || app
        .terminal_ready(slow, Duration::from_millis(500))));
    let start = Instant::now();
    let out = sonata_as(&app, asker, &["ask", "Slow", "think"]);
    let took = start.elapsed();
    assert_eq!(out.code, 0, "{}", out.stderr);
    assert!(out.stdout.contains("kimi:think"), "{}", out.stdout);
    assert!(
        took >= Duration::from_secs(25) && took < Duration::from_secs(29),
        "{took:?}"
    );
    unsafe { std::env::remove_var("FAKE_KIMI_PAUSE") };

    // Without hooks, Kimi answers by the quiet-screen rule (3 s).
    unsafe { std::env::set_var("FAKE_KIMI_NO_HOOKS", "1") };
    let plain = kimi_terminal(&app, ws, "Plain", 2100.0, None);
    app.create_link(ws, asker, plain).unwrap();
    assert!(wait_until(Duration::from_secs(10), || app
        .terminal_ready(plain, Duration::from_millis(500))));
    let start = Instant::now();
    let out = sonata_as(&app, asker, &["ask", "Plain", "hi"]);
    assert_eq!(out.code, 0, "{}", out.stderr);
    assert!(out.stdout.contains("kimi:hi"), "{}", out.stdout);
    assert!(
        start.elapsed() < Duration::from_secs(10),
        "{:?}",
        start.elapsed()
    );

    // A hooked Kimi whose prompt hook does not fire for an ask still ends by the hooked fallback.
    assert_eq!(sonata_as(&app, plain, &["_turn", "complete"]).code, 0);
    let start = Instant::now();
    let out = sonata_as(&app, asker, &["ask", "Plain", "again"]);
    assert_eq!(out.code, 0, "{}", out.stderr);
    let took = start.elapsed();
    assert!(
        took >= Duration::from_secs(19) && took < Duration::from_secs(25),
        "{took:?}"
    );
    unsafe { std::env::remove_var("FAKE_KIMI_NO_HOOKS") };
    app.quit();
}

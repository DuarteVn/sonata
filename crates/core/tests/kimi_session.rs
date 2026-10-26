//! Kimi terminals running a fake `kimi` (feature 010): typed instructions and how asks end.
//! One test function because it changes the process PATH-related environment and HOME.

mod common;

use std::time::Duration;

use common::{Sandbox, install_fake_kimi, messaging_app, shells, sonata_as, wait_until};
use sonata_core::App;
use sonata_core::app::{NewTerminal, TerminalSettings};
use sonata_core::model::node::Frame;
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
    app.quit();
}

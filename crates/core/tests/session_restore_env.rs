//! Terminals and the usage probe never inherit an outer agent's session markers (feature 009,
//! research R5). One test function, because it changes the process environment.

mod common;

use std::time::Duration;

use common::{Sandbox, shells, test_app, wait_until};
use sonata_core::app::NewTerminal;
use sonata_core::model::node::Frame;
use sonata_core::terminal::pty::AGENT_SESSION_MARKERS;
use sonata_core::usage::runner::{RunSpec, run};

const SET: [(&str, &str); 11] = [
    ("CLAUDECODE", "1"),
    ("CLAUDE_CODE_CHILD_SESSION", "1"),
    ("CLAUDE_CODE_MESSAGING_TOKEN", "secret"),
    ("CLAUDE_PID", "1"),
    ("CODEX_SANDBOX", "seatbelt"),
    ("CLAUDE_CODE_USE_BEDROCK", "1"),
    ("ANTHROPIC_MODEL", "x"),
    // Kimi Code (feature 010): two markers of a running Kimi, two user settings.
    ("KIMI_PLUGIN_ROOT", "/x"),
    ("KIMI_CODE_UPDATE_REEXEC", "1"),
    ("KIMI_CODE_HOME", "/k"),
    ("NO_COLOR", "1"),
];

fn has(env: &str, key: &str) -> bool {
    env.lines().any(|l| l.starts_with(&format!("{key}=")))
}

#[test]
fn agent_session_markers_are_removed_and_user_settings_kept() {
    // SAFETY: this test binary has a single test, so nothing reads the environment concurrently.
    unsafe {
        for (k, v) in SET {
            std::env::set_var(k, v);
        }
    }

    // Terminals.
    let sb = Sandbox::new();
    let (app, _) = test_app(&sb);
    let (ws, _) = shells(&app, &sb, &["A"]);
    let out = sb.tmp.path().join("env.txt");
    let frame = Frame {
        x: 0.0,
        y: 500.0,
        width: 640.0,
        height: 400.0,
    };
    app.create_terminal(
        ws,
        NewTerminal {
            frame,
            launch_command: Some(format!("env > '{}'", out.display())),
            ..Default::default()
        },
    )
    .unwrap();
    assert!(wait_until(Duration::from_secs(10), || {
        std::fs::read_to_string(&out).is_ok_and(|s| has(&s, "SONATA_TERMINAL_ID"))
    }));
    let env = std::fs::read_to_string(&out).unwrap();
    for key in AGENT_SESSION_MARKERS {
        assert!(!has(&env, key), "{key} leaked into a terminal");
    }
    assert!(has(&env, "CLAUDE_CODE_USE_BEDROCK") && has(&env, "ANTHROPIC_MODEL"));
    for key in ["KIMI_PLUGIN_ROOT", "KIMI_CODE_UPDATE_REEXEC"] {
        assert!(!has(&env, key), "{key} leaked into a terminal");
    }
    assert!(has(&env, "KIMI_CODE_HOME") && has(&env, "NO_COLOR"));
    app.quit();

    // The usage probe's child, even when a marker is passed on explicitly.
    let mut spec = RunSpec::new(vec!["/usr/bin/env".into()], Duration::from_secs(5));
    spec.env = SET
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
    let env = String::from_utf8(run(&spec).unwrap().stdout).unwrap();
    for key in AGENT_SESSION_MARKERS {
        assert!(!has(&env, key), "{key} leaked into the usage probe");
    }
    assert!(has(&env, "CLAUDE_CODE_USE_BEDROCK"));
}

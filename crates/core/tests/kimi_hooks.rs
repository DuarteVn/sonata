//! Kimi Code CLI through Sonata (feature 010): session files, the `sonata run kimi` wrapper,
//! recruit, and `sonata _hook kimi …`. Nothing here changes the process environment; the
//! wrapper runs with its own environment and a fake `kimi`.

mod common;

use std::path::Path;
use std::process::{Command, Stdio};

use common::{CliOut, Sandbox, cli_bin, install_fake_kimi, messaging_app, shells, sonata_as};
use sonata_core::App;
use sonata_core::app::TerminalSettings;
use sonata_core::model::node::RoleRef;
use uuid::Uuid;

/// Runs `sonata <args>` as terminal `id`, with `extra` environment and the fake `kimi` folder.
fn sonata_env(app: &App, id: Uuid, args: &[&str], extra: &[(&str, &Path)]) -> CliOut {
    let mut cmd = Command::new(cli_bin());
    cmd.args(args)
        .env("SONATA_SOCKET", app.socket_path().expect("socket"))
        .env("SONATA_TERMINAL_ID", id.to_string())
        .env(
            "SONATA_TOKEN",
            app.terminals().session_token(id).expect("token"),
        )
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for (k, v) in extra {
        cmd.env(k, v);
    }
    let out = cmd.output().expect("run sonata");
    CliOut {
        code: out.status.code().unwrap_or(-1),
        stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
    }
}

#[test]
fn session_reply_has_the_kimi_agent_file() {
    let sb = Sandbox::new();
    let (app, _) = messaging_app(&sb);
    let (ws, ids) = shells(&app, &sb, &["K"]);
    app.update_terminal_settings(
        ids[0],
        TerminalSettings {
            role: Some(Some(RoleRef {
                name: None,
                text: "You review diffs.".into(),
            })),
            ..Default::default()
        },
    )
    .unwrap();
    let out = sonata_as(&app, ids[0], &["--json", "_session", "kimi", "/x/sonata"]);
    assert_eq!(out.code, 0, "{}", out.stderr);
    let v: serde_json::Value = serde_json::from_str(&out.stdout).expect(&out.stdout);
    assert_eq!(v["data"]["agent"], "kimi");
    let file = v["data"]["agentFile"]
        .as_str()
        .expect("agentFile")
        .to_string();
    assert!(file.ends_with("kimi-agent.md"), "{file}");
    let text = std::fs::read_to_string(&file).unwrap();
    assert!(text.contains("You review diffs.") && text.contains("${base_prompt}"));
    app.delete_node(ws, ids[0], true).unwrap();
    assert!(
        !Path::new(&file).exists(),
        "session files removed with the terminal"
    );
    app.quit();
}

#[test]
fn wrapper_adds_the_agent_file_only_for_new_conversations() {
    let sb = Sandbox::new();
    let (app, _) = messaging_app(&sb);
    let (_, ids) = shells(&app, &sb, &["K"]);
    let bin = sb.tmp.path().join("fakebin");
    let log = sb.tmp.path().join("log");
    std::fs::create_dir_all(&log).unwrap();
    install_fake_kimi(&bin);
    let env = [
        ("SONATA_AGENT_BIN_DIR", bin.as_path()),
        ("FAKE_LOG", log.as_path()),
    ];
    let run = |args: &[&str]| -> String {
        let _ = std::fs::remove_file(log.join("kimi.args"));
        let mut full = vec!["run", "kimi"];
        full.extend_from_slice(args);
        let out = sonata_env(&app, ids[0], &full, &env);
        assert_eq!(out.code, 0, "{args:?}: {}", out.stderr);
        std::fs::read_to_string(log.join("kimi.args"))
            .unwrap()
            .trim_end()
            .to_string()
    };

    let args = run(&["--yolo"]);
    let words: Vec<&str> = args.split_whitespace().collect();
    assert_eq!(words.len(), 3, "{args}");
    assert_eq!(words[0], "--agent-file");
    assert!(words[1].ends_with("kimi-agent.md"), "{args}");
    assert_eq!(words[2], "--yolo");

    let choosing: [&[&str]; 11] = [
        &["-S", "x"],
        &["--session", "x"],
        &["-r", "x"],
        &["--resume", "x"],
        &["-c"],
        &["--continue"],
        &["-C"],
        &["--agent", "a"],
        &["--agent-file", "f"],
        &["-p", "hi"],
        &["--prompt", "hi"],
    ];
    for a in choosing {
        assert_eq!(run(a), a.join(" "), "{a:?}");
    }
    for sub in [
        "login",
        "acp",
        "web",
        "app",
        "install-desktop",
        "doctor",
        "export",
        "fork",
        "session",
        "provider",
        "vis",
        "migrate",
        "upgrade",
        "update",
    ] {
        assert_eq!(run(&[sub, "x"]), format!("{sub} x"), "{sub}");
    }
    app.quit();
}

#[test]
fn wrapper_outside_sonata_and_missing_kimi() {
    let tmp = tempfile::tempdir().unwrap();
    let bin = tmp.path().join("fakebin");
    let log = tmp.path().join("log");
    std::fs::create_dir_all(&log).unwrap();
    install_fake_kimi(&bin);
    let out = Command::new(cli_bin())
        .args(["run", "kimi", "x"])
        .env_remove("SONATA_SOCKET")
        .env_remove("SONATA_TERMINAL_ID")
        .env_remove("SONATA_TOKEN")
        .env("SONATA_AGENT_BIN_DIR", &bin)
        .env("FAKE_LOG", &log)
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert!(out.status.success(), "{out:?}");
    assert_eq!(
        std::fs::read_to_string(log.join("kimi.args")).unwrap(),
        "x\n"
    );

    // No `kimi` anywhere: the existing missing-agent error (FR-003).
    let empty = tmp.path().join("empty");
    std::fs::create_dir_all(&empty).unwrap();
    let out = Command::new(cli_bin())
        .args(["run", "kimi"])
        .env("SONATA_AGENT_BIN_DIR", &empty)
        .env("PATH", &empty)
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(127));
    assert!(
        String::from_utf8_lossy(&out.stderr)
            .contains("sonata: \"kimi\" was not found on your PATH."),
        "{out:?}"
    );
}

#[test]
fn orchestrator_recruits_kimi() {
    let sb = Sandbox::new();
    let (app, _) = messaging_app(&sb);
    let (ws, ids) = shells(&app, &sb, &["Lead"]);
    app.update_terminal_settings(
        ids[0],
        TerminalSettings {
            orchestrator: Some(true),
            ..Default::default()
        },
    )
    .unwrap();
    let out = sonata_as(&app, ids[0], &["recruit", "kimi", "--name", "K"]);
    assert_eq!(out.code, 0, "{}", out.stderr);
    let m = app.workspaces();
    let w = m.workspace(ws).unwrap();
    let k = w
        .nodes
        .iter()
        .find(|n| n.terminal().is_some_and(|t| t.name == "K"))
        .expect("recruited");
    assert_eq!(
        k.terminal().unwrap().launch_command.as_deref(),
        Some("kimi")
    );
    assert!(w.links.iter().any(|l| l.joins(ids[0], k.id)));
    drop(m);
    let usage = sonata_as(&app, ids[0], &["recruit"]);
    assert!(usage.stderr.contains("kimi"), "{}", usage.stderr);
    app.quit();
}

fn quiet(out: &CliOut, what: &str) {
    assert_eq!(out.code, 0, "{what}: {}", out.stderr);
    assert!(out.stdout.is_empty(), "{what}: stdout {:?}", out.stdout);
    assert!(out.stderr.is_empty(), "{what}: stderr {:?}", out.stderr);
}

/// The `sonata list` state word for `name`, as seen by `asker`.
fn listed_state(app: &App, asker: Uuid, name: &str) -> String {
    let out = sonata_as(app, asker, &["list"]);
    out.stdout
        .lines()
        .find(|l| l.split_whitespace().nth(1) == Some(name))
        .and_then(|l| {
            ["working", "idle", "exited", "starting"]
                .into_iter()
                .find(|w| l.split_whitespace().any(|x| x == *w))
        })
        .unwrap_or_else(|| panic!("{name} not in: {}", out.stdout))
        .to_string()
}

#[test]
fn kimi_hooks_report_turns_quietly() {
    use common::{WAIT, sonata_with, wait_until};
    use sonata_core::app::NewTerminal;
    use std::time::Duration;

    let sb = Sandbox::new();
    let (app, _) = messaging_app(&sb);
    let (ws, ids) = shells(&app, &sb, &["Asker"]);
    let asker = ids[0];
    let new = |name: &str, cmd: &str, x: f64| {
        let id = app
            .create_terminal(
                ws,
                NewTerminal {
                    frame: sonata_core::model::node::Frame {
                        x,
                        y: 500.0,
                        width: 640.0,
                        height: 400.0,
                    },
                    name: Some(name.into()),
                    launch_command: Some(cmd.into()),
                    ..Default::default()
                },
            )
            .unwrap()
            .id;
        app.create_link(ws, asker, id).unwrap();
        // No `kimi` here: the wrapper says so and the shell stays at its prompt.
        assert!(
            wait_until(WAIT, || app
                .terminals()
                .screen_text(id)
                .is_some_and(|s| s.contains("was not found"))),
            "{name}: {:?}",
            app.terminals().screen_text(id)
        );
        assert!(wait_until(WAIT, || app.terminal_ready(id, Duration::from_millis(300))));
        id
    };
    let k = new("K", "kimi", 0.0);
    let payload = r#"{"hook_event_name":"UserPromptSubmit","session_id":"session_a","cwd":"."}"#;

    assert!(!app.terminals().agent_hooked(k));
    assert_eq!(listed_state(&app, asker, "K"), "idle");

    let out = sonata_with(&app, k, &["_hook", "kimi", "prompt-submit"], Some(payload));
    quiet(&out, "prompt-submit");
    assert!(app.terminals().agent_hooked(k));
    assert!(app.terminals().turn_active(k));
    assert_eq!(listed_state(&app, asker, "K"), "working");

    for event in ["stop", "stop-failure", "interrupt"] {
        quiet(
            &sonata_with(&app, k, &["_hook", "kimi", "prompt-submit"], Some(payload)),
            event,
        );
        assert!(app.terminals().turn_active(k), "{event}");
        let out = sonata_with(
            &app,
            k,
            &["_hook", "kimi", event],
            Some(r#"{"session_id":"session_a"}"#),
        );
        quiet(&out, event);
        assert!(!app.terminals().turn_active(k), "{event}");
        assert_eq!(listed_state(&app, asker, "K"), "idle", "{event}");
    }

    // Unknown events and bad payloads report nothing and never fail.
    quiet(
        &sonata_with(&app, k, &["_hook", "kimi", "nope"], Some("{}")),
        "unknown",
    );
    quiet(
        &sonata_with(
            &app,
            k,
            &["_hook", "kimi", "prompt-submit"],
            Some("not json"),
        ),
        "bad json",
    );
    assert!(!app.terminals().turn_active(k), "bad json reports nothing");

    // Outside a Sonata terminal, or with Sonata gone: silent success.
    for env in [
        vec![],
        vec![
            ("SONATA_TERMINAL_ID", "x"),
            ("SONATA_SOCKET", "/nonexistent/s"),
        ],
    ] {
        let mut cmd = Command::new(cli_bin());
        cmd.args(["_hook", "kimi", "stop"])
            .env_remove("SONATA_TERMINAL_ID")
            .env_remove("SONATA_SOCKET")
            .env_remove("SONATA_TOKEN")
            .stdin(Stdio::null());
        for (key, value) in &env {
            cmd.env(key, value);
        }
        let out = cmd.output().unwrap();
        assert!(out.status.success(), "{env:?}");
        assert!(
            out.stdout.is_empty() && out.stderr.is_empty(),
            "{env:?}: {out:?}"
        );
    }

    // A restart is a new process: not hooked until its hooks report again.
    app.restart_terminal(k, true).unwrap();
    assert!(!app.terminals().agent_hooked(k));
    assert!(!app.terminals().turn_active(k));

    // Other terminals keep the foreground-program status even when a turn hook reports. (A
    // plain shell stands in for Claude Code, which must not really start in tests.)
    let s = app
        .create_terminal(
            ws,
            NewTerminal {
                name: Some("S".into()),
                ..Default::default()
            },
        )
        .unwrap()
        .id;
    app.create_link(ws, asker, s).unwrap();
    assert!(wait_until(WAIT, || app.terminal_ready(s, Duration::from_millis(300))));
    assert_eq!(sonata_as(&app, s, &["_turn", "start"]).code, 0);
    assert!(app.terminals().turn_active(s));
    assert_eq!(listed_state(&app, asker, "S"), "idle");
    app.quit();
}

/// Runs `sonata _hook kimi <event>` as terminal `id` with `payload` on stdin; `off` sets
/// `SONATA_CONVERSATION=off` like a `kimi -p` run.
fn kimi_hook(app: &App, id: Uuid, event: &str, payload: &str, off: bool) -> CliOut {
    use std::io::Write;
    let mut cmd = Command::new(cli_bin());
    cmd.args(["_hook", "kimi", event])
        .env("SONATA_SOCKET", app.socket_path().expect("socket"))
        .env("SONATA_TERMINAL_ID", id.to_string())
        .env("SONATA_TOKEN", app.terminals().session_token(id).unwrap())
        .env_remove("SONATA_CONVERSATION")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if off {
        cmd.env("SONATA_CONVERSATION", "off");
    }
    let mut child = cmd.spawn().unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(payload.as_bytes())
        .unwrap();
    let out = child.wait_with_output().unwrap();
    CliOut {
        code: out.status.code().unwrap_or(-1),
        stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
    }
}

#[test]
fn kimi_hooks_report_the_conversation() {
    use sonata_core::model::node::ConversationAgent;

    let sb = Sandbox::new();
    let (app, _) = messaging_app(&sb);
    let (ws, ids) = shells(&app, &sb, &["K"]);
    let k = ids[0];
    let stored = || {
        app.workspaces()
            .workspace(ws)
            .and_then(|w| w.node(k).cloned())
            .and_then(|n| n.terminal().and_then(|t| t.conversation.clone()))
            .map(|c| (c.agent, c.id))
    };
    let start = |source: &str, id: &str| {
        format!(r#"{{"hook_event_name":"SessionStart","source":"{source}","session_id":"{id}"}}"#)
    };
    let kimi = |id: &str| Some((ConversationAgent::Kimi, id.to_string()));

    // SessionStart: `resume` records the session, `startup` clears it, others do nothing.
    quiet(
        &kimi_hook(&app, k, "session-start", &start("resume", "session_r"), false),
        "resume",
    );
    assert_eq!(stored(), kimi("session_r"));
    quiet(
        &kimi_hook(&app, k, "session-start", &start("other", "session_o"), false),
        "other",
    );
    assert_eq!(stored(), kimi("session_r"));
    quiet(
        &kimi_hook(&app, k, "session-start", &start("startup", "session_n"), false),
        "startup",
    );
    assert_eq!(stored(), None);

    // UserPromptSubmit starts the turn and records the session it runs in.
    let prompt = r#"{"hook_event_name":"UserPromptSubmit","session_id":"session_p","cwd":"."}"#;
    quiet(&kimi_hook(&app, k, "prompt-submit", prompt, false), "prompt");
    assert!(app.terminals().turn_active(k));
    assert_eq!(stored(), kimi("session_p"));
    quiet(
        &kimi_hook(&app, k, "stop", r#"{"session_id":"session_p"}"#, false),
        "stop",
    );

    // `kimi -p` runs report turns but never touch the stored conversation.
    let other = r#"{"hook_event_name":"UserPromptSubmit","session_id":"session_x","cwd":"."}"#;
    quiet(&kimi_hook(&app, k, "prompt-submit", other, true), "off");
    assert!(app.terminals().turn_active(k));
    assert_eq!(stored(), kimi("session_p"));
    quiet(
        &kimi_hook(&app, k, "session-start", &start("startup", "s"), true),
        "off start",
    );
    assert_eq!(stored(), kimi("session_p"));

    // An id Sonata would refuse is not reported, and the hook still succeeds quietly.
    quiet(
        &kimi_hook(&app, k, "session-start", &start("resume", "a;b"), false),
        "bad id",
    );
    assert_eq!(stored(), kimi("session_p"));
    app.quit();
}

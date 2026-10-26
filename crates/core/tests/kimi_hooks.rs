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

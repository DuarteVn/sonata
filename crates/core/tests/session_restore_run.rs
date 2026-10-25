//! `sonata run` resuming the stored conversation, with fake agents (feature 009). The wrapper
//! runs as a subprocess with the terminal's identity, so no test changes the process
//! environment.

mod common;

use std::path::Path;
use std::process::{Command, Output, Stdio};

use common::{
    Sandbox, WAIT, fake_agent_args, install_fake_agent, messaging_app, shells, wait_until,
};
use sonata_core::App;
use sonata_core::model::node::ConversationAgent;
use uuid::Uuid;

const ID: &str = "7294d08d-ac3c-4e7e-925e-16750c04c269";

/// Runs `sonata run <args>` as terminal `node` would, with fake agents from `bin`.
fn run_as(app: &App, node: Uuid, bin: &Path, args: &[&str], env: &[(&str, &str)]) -> Output {
    let mut cmd = Command::new(common::cli_bin());
    cmd.arg("run")
        .args(args)
        .env("SONATA_SOCKET", app.socket_path().unwrap())
        .env("SONATA_TERMINAL_ID", node.to_string())
        .env("SONATA_TOKEN", app.terminals().session_token(node).unwrap())
        .env("SONATA_AGENT_BIN_DIR", bin)
        .env_remove("SONATA_CONVERSATION")
        .stdin(Stdio::null());
    for (k, v) in env {
        cmd.env(k, v);
    }
    cmd.output().unwrap()
}

/// A terminal whose next start offers `agent`'s conversation `id`.
fn offered(app: &App, node: Uuid, agent: ConversationAgent, id: &str) {
    app.record_conversation(node, agent, Some(id)).unwrap();
    app.restart_terminal(node, true).unwrap();
    assert!(wait_until(WAIT, || app
        .terminal_ready(node, std::time::Duration::from_millis(300))));
}

fn words(line: &str) -> Vec<&str> {
    line.split_whitespace().collect()
}

#[test]
fn claude_resumes_the_offered_conversation_with_session_files() {
    let sb = Sandbox::new();
    let bin = sb.tmp.path().join("bin");
    install_fake_agent(&bin, "claude", 0, 0);
    let (app, _) = messaging_app(&sb);
    let (_, ids) = shells(&app, &sb, &["A"]);
    offered(&app, ids[0], ConversationAgent::Claude, ID);

    let out = run_as(&app, ids[0], &bin, &["claude", "--model", "x"], &[]);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    let args = fake_agent_args(&bin, "claude");
    assert_eq!(args.len(), 1, "{args:?}");
    let w = words(&args[0]);
    assert_eq!(&w[..2], ["--resume", ID], "{args:?}");
    assert!(w.contains(&"--settings") && w.contains(&"--append-system-prompt-file"));
    assert!(args[0].ends_with("--model x"), "{args:?}");

    // The offer is used up: the next run starts a new conversation.
    run_as(&app, ids[0], &bin, &["claude"], &[]);
    let args = fake_agent_args(&bin, "claude");
    assert!(!args[1].contains("--resume"), "{args:?}");
    app.quit();
}

#[test]
fn codex_resumes_with_its_subcommand_and_notify_override() {
    let sb = Sandbox::new();
    let bin = sb.tmp.path().join("bin");
    install_fake_agent(&bin, "codex", 0, 0);
    let (app, _) = messaging_app(&sb);
    let (_, ids) = shells(&app, &sb, &["A"]);
    offered(&app, ids[0], ConversationAgent::Codex, "c3");

    let out = run_as(&app, ids[0], &bin, &["codex", "--model", "o"], &[]);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    let args = fake_agent_args(&bin, "codex");
    let w = words(&args[0]);
    assert_eq!(&w[..2], ["resume", "c3"], "{args:?}");
    assert!(
        args[0].contains("notify=[") && args[0].ends_with("--model o"),
        "{args:?}"
    );
    app.quit();
}

#[test]
fn arguments_that_pick_a_conversation_or_a_subcommand_get_no_resume() {
    let sb = Sandbox::new();
    let bin = sb.tmp.path().join("bin");
    install_fake_agent(&bin, "claude", 0, 0);
    install_fake_agent(&bin, "codex", 0, 0);
    let (app, _) = messaging_app(&sb);
    let (_, ids) = shells(&app, &sb, &["A"]);
    let claude_cases: &[&[&str]] = &[
        &["-r"],
        &["--resume", "x"],
        &["-c"],
        &["--continue"],
        &["--session-id", "u"],
        &["--from-pr", "1"],
        &["--teleport"],
        &["-p", "hi"],
        &["--print"],
        &["mcp", "list"],
    ];
    for case in claude_cases {
        offered(&app, ids[0], ConversationAgent::Claude, ID);
        let mut args = vec!["claude"];
        args.extend_from_slice(case);
        run_as(&app, ids[0], &bin, &args, &[]);
        let last = fake_agent_args(&bin, "claude").pop().unwrap();
        assert!(!last.contains(ID), "{case:?}: {last}");
        assert!(last.ends_with(&case.join(" ")), "{case:?}: {last}");
        // The offer was used up anyway.
        run_as(&app, ids[0], &bin, &["claude"], &[]);
        let last = fake_agent_args(&bin, "claude").pop().unwrap();
        assert!(!last.contains(ID), "{case:?}: {last}");
    }
    offered(&app, ids[0], ConversationAgent::Codex, "c3");
    run_as(&app, ids[0], &bin, &["codex", "exec", "x"], &[]);
    let last = fake_agent_args(&bin, "codex").pop().unwrap();
    assert!(!last.contains("c3") && last.ends_with("exec x"), "{last}");
    app.quit();
}

#[test]
fn without_an_offer_arguments_and_exit_status_are_unchanged() {
    let sb = Sandbox::new();
    let bin = sb.tmp.path().join("bin");
    install_fake_agent(&bin, "claude", 3, 0);
    let (app, _) = messaging_app(&sb);
    let (_, ids) = shells(&app, &sb, &["A"]);
    let out = run_as(&app, ids[0], &bin, &["claude", "--model", "x"], &[]);
    assert_eq!(out.status.code(), Some(3));
    let args = fake_agent_args(&bin, "claude");
    assert!(!args[0].contains("--resume") && args[0].ends_with("--model x"));
    app.quit();
}

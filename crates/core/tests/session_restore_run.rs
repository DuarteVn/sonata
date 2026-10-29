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

#[test]
fn print_mode_turns_conversation_reports_off() {
    let sb = Sandbox::new();
    let bin = sb.tmp.path().join("bin");
    let env_file = bin.join("claude.env");
    common::fake_bin::fake_bin(&bin, "claude", &format!("env > '{}'", env_file.display()));
    let (app, _) = messaging_app(&sb);
    let (_, ids) = shells(&app, &sb, &["A"]);
    let off = |args: &[&str]| {
        let _ = std::fs::remove_file(&env_file);
        run_as(&app, ids[0], &bin, args, &[]);
        std::fs::read_to_string(&env_file)
            .unwrap()
            .lines()
            .any(|l| l == "SONATA_CONVERSATION=off")
    };
    assert!(off(&["claude", "-p", "hi"]));
    assert!(off(&["claude", "--print", "hi"]));
    assert!(!off(&["claude"]));
    assert!(!off(&["claude", "--model", "x"]));
    app.quit();
}

const NOTICE: &str = "sonata: the previous conversation could not be resumed; starting a new one.";

/// A fake agent that exits 1 after `delay` when its first argument is `first`, else 0 at once.
fn failing_resume(bin: &Path, name: &str, first: &str, delay: &str) {
    let log = bin.join(format!("{name}.args"));
    common::fake_bin::fake_bin(
        bin,
        name,
        &format!(
            "echo \"$*\" >> '{}'\nif [ \"$1\" = \"{first}\" ]; then sleep {delay}; exit 1; fi\nexit 0",
            log.display()
        ),
    );
}

#[test]
fn failed_resume_prints_one_notice_and_starts_a_new_conversation() {
    let sb = Sandbox::new();
    let bin = sb.tmp.path().join("bin");
    failing_resume(&bin, "claude", "--resume", "0");
    failing_resume(&bin, "codex", "resume", "0");
    let (app, _) = messaging_app(&sb);
    let (_, ids) = shells(&app, &sb, &["A"]);
    let window = [("SONATA_RESUME_WINDOW_MS", "5000")];

    offered(&app, ids[0], ConversationAgent::Claude, ID);
    let out = run_as(&app, ids[0], &bin, &["claude", "--model", "x"], &window);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(stderr.matches(NOTICE).count(), 1, "{stderr}");
    let args = fake_agent_args(&bin, "claude");
    assert_eq!(args.len(), 2, "{args:?}");
    assert!(args[0].starts_with(&format!("--resume {ID} ")));
    assert!(!args[1].contains("--resume") && args[1].contains("--settings"));
    assert!(args[1].ends_with("--model x"), "{args:?}");

    offered(&app, ids[0], ConversationAgent::Codex, "c3");
    let out = run_as(&app, ids[0], &bin, &["codex"], &window);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    assert_eq!(
        String::from_utf8_lossy(&out.stderr).matches(NOTICE).count(),
        1
    );
    let args = fake_agent_args(&bin, "codex");
    assert_eq!(args.len(), 2, "{args:?}");
    assert!(args[0].starts_with("resume c3 ") && !args[1].contains("resume"));
    app.quit();
}

#[test]
fn late_failures_and_successes_are_not_retried() {
    let sb = Sandbox::new();
    let bin = sb.tmp.path().join("bin");
    failing_resume(&bin, "claude", "--resume", "1");
    install_fake_agent(&bin, "codex", 0, 0);
    let (app, _) = messaging_app(&sb);
    let (_, ids) = shells(&app, &sb, &["A"]);
    let window = [("SONATA_RESUME_WINDOW_MS", "500")];

    // Exits with an error after the window: a normal session that ended badly.
    offered(&app, ids[0], ConversationAgent::Claude, ID);
    let out = run_as(&app, ids[0], &bin, &["claude"], &window);
    assert_eq!(out.status.code(), Some(1));
    assert!(!String::from_utf8_lossy(&out.stderr).contains(NOTICE));
    assert_eq!(fake_agent_args(&bin, "claude").len(), 1);

    // Exits cleanly within the window: the user quit.
    offered(&app, ids[0], ConversationAgent::Codex, "c3");
    let out = run_as(&app, ids[0], &bin, &["codex"], &window);
    assert_eq!(out.status.code(), Some(0));
    assert!(!String::from_utf8_lossy(&out.stderr).contains(NOTICE));
    assert_eq!(fake_agent_args(&bin, "codex").len(), 1);
    app.quit();
}

// ---- Kimi Code (feature 010, T053) ----

const KIMI_ID: &str = "session_3f2a0c1e-7b5d-4e2a-9c1f-0a1b2c3d4e5f";

#[test]
fn kimi_resumes_with_session_and_without_the_agent_file() {
    let sb = Sandbox::new();
    let bin = sb.tmp.path().join("bin");
    install_fake_agent(&bin, "kimi", 0, 0);
    let (app, _) = messaging_app(&sb);
    let (_, ids) = shells(&app, &sb, &["A"]);
    offered(&app, ids[0], ConversationAgent::Kimi, KIMI_ID);

    let out = run_as(&app, ids[0], &bin, &["kimi", "--yolo"], &[]);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    let args = fake_agent_args(&bin, "kimi");
    assert_eq!(args, [format!("--session {KIMI_ID} --yolo")]);

    // The offer is used up: the next run starts a new conversation with the agent file.
    run_as(&app, ids[0], &bin, &["kimi", "--yolo"], &[]);
    let w = words(&fake_agent_args(&bin, "kimi")[1]).join(" ");
    assert!(
        w.starts_with("--agent-file ") && w.ends_with("kimi-agent.md --yolo"),
        "{w}"
    );
    app.quit();
}

#[test]
fn kimi_arguments_that_choose_run_unchanged_and_use_up_the_offer() {
    let sb = Sandbox::new();
    let bin = sb.tmp.path().join("bin");
    install_fake_agent(&bin, "kimi", 0, 0);
    let (app, _) = messaging_app(&sb);
    let (_, ids) = shells(&app, &sb, &["A"]);
    let cases: &[&[&str]] = &[
        &["-S", "session_x"],
        &["--session=session_x"],
        &["-r"],
        &["--resume", "session_x"],
        &["-c"],
        &["--continue"],
        &["-C"],
        &["--agent", "okabe"],
        &["--agent-file", "/a.md"],
        &["-p", "hi"],
        &["--prompt", "hi"],
        &["doctor"],
    ];
    for case in cases {
        offered(&app, ids[0], ConversationAgent::Kimi, KIMI_ID);
        let mut args = vec!["kimi"];
        args.extend_from_slice(case);
        run_as(&app, ids[0], &bin, &args, &[]);
        let last = fake_agent_args(&bin, "kimi").pop().unwrap();
        assert_eq!(last, case.join(" "), "{case:?}");
        run_as(&app, ids[0], &bin, &["kimi"], &[]);
        let last = fake_agent_args(&bin, "kimi").pop().unwrap();
        assert!(!last.contains(KIMI_ID), "{case:?}: {last}");
    }
    // A Claude conversation is not offered to Kimi, but Kimi's start uses the offer up.
    offered(&app, ids[0], ConversationAgent::Claude, ID);
    run_as(&app, ids[0], &bin, &["kimi"], &[]);
    let last = fake_agent_args(&bin, "kimi").pop().unwrap();
    assert!(
        last.starts_with("--agent-file ") && !last.contains(ID),
        "{last}"
    );
    install_fake_agent(&bin, "claude", 0, 0);
    run_as(&app, ids[0], &bin, &["claude"], &[]);
    let last = fake_agent_args(&bin, "claude").pop().unwrap();
    assert!(!last.contains("--resume"), "{last}");
    app.quit();
}

#[test]
fn kimi_failed_resume_prints_one_notice_and_starts_with_the_agent_file() {
    let sb = Sandbox::new();
    let bin = sb.tmp.path().join("bin");
    failing_resume(&bin, "kimi", "--session", "0");
    let (app, _) = messaging_app(&sb);
    let (_, ids) = shells(&app, &sb, &["A"]);
    offered(&app, ids[0], ConversationAgent::Kimi, KIMI_ID);

    let window = [("SONATA_RESUME_WINDOW_MS", "500")];
    let out = run_as(&app, ids[0], &bin, &["kimi", "--yolo"], &window);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(stderr.matches(NOTICE).count(), 1, "{stderr}");
    let args = fake_agent_args(&bin, "kimi");
    assert_eq!(args.len(), 2, "{args:?}");
    assert_eq!(args[0], format!("--session {KIMI_ID} --yolo"));
    assert!(
        args[1].starts_with("--agent-file ") && args[1].ends_with(" --yolo"),
        "{args:?}"
    );
    app.quit();
}

#[test]
fn kimi_prompt_mode_turns_conversation_reports_off() {
    let sb = Sandbox::new();
    let bin = sb.tmp.path().join("bin");
    let env_file = bin.join("kimi.env");
    common::fake_bin::fake_bin(&bin, "kimi", &format!("env > '{}'", env_file.display()));
    let (app, _) = messaging_app(&sb);
    let (_, ids) = shells(&app, &sb, &["A"]);
    let off = |args: &[&str]| {
        let _ = std::fs::remove_file(&env_file);
        run_as(&app, ids[0], &bin, args, &[]);
        std::fs::read_to_string(&env_file)
            .unwrap()
            .lines()
            .any(|l| l == "SONATA_CONVERSATION=off")
    };
    assert!(off(&["kimi", "-p", "hi"]));
    assert!(off(&["kimi", "--prompt", "hi"]));
    assert!(off(&["kimi", "--prompt=hi"]));
    assert!(!off(&["kimi"]));
    assert!(!off(&["kimi", "-m", "k2"]));
    app.quit();
}

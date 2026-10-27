//! `sonata run <agent> [args…]`: starts an agent with Sonata's session-only instructions and
//! end-of-turn hooks. Nothing is written to the agent's own configuration (research R4). On the
//! first run after a terminal starts, it resumes the terminal's stored conversation (feature 009).

use std::os::unix::process::{CommandExt, ExitStatusExt};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, ExitStatus};
use std::time::{Duration, Instant};

use crate::client;

/// Finds `name` in `SONATA_AGENT_BIN_DIR` (if set), then on PATH, skipping the folder that holds
/// this `sonata` binary.
fn find_real(name: &str) -> Option<PathBuf> {
    if let Some(dir) = std::env::var_os("SONATA_AGENT_BIN_DIR") {
        let p = PathBuf::from(dir).join(name);
        if p.is_file() {
            return Some(p);
        }
    }
    let own_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(Path::to_path_buf));
    std::env::var_os("PATH").and_then(|paths| {
        std::env::split_paths(&paths)
            .filter(|d| Some(d) != own_dir.as_ref())
            .map(|d| d.join(name))
            .find(|p| p.is_file())
    })
}

/// Claude Code options that already choose a conversation or are not an interactive session.
const CLAUDE_OWN_CHOICE: [&str; 9] = [
    "-r",
    "--resume",
    "-c",
    "--continue",
    "--session-id",
    "--from-pr",
    "--teleport",
    "-p",
    "--print",
];

/// False when the user's arguments already pick a conversation, or start a subcommand or a
/// one-shot run instead of an interactive session (research R3).
fn may_resume(agent: &str, args: &[String]) -> bool {
    if args.first().is_some_and(|a| !a.starts_with('-')) {
        return false;
    }
    match agent {
        "claude" => !args.iter().any(|a| {
            let flag = a.split('=').next().unwrap_or(a);
            CLAUDE_OWN_CHOICE.contains(&flag)
        }),
        "kimi" => !kimi_chooses(args),
        _ => true,
    }
}

/// The same rule as the app's: the id becomes an argument, so only `[A-Za-z0-9_-]{1,128}`.
fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 128
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

/// Arguments that make `agent` resume conversation `id`.
fn resume_args(agent: &str, id: &str) -> Vec<String> {
    match agent {
        "claude" => vec!["--resume".into(), id.into()],
        "kimi" => vec!["--session".into(), id.into()],
        _ => vec!["resume".into(), id.into()],
    }
}

/// Kimi arguments that pick a conversation, an agent, or a non-interactive mode (contracts/cli.md).
const KIMI_CHOOSING_FLAGS: [&str; 11] = [
    "-S",
    "--session",
    "-r",
    "--resume",
    "-c",
    "--continue",
    "-C",
    "--agent",
    "--agent-file",
    "-p",
    "--prompt",
];
const KIMI_SUBCOMMANDS: [&str; 14] = [
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
];

/// True when the user's own Kimi arguments already choose what to run, so Sonata adds nothing.
fn kimi_chooses(args: &[String]) -> bool {
    args.first()
        .is_some_and(|a| KIMI_SUBCOMMANDS.contains(&a.as_str()))
        || args.iter().any(|a| {
            let flag = a.split_once('=').map_or(a.as_str(), |(f, _)| f);
            KIMI_CHOOSING_FLAGS.contains(&flag)
        })
}

pub fn main(args: Vec<String>) -> ExitCode {
    let Some(agent) = args.first().cloned() else {
        eprintln!("sonata: usage: sonata run <claude|codex|kimi> [args…]");
        return ExitCode::from(crate::EXIT_USAGE);
    };
    let rest = &args[1..];
    let Some(real) = find_real(&agent) else {
        eprintln!("sonata: \"{agent}\" was not found on your PATH.");
        return ExitCode::from(127);
    };

    // Ask the app for this session's files. Outside Sonata, just run the agent.
    let mut session: Vec<String> = Vec::new();
    let mut resume: Option<String> = None;
    if let Ok(env) = client::env() {
        let hook = std::env::current_exe()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|_| "sonata".into());
        let req = vec!["_session".to_string(), agent.clone(), hook];
        match client::request(&env, &req, None, |_| {}) {
            Ok(res) if res["ok"].as_bool() == Some(true) => {
                let data = &res["data"];
                match agent.as_str() {
                    "claude" => {
                        if let Some(f) = data["instructionsFile"].as_str() {
                            session.extend(["--append-system-prompt-file".into(), f.into()]);
                        }
                        if let Some(f) = data["settingsFile"].as_str() {
                            session.extend(["--settings".into(), f.into()]);
                        }
                    }
                    "codex" => {
                        if let Some(n) = data["codexNotify"].as_str() {
                            session.extend(["-c".into(), format!("notify={n}")]);
                        }
                    }
                    // Kimi binds the agent file to a new conversation; it cannot be combined
                    // with a resume or a user-chosen agent (feature 010, research R2).
                    "kimi" if !kimi_chooses(rest) => {
                        if let Some(f) = data["agentFile"].as_str() {
                            session.extend(["--agent-file".into(), f.into()]);
                        }
                    }
                    _ => {}
                }
                resume = data["resume"]["id"]
                    .as_str()
                    .filter(|_| data["resume"]["agent"] == agent.as_str())
                    .filter(|id| valid_id(id))
                    .map(str::to_string);
            }
            Ok(res) => eprintln!(
                "sonata: {}",
                res["text"]
                    .as_str()
                    .unwrap_or("could not prepare the session")
            ),
            Err(e) => eprintln!("sonata: {e}"),
        }
    }

    // A one-shot `claude -p` or `kimi -p` must not replace or clear the terminal's
    // conversation (R9; feature 010).
    let print_flags: &[&str] = match agent.as_str() {
        "claude" => &["-p", "--print"],
        "kimi" => &["-p", "--prompt"],
        _ => &[],
    };
    let print_mode = rest.iter().any(|a| {
        let flag = a.split_once('=').map_or(a.as_str(), |(f, _)| f);
        print_flags.contains(&flag)
    });
    // Kimi's agent file only applies to a new conversation, so a resume runs without it.
    let resume_session: &[String] = if agent == "kimi" { &[] } else { &session };
    let command = |lead: &[String], session: &[String]| {
        let mut cmd = Command::new(&real);
        cmd.arg0(&agent).args(lead).args(session).args(rest);
        if print_mode {
            cmd.env("SONATA_CONVERSATION", "off");
        }
        cmd
    };
    if let Some(id) = resume.filter(|_| may_resume(&agent, rest)) {
        // An error exit soon after start means the conversation could not be resumed (files
        // gone, agent updated, folder changed): say so once and start a new one (research R4).
        // The new conversation's start clears the stored one, so it is never tried again.
        let started = Instant::now();
        match run_child(command(&resume_args(&agent, &id), resume_session)) {
            Ok(status) if status.code().is_some_and(|c| c != 0) && started.elapsed() < window() => {
                eprintln!(
                    "sonata: the previous conversation could not be resumed; starting a new one."
                );
            }
            Ok(status) => return ExitCode::from(exit_code(status)),
            Err(err) => {
                eprintln!("sonata: could not start {agent}: {err}");
                return ExitCode::from(126);
            }
        }
    }
    let err = command(&[], &session).exec();
    eprintln!("sonata: could not start {agent}: {err}");
    ExitCode::from(126)
}

/// How long after start an error exit counts as a failed resume. `SONATA_RESUME_WINDOW_MS`
/// shortens it for tests.
fn window() -> Duration {
    std::env::var("SONATA_RESUME_WINDOW_MS")
        .ok()
        .and_then(|v| v.parse().ok())
        .map(Duration::from_millis)
        .unwrap_or(RESUME_WINDOW)
}

const RESUME_WINDOW: Duration = Duration::from_secs(10);

/// The wrapper's exit code for the agent's status: its code, or 128 + signal when killed.
fn exit_code(status: ExitStatus) -> u8 {
    match (status.code(), status.signal()) {
        (Some(code), _) => code as u8,
        (None, Some(sig)) => 128u8.saturating_add(sig as u8),
        _ => 1,
    }
}

/// Runs the agent as a child and waits. Like `system(3)`, the wrapper ignores Ctrl+C and
/// Ctrl+\ while it waits, so only the agent gets them (research R4).
fn run_child(mut cmd: Command) -> std::io::Result<ExitStatus> {
    // SAFETY: only async-signal-safe `signal` calls between fork and exec.
    unsafe {
        cmd.pre_exec(|| {
            libc::signal(libc::SIGINT, libc::SIG_DFL);
            libc::signal(libc::SIGQUIT, libc::SIG_DFL);
            Ok(())
        });
    }
    // SAFETY: `signal` with SIG_IGN and a saved previous handler; restored below.
    let (int, quit) = unsafe {
        (
            libc::signal(libc::SIGINT, libc::SIG_IGN),
            libc::signal(libc::SIGQUIT, libc::SIG_IGN),
        )
    };
    let status = cmd.spawn().and_then(|mut child| child.wait());
    // SAFETY: restores the handlers saved above.
    unsafe {
        libc::signal(libc::SIGINT, int);
        libc::signal(libc::SIGQUIT, quit);
    }
    status
}

#[cfg(test)]
mod tests {
    use super::{kimi_chooses, may_resume, valid_id};

    fn args(a: &[&str]) -> Vec<String> {
        a.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn resume_only_for_plain_interactive_sessions() {
        assert!(may_resume("claude", &args(&[])));
        assert!(may_resume("claude", &args(&["--model", "x"])));
        assert!(!may_resume("claude", &args(&["--resume=abc"])));
        assert!(!may_resume("claude", &args(&["mcp", "list"])));
        assert!(may_resume("codex", &args(&["-c", "model=o"])));
        assert!(!may_resume("codex", &args(&["resume"])));
    }

    #[test]
    fn ids_are_checked_before_they_become_arguments() {
        assert!(valid_id("7294d08d-ac3c-4e7e-925e-16750c04c269"));
        assert!(!valid_id(""));
        assert!(!valid_id("a b"));
        assert!(!valid_id(&"x".repeat(129)));
    }

    #[test]
    fn kimi_choice_detection() {
        assert!(!kimi_chooses(&args(&[])));
        assert!(!kimi_chooses(&args(&["--yolo", "-m", "k2"])));
        assert!(kimi_chooses(&args(&["--session=session_x"])));
        assert!(kimi_chooses(&args(&["--yolo", "-c"])));
        assert!(kimi_chooses(&args(&["doctor"])));
        // A subcommand name later in the arguments is an argument value, not a subcommand.
        assert!(!kimi_chooses(&args(&["-m", "doctor"])));
    }
}

//! `sonata run <agent> [args…]`: starts an agent with Sonata's session-only instructions and
//! end-of-turn hooks. Nothing is written to the agent's own configuration (research R4). On the
//! first run after a terminal starts, it resumes the terminal's stored conversation (feature 009).

use std::os::unix::process::{CommandExt, ExitStatusExt};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

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
        _ => vec!["resume".into(), id.into()],
    }
}

pub fn main(args: Vec<String>) -> ExitCode {
    let Some(agent) = args.first().cloned() else {
        eprintln!("sonata: usage: sonata run <claude|codex> [args…]");
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

    // A one-shot `claude -p` must not replace or clear the terminal's conversation (R9).
    let print_mode = agent == "claude" && rest.iter().any(|a| a == "-p" || a == "--print");
    let command = |lead: &[String]| {
        let mut cmd = Command::new(&real);
        cmd.arg0(&agent).args(lead).args(&session).args(rest);
        if print_mode {
            cmd.env("SONATA_CONVERSATION", "off");
        }
        cmd
    };
    if let Some(id) = resume.filter(|_| may_resume(&agent, rest)) {
        return match run_child(command(&resume_args(&agent, &id))) {
            Ok(code) => ExitCode::from(code),
            Err(err) => {
                eprintln!("sonata: could not start {agent}: {err}");
                ExitCode::from(126)
            }
        };
    }
    let err = command(&[]).exec();
    eprintln!("sonata: could not start {agent}: {err}");
    ExitCode::from(126)
}

/// Runs the agent as a child and returns its exit code (128 + signal when killed). Like
/// `system(3)`, the wrapper ignores Ctrl+C and Ctrl+\ while it waits, so only the agent gets
/// them (research R4).
fn run_child(mut cmd: Command) -> std::io::Result<u8> {
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
    let status = status?;
    Ok(match (status.code(), status.signal()) {
        (Some(code), _) => code as u8,
        (None, Some(sig)) => 128u8.saturating_add(sig as u8),
        _ => 1,
    })
}

#[cfg(test)]
mod tests {
    use super::{may_resume, valid_id};

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
}

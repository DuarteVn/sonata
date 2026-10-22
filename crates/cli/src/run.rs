//! `sonata run <agent> [args…]`: starts an agent with Sonata's session-only instructions and
//! end-of-turn hooks. Nothing is written to the agent's own configuration (research R4).

use std::os::unix::process::CommandExt;
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
    let mut cmd = Command::new(&real);
    cmd.arg0(&agent);

    // Ask the app for this session's files. Outside Sonata, just run the agent.
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
                            cmd.arg("--append-system-prompt-file").arg(f);
                        }
                        if let Some(f) = data["settingsFile"].as_str() {
                            cmd.arg("--settings").arg(f);
                        }
                    }
                    "codex" => {
                        if let Some(n) = data["codexNotify"].as_str() {
                            cmd.arg("-c").arg(format!("notify={n}"));
                        }
                    }
                    _ => {}
                }
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
    cmd.args(rest);
    let err = cmd.exec();
    eprintln!("sonata: could not start {agent}: {err}");
    ExitCode::from(126)
}

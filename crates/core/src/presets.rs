//! Quick-start presets for the New Terminal dialog and detection of their programs through the
//! user's login shell (research R4).

use std::io::Read;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Preset {
    pub id: &'static str,
    pub name: &'static str,
    pub command: Option<&'static str>,
}

pub const PRESETS: [Preset; 5] = [
    Preset {
        id: "claude",
        name: "Claude Code",
        command: Some("claude"),
    },
    Preset {
        id: "codex",
        name: "Codex",
        command: Some("codex"),
    },
    Preset {
        id: "opencode",
        name: "OpenCode",
        command: Some("opencode"),
    },
    Preset {
        id: "gemini",
        name: "Gemini CLI",
        command: Some("gemini"),
    },
    Preset {
        id: "shell",
        name: "Shell",
        command: None,
    },
];

pub const DETECT_TIMEOUT: Duration = Duration::from_secs(2);

/// `found`: `Some(true|false)`, or `None` when detection did not finish in time.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PresetStatus {
    pub id: &'static str,
    pub found: Option<bool>,
}

/// Asks `shell -l -c` which preset programs are on its `PATH`.
pub fn detect(shell: &Path, timeout: Duration) -> Vec<PresetStatus> {
    let names: Vec<&str> = PRESETS.iter().filter_map(|p| p.command).collect();
    let script = format!(
        "for p in {}; do command -v \"$p\"; done; exit 0",
        names.join(" ")
    );
    let output = run_with_timeout(shell, &script, timeout);
    PRESETS
        .iter()
        .map(|p| PresetStatus {
            id: p.id,
            found: match (p.command, &output) {
                (None, _) => Some(true),
                (Some(_), None) => None,
                (Some(cmd), Some(out)) => Some(out.lines().any(|line| {
                    let line = line.trim();
                    line == cmd || Path::new(line).file_name().is_some_and(|f| f == cmd)
                })),
            },
        })
        .collect()
}

fn run_with_timeout(shell: &Path, script: &str, timeout: Duration) -> Option<String> {
    let mut cmd = Command::new(shell);
    crate::platform::child_env().apply_std(&mut cmd);
    let mut child = cmd
        .args(["-l", "-c", script])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    let start = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if start.elapsed() < timeout => std::thread::sleep(Duration::from_millis(20)),
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
        }
    }
    let mut out = String::new();
    child.stdout.take()?.read_to_string(&mut out).ok()?;
    Some(out)
}

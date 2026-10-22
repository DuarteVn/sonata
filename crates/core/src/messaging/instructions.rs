//! Session-only instructions and hook settings for agents started with `sonata run`
//! (research 003 R4, R6). Written under `<data root>/run/<terminal id>/`, removed on stop.

use std::path::{Path, PathBuf};

use serde_json::json;

use crate::paths::ensure_private_dir;
use crate::persistence::atomic;

pub struct SessionInfo<'a> {
    pub name: &'a str,
    pub workspace: &'a str,
    pub role: Option<&'a str>,
    pub orchestrator: bool,
}

pub fn instructions(info: &SessionInfo<'_>) -> String {
    let mut s = format!(
        "# You are running inside Sonata\n\n\
         Sonata is a canvas where several AI agents and notes run side by side. You are the \
         terminal \"{}\" in the workspace \"{}\". Lines drawn on the canvas link you to other \
         terminals and notes; you can only reach what you are linked to.\n\n",
        info.name, info.workspace
    );
    if let Some(role) = info.role {
        s.push_str(&format!("## Your role\n\n{role}\n\n"));
    }
    s.push_str(
        "## The `sonata` command\n\n\
         Run these in your shell tool:\n\n\
         - `sonata list` — who you are and what you are linked to.\n\
         - `sonata ask <name> \"<message>\"` — send a message to a linked terminal and wait for \
         its answer. `sonata ask --all \"…\"` asks every linked terminal at once. Use \
         `--timeout 30m` for long tasks.\n\
         - `sonata check <name>` — see a linked terminal's screen without sending anything.\n\
         - `sonata note read <note>` / `sonata note write <note> \"<text>\" [--append]` — shared \
         notes.\n\
         - `sonata notify \"<message>\"` — tell the user something important (macOS \
         notification).\n\n\
         When another agent asks you something through Sonata, the message arrives as a normal \
         prompt. Answer normally; your final message is returned to them. For long or structured \
         answers, run `sonata reply --stdin` with the full answer.\n",
    );
    if info.orchestrator {
        s.push_str(
            "\n## You are the Orchestrator\n\n\
             You can build your team:\n\n\
             - `sonata recruit <claude|codex|opencode|gemini|shell> --name <Name> [--role-text \
             \"<what they do>\"] [--folder <path>]` — start a new agent linked to you.\n\
             - `sonata dismiss <name>` — close an agent you recruited.\n\
             - `sonata link <a> <b>` / `sonata unlink <a> <b>` — connect or disconnect items in \
             this workspace.\n",
        );
    }
    s
}

/// Shorter text typed into agents that cannot receive instructions at start.
pub fn typed_instructions(info: &SessionInfo<'_>) -> String {
    let mut s = format!(
        "You are running inside Sonata as the terminal \"{}\". Run `sonata help` to see how to \
         talk to the terminals and notes linked to you (`sonata list`, `sonata ask`).",
        info.name
    );
    if let Some(role) = info.role {
        s.push_str(&format!(" Your role: {role}"));
    }
    s
}

pub struct SessionFiles {
    pub dir: PathBuf,
    pub instructions: PathBuf,
    pub settings: PathBuf,
}

/// Writes the files for one agent session. `hook` is the absolute path of the `sonata` binary.
pub fn write_session(
    run_dir: &Path,
    info: &SessionInfo<'_>,
    hook: &str,
) -> std::io::Result<SessionFiles> {
    ensure_private_dir(run_dir)?;
    let instructions_path = run_dir.join("instructions.md");
    atomic::write(&instructions_path, instructions(info).as_bytes())?;
    let quoted = format!("'{}'", hook.replace('\'', "'\\''"));
    let settings = json!({
        "hooks": {
            "Stop": [{"hooks": [{"type": "command", "command": format!("{quoted} _hook claude-stop")}]}],
            "UserPromptSubmit": [{"hooks": [{"type": "command", "command": format!("{quoted} _hook turn-start")}]}]
        }
    });
    let settings_path = run_dir.join("settings.json");
    atomic::write(
        &settings_path,
        serde_json::to_vec_pretty(&settings)
            .unwrap_or_default()
            .as_slice(),
    )?;
    Ok(SessionFiles {
        dir: run_dir.to_path_buf(),
        instructions: instructions_path,
        settings: settings_path,
    })
}

/// TOML value for Codex's `-c notify=…` session override.
pub fn codex_notify(hook: &str) -> String {
    let esc = hook.replace('\\', "\\\\").replace('"', "\\\"");
    format!("[\"{esc}\", \"_hook\", \"codex-notify\"]")
}

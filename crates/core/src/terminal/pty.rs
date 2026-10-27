//! Spawning shells in pseudo-terminals (research R5).

use std::ffi::CStr;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

use portable_pty::{Child, ChildKiller, CommandBuilder, MasterPty, PtySize, native_pty_system};
use uuid::Uuid;

/// The user's login shell from the password database, then `$SHELL`, then `/bin/zsh`.
pub fn login_shell() -> PathBuf {
    // SAFETY: getpwuid returns a pointer into static storage or null; we copy the string
    // immediately and do not keep the pointer.
    let from_passwd = unsafe {
        let pw = libc::getpwuid(libc::getuid());
        if pw.is_null() || (*pw).pw_shell.is_null() {
            None
        } else {
            CStr::from_ptr((*pw).pw_shell)
                .to_str()
                .ok()
                .map(PathBuf::from)
        }
    };
    from_passwd
        .filter(|p| p.is_absolute() && p.exists())
        .or_else(|| {
            std::env::var_os("SHELL")
                .map(PathBuf::from)
                .filter(|p| p.exists())
        })
        .unwrap_or_else(|| PathBuf::from("/bin/zsh"))
}

pub struct Spawned {
    pub master: Box<dyn MasterPty + Send>,
    pub reader: Box<dyn Read + Send>,
    pub writer: Box<dyn Write + Send>,
    pub child: Box<dyn Child + Send + Sync>,
    pub killer: Box<dyn ChildKiller + Send + Sync>,
    pub pid: Option<u32>,
    /// Set when the requested folder was missing and the home folder was used instead.
    pub warning: Option<String>,
}

pub struct SpawnOptions<'a> {
    pub node_id: Uuid,
    pub cwd: &'a Path,
    pub shell: Option<&'a Path>,
    pub launch_command: Option<&'a str>,
    pub rows: u16,
    pub cols: u16,
    /// Extra environment (Sonata session identity, socket, CLI folder on PATH).
    pub env: &'a [(String, String)],
    /// Start Claude Code, Codex, and Kimi through `sonata run` (feature 003, research R4).
    pub wrap_agents: bool,
    /// Sonata's zsh startup files (see `terminal::zsh`); used when the shell is zsh.
    pub zsh_dir: Option<&'a Path>,
}

/// Agents that `sonata run` knows how to start with session instructions and hooks.
pub const WRAPPED_AGENTS: [&str; 3] = ["claude", "codex", "kimi"];

/// Environment variables that mark a process as running inside an agent session (research R5).
/// Terminals and the usage probe drop them, so an agent started there is a fresh, top-level
/// session and an outer agent's messaging secret never leaks. User settings such as
/// `CLAUDE_CODE_USE_BEDROCK` or `ANTHROPIC_*` are not on the list.
pub const AGENT_SESSION_MARKERS: [&str; 14] = [
    "CLAUDECODE",
    "CLAUDE_CODE_ENTRYPOINT",
    "CLAUDE_CODE_EXECPATH",
    "CLAUDE_CODE_SESSION_ID",
    "CLAUDE_CODE_CHILD_SESSION",
    "CLAUDE_CODE_SESSION_ATTENDED",
    "CLAUDE_CODE_MESSAGING_SOCKET",
    "CLAUDE_CODE_MESSAGING_TOKEN",
    "CLAUDE_PID",
    "CLAUDE_EFFORT",
    "CODEX_SANDBOX",
    "CODEX_SANDBOX_NETWORK_DISABLED",
    // Kimi Code (feature 010, research R8).
    "KIMI_PLUGIN_ROOT",
    "KIMI_CODE_UPDATE_REEXEC",
];

/// `claude --foo` becomes `sonata run claude --foo` when wrapping is on.
pub fn wrap_launch_command(command: &str, wrap: bool) -> String {
    let first = command.split_whitespace().next().unwrap_or_default();
    if wrap && WRAPPED_AGENTS.contains(&first) {
        format!("sonata run {command}")
    } else {
        command.to_string()
    }
}

pub fn spawn(opts: SpawnOptions<'_>) -> io::Result<Spawned> {
    let pty = native_pty_system()
        .openpty(PtySize {
            rows: opts.rows.max(1),
            cols: opts.cols.max(1),
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(io::Error::other)?;

    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/"));
    let (cwd, warning) = if opts.cwd.is_dir() {
        (opts.cwd.to_path_buf(), None)
    } else {
        (
            home.clone(),
            Some(format!(
                "The folder {} does not exist; the terminal started in your home folder.",
                opts.cwd.display()
            )),
        )
    };

    let shell = opts
        .shell
        .map(Path::to_path_buf)
        .unwrap_or_else(login_shell);
    let mut cmd = CommandBuilder::new(&shell);
    cmd.arg("-l");
    cmd.cwd(&cwd);
    cmd.env("TERM", "xterm-256color");
    cmd.env("COLORTERM", "truecolor");
    cmd.env("TERM_PROGRAM", "Sonata");
    cmd.env("TERM_PROGRAM_VERSION", env!("CARGO_PKG_VERSION"));
    cmd.env("SONATA_TERMINAL_ID", opts.node_id.to_string());
    cmd.env("PWD", &cwd);
    cmd.env_remove("SONATA_HOME");
    for key in AGENT_SESSION_MARKERS {
        cmd.env_remove(key);
    }
    for (k, v) in opts.env {
        cmd.env(k, v);
    }
    if let Some(dir) = opts.zsh_dir
        && shell.file_name().is_some_and(|n| n == "zsh")
    {
        let user = std::env::var_os("ZDOTDIR").unwrap_or_else(|| home.clone().into_os_string());
        cmd.env("SONATA_USER_ZDOTDIR", user);
        cmd.env("ZDOTDIR", dir);
    }

    let child = pty.slave.spawn_command(cmd).map_err(io::Error::other)?;
    drop(pty.slave);
    let pid = child.process_id();
    let killer = child.clone_killer();
    let reader = pty.master.try_clone_reader().map_err(io::Error::other)?;
    // The launch command is typed by the terminal manager once the shell shows its prompt:
    // input typed earlier goes through the TTY's line buffer, which cuts lines at 1024 bytes.
    let writer = pty.master.take_writer().map_err(io::Error::other)?;
    Ok(Spawned {
        master: pty.master,
        reader,
        writer,
        child,
        killer,
        pid,
        warning,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kimi_is_wrapped() {
        assert!(WRAPPED_AGENTS.contains(&"kimi"));
        assert_eq!(
            wrap_launch_command("kimi --yolo", true),
            "sonata run kimi --yolo"
        );
        assert_eq!(wrap_launch_command("kimi", false), "kimi");
    }
}

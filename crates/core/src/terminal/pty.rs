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

    let child = pty.slave.spawn_command(cmd).map_err(io::Error::other)?;
    drop(pty.slave);
    let pid = child.process_id();
    let killer = child.clone_killer();
    let reader = pty.master.try_clone_reader().map_err(io::Error::other)?;
    let mut writer = pty.master.take_writer().map_err(io::Error::other)?;
    if let Some(command) = opts.launch_command.map(str::trim).filter(|c| !c.is_empty()) {
        // The TTY buffers this until the shell reads input, so it runs as the first command.
        writer.write_all(format!("{command}\r").as_bytes())?;
        writer.flush()?;
    }
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

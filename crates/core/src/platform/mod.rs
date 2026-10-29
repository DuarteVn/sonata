//! What differs between operating systems (feature 011, research R1). Shared rules live here;
//! each OS has its own file, chosen at build time, with the same functions.

use std::ffi::OsString;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;

#[cfg(target_os = "linux")]
use linux as os;
#[cfg(target_os = "macos")]
use macos as os;

/// Variables the AppImage runtime sets for itself only (research R6).
const APPIMAGE_ONLY: [&str; 4] = ["APPIMAGE", "APPDIR", "ARGV0", "OWD"];

/// Environment changes for the programs Sonata starts for the user.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EnvFix {
    pub remove: Vec<String>,
    pub set: Vec<(String, String)>,
}

impl EnvFix {
    pub fn is_empty(&self) -> bool {
        self.remove.is_empty() && self.set.is_empty()
    }

    pub fn apply_std(&self, cmd: &mut std::process::Command) {
        for key in &self.remove {
            cmd.env_remove(key);
        }
        for (key, value) in &self.set {
            cmd.env(key, value);
        }
    }

    pub fn apply_pty(&self, cmd: &mut portable_pty::CommandBuilder) {
        for key in &self.remove {
            cmd.env_remove(key);
        }
        for (key, value) in &self.set {
            cmd.env(key, value);
        }
    }
}

/// Inside an AppImage (`APPIMAGE` and `APPDIR` set), removes the runtime's own variables and the
/// `APPDIR` entries of every colon-separated list, so system programs see the system's paths.
pub fn host_env(vars: impl IntoIterator<Item = (String, String)>) -> EnvFix {
    let vars: Vec<(String, String)> = vars.into_iter().collect();
    let get = |name: &str| {
        vars.iter()
            .find(|(k, v)| k == name && !v.is_empty())
            .map(|(_, v)| v.as_str())
    };
    let (Some(_), Some(appdir)) = (get("APPIMAGE"), get("APPDIR")) else {
        return EnvFix::default();
    };
    let appdir = appdir.trim_end_matches('/');
    let inside = |entry: &str| entry == appdir || entry.starts_with(&format!("{appdir}/"));
    let mut fix = EnvFix::default();
    for (key, value) in &vars {
        if APPIMAGE_ONLY.contains(&key.as_str()) {
            fix.remove.push(key.clone());
        } else if value.split(':').any(inside) {
            let kept: Vec<&str> = value
                .split(':')
                .filter(|e| !e.is_empty() && !inside(e))
                .collect();
            if kept.is_empty() {
                fix.remove.push(key.clone());
            } else {
                fix.set.push((key.clone(), kept.join(":")));
            }
        }
    }
    fix
}

/// [`host_env`] of this process, computed once.
pub fn child_env() -> &'static EnvFix {
    static FIX: OnceLock<EnvFix> = OnceLock::new();
    FIX.get_or_init(|| {
        host_env(
            std::env::vars_os()
                .filter_map(|(k, v)| Some((k.into_string().ok()?, v.into_string().ok()?))),
        )
    })
}

/// A variable of this process as programs started for the user see it (after [`child_env`]).
pub fn host_var(key: &str) -> Option<String> {
    let fix = child_env();
    if fix.remove.iter().any(|k| k == key) {
        return None;
    }
    match fix.set.iter().find(|(k, _)| k == key) {
        Some((_, v)) => Some(v.clone()),
        None => std::env::var(key).ok(),
    }
}

/// A program and its arguments.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Launch {
    pub program: PathBuf,
    pub args: Vec<OsString>,
}

impl Launch {
    /// Starts the program without waiting, with [`child_env`] applied.
    pub fn spawn(&self) -> io::Result<()> {
        let mut cmd = std::process::Command::new(&self.program);
        cmd.args(&self.args);
        child_env().apply_std(&mut cmd);
        cmd.spawn().map(drop)
    }
}

/// Opens a web address in the default browser.
pub fn open_url_command(url: &str) -> Launch {
    os::open_url_command(url)
}

/// Shows a file in the system file manager.
pub fn reveal_command(path: &Path) -> Launch {
    os::reveal_command(path)
}

/// The message shown when no usable git is found.
pub fn git_install_hint() -> &'static str {
    os::GIT_INSTALL_HINT
}

/// Where to look for git after the login shell's `command -v git`.
pub fn git_candidates() -> &'static [&'static str] {
    os::GIT_CANDIDATES
}

/// A git that may open an install prompt instead of running (macOS `/usr/bin/git`), and whether
/// it is safe to use now. It is tried only after every other candidate.
pub fn git_shim() -> Option<(&'static Path, bool)> {
    os::git_shim()
}

/// What the user's computer is called in messages ("this Mac has git 2.30").
pub fn machine_name() -> &'static str {
    os::MACHINE_NAME
}

/// The shell used when neither the user database nor `$SHELL` names one that exists.
pub fn default_shell() -> PathBuf {
    PathBuf::from(os::DEFAULT_SHELL)
}

/// A string every Sonata CLI binary contains (the variable it reads its terminal from), used to
/// tell a Sonata CLI from another program with the same name.
pub const CLI_MARKER: &str = "SONATA_TERMINAL_ID";

/// Copies the bundled CLI to `target` (for example `~/.local/bin/sonata`, research R7) when the
/// copy is missing or is an older Sonata CLI. Returns whether it wrote. A different program at
/// `target` is never replaced.
pub fn install_cli(bundled: &Path, target: &Path) -> io::Result<bool> {
    use std::io::Write;
    use std::os::unix::fs::PermissionsExt;

    let new = std::fs::read(bundled)?;
    match std::fs::read(target) {
        Ok(old) if old == new => return Ok(false),
        Ok(old) if !contains(&old, CLI_MARKER.as_bytes()) => return Ok(false),
        Ok(_) => {}
        Err(e) if e.kind() == io::ErrorKind::NotFound => {}
        Err(e) => return Err(e),
    }
    if let Some(dir) = target.parent() {
        std::fs::create_dir_all(dir)?;
    }
    crate::persistence::atomic::write_with(target, |file| {
        file.write_all(&new)?;
        file.set_permissions(std::fs::Permissions::from_mode(0o755))
    })?;
    Ok(true)
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack.windows(needle.len()).any(|w| w == needle)
}

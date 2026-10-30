//! Shells a Windows terminal can start (feature 012, research R7): PowerShell 7, Windows
//! PowerShell, Command Prompt, Git Bash, and WSL distributions. Discovery reads the file system
//! and runs only `wsl.exe --list`; other systems use the login shell and get an empty list.

use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::model::node::{ShellKind, TerminalShell};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShellChoice {
    pub kind: ShellKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub distro: Option<String>,
    pub label: String,
    pub path: PathBuf,
    /// The shell used when a terminal has no choice.
    pub default: bool,
}

impl ShellChoice {
    pub fn matches(&self, want: &TerminalShell) -> bool {
        self.kind == want.kind && self.distro == want.distro
    }
}

/// What discovery reads; tests build their own.
#[derive(Debug, Clone, Default)]
pub struct ShellEnv {
    /// `PATH` entries.
    pub path: Vec<PathBuf>,
    pub program_files: Option<PathBuf>,
    pub local_app_data: Option<PathBuf>,
    pub system_root: Option<PathBuf>,
    pub comspec: Option<PathBuf>,
}

impl ShellEnv {
    pub fn current() -> Self {
        let var = |k: &str| {
            std::env::var_os(k)
                .filter(|v| !v.is_empty())
                .map(PathBuf::from)
        };
        Self {
            path: std::env::var_os("PATH")
                .map(|p| std::env::split_paths(&p).collect())
                .unwrap_or_default(),
            program_files: var("ProgramFiles"),
            local_app_data: var("LOCALAPPDATA"),
            system_root: var("SystemRoot"),
            comspec: var("ComSpec"),
        }
    }

    fn on_path(&self, file: &str) -> Option<PathBuf> {
        self.path
            .iter()
            .filter(|d| d.is_absolute())
            .map(|d| d.join(file))
            .find(|p| p.is_file())
    }
}

/// Finds the installed shells, the default (first PowerShell) marked. `wsl` lists WSL
/// distributions for a `wsl.exe` path.
pub fn discover_in(env: &ShellEnv, wsl: impl Fn(&Path) -> Vec<String>) -> Vec<ShellChoice> {
    let mut out = Vec::new();
    let mut add = |kind, distro: Option<String>, label: String, path: Option<PathBuf>| {
        if let Some(path) = path.filter(|p| p.is_file()) {
            out.push(ShellChoice {
                kind,
                distro,
                label,
                path,
                default: false,
            });
        }
    };
    let pwsh = env.on_path("pwsh.exe").or_else(|| {
        env.program_files
            .as_ref()
            .map(|p| p.join("PowerShell").join("7").join("pwsh.exe"))
    });
    add(
        ShellKind::Pwsh,
        None,
        kind_label(ShellKind::Pwsh).into(),
        pwsh,
    );
    let powershell = env.system_root.as_ref().map(|r| {
        r.join("System32")
            .join("WindowsPowerShell")
            .join("v1.0")
            .join("powershell.exe")
    });
    add(
        ShellKind::Powershell,
        None,
        "Windows PowerShell".into(),
        powershell,
    );
    let cmd = env.comspec.clone().or_else(|| {
        env.system_root
            .as_ref()
            .map(|r| r.join("System32").join("cmd.exe"))
    });
    add(ShellKind::Cmd, None, "Command Prompt".into(), cmd);
    add(ShellKind::GitBash, None, "Git Bash".into(), git_bash(env));
    if let Some(exe) = env
        .system_root
        .as_ref()
        .map(|r| r.join("System32").join("wsl.exe"))
        .filter(|p| p.is_file())
    {
        for name in wsl(&exe) {
            add(
                ShellKind::Wsl,
                Some(name.clone()),
                format!("{name} (WSL)"),
                Some(exe.clone()),
            );
        }
    }
    if let Some(first) = out
        .iter_mut()
        .find(|c| matches!(c.kind, ShellKind::Pwsh | ShellKind::Powershell))
    {
        first.default = true;
    } else if let Some(first) = out.first_mut() {
        first.default = true;
    }
    out
}

/// `bin\bash.exe` of Git for Windows: next to the `git.exe` on `PATH` (`<git>\cmd\git.exe`),
/// then the default install folders.
fn git_bash(env: &ShellEnv) -> Option<PathBuf> {
    let beside_git = env.on_path("git.exe").and_then(|git| {
        let root = git.parent()?.parent()?;
        Some(root.join("bin").join("bash.exe"))
    });
    [
        beside_git,
        env.program_files
            .as_ref()
            .map(|p| p.join("Git").join("bin").join("bash.exe")),
        env.local_app_data
            .as_ref()
            .map(|p| p.join("Programs").join("Git").join("bin").join("bash.exe")),
    ]
    .into_iter()
    .flatten()
    .find(|p| p.is_file())
}

/// Parses `wsl.exe --list --quiet`, which prints UTF-16LE (sometimes with a BOM) and may end
/// lines with `\r\n`.
pub fn parse_wsl_list(bytes: &[u8]) -> Vec<String> {
    let units: Vec<u16> = bytes
        .as_chunks::<2>()
        .0
        .iter()
        .map(|c| u16::from_le_bytes(*c))
        .collect();
    String::from_utf16_lossy(&units)
        .trim_start_matches('\u{feff}')
        .lines()
        .map(|l| l.trim().trim_matches('\0').to_string())
        .filter(|l| !l.is_empty())
        .collect()
}

/// Runs `wsl.exe --list --quiet`; no distributions (or no WSL) gives an empty list.
pub fn list_wsl(exe: &Path) -> Vec<String> {
    let mut cmd = std::process::Command::new(exe);
    cmd.args(["--list", "--quiet"])
        .stdin(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    crate::platform::no_window(&mut cmd);
    match cmd.output() {
        Ok(out) if out.status.success() => parse_wsl_list(&out.stdout),
        _ => Vec::new(),
    }
}

/// The installed shells on Windows; empty elsewhere (the login shell is used).
pub fn discover() -> Vec<ShellChoice> {
    if cfg!(windows) {
        discover_in(&ShellEnv::current(), list_wsl)
    } else {
        Vec::new()
    }
}

/// UI name of a shell kind.
pub fn kind_label(kind: ShellKind) -> &'static str {
    match kind {
        ShellKind::Pwsh => "PowerShell 7",
        ShellKind::Powershell => "Windows PowerShell",
        ShellKind::Cmd => "Command Prompt",
        ShellKind::GitBash => "Git Bash",
        ShellKind::Wsl => "WSL",
    }
}

/// The kind of shell an executable is, from its file name.
pub fn kind_of(path: &Path) -> Option<ShellKind> {
    let name = path.file_name()?.to_str()?.to_ascii_lowercase();
    match name.as_str() {
        "pwsh.exe" | "pwsh" => Some(ShellKind::Pwsh),
        "powershell.exe" => Some(ShellKind::Powershell),
        "cmd.exe" => Some(ShellKind::Cmd),
        "bash.exe" => Some(ShellKind::GitBash),
        "wsl.exe" => Some(ShellKind::Wsl),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn utf16(s: &str, bom: bool) -> Vec<u8> {
        let mut out = Vec::new();
        if bom {
            out.extend_from_slice(&0xfeffu16.to_le_bytes());
        }
        for u in s.encode_utf16() {
            out.extend_from_slice(&u.to_le_bytes());
        }
        out
    }

    #[test]
    fn wsl_list_parses_utf16_lines() {
        assert_eq!(
            parse_wsl_list(&utf16("Ubuntu\r\nDebian\r\n\r\n", false)),
            vec!["Ubuntu", "Debian"]
        );
        assert_eq!(
            parse_wsl_list(&utf16("Ubuntu-22.04\n", true)),
            vec!["Ubuntu-22.04"]
        );
        assert!(parse_wsl_list(&[]).is_empty());
    }

    fn touch(p: &Path) {
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, b"").unwrap();
    }

    #[test]
    fn discovery_orders_and_marks_the_default() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let sys = root.join("Windows");
        touch(
            &sys.join("System32")
                .join("WindowsPowerShell")
                .join("v1.0")
                .join("powershell.exe"),
        );
        touch(&sys.join("System32").join("cmd.exe"));
        touch(&sys.join("System32").join("wsl.exe"));
        touch(&root.join("bin7").join("pwsh.exe"));
        touch(&root.join("Git").join("cmd").join("git.exe"));
        touch(&root.join("Git").join("bin").join("bash.exe"));
        let env = ShellEnv {
            path: vec![root.join("bin7"), root.join("Git").join("cmd")],
            program_files: Some(root.join("PF")),
            local_app_data: None,
            system_root: Some(sys.clone()),
            comspec: None,
        };
        let found = discover_in(&env, |_| vec!["Ubuntu".into()]);
        let kinds: Vec<_> = found.iter().map(|c| c.kind).collect();
        assert_eq!(
            kinds,
            vec![
                ShellKind::Pwsh,
                ShellKind::Powershell,
                ShellKind::Cmd,
                ShellKind::GitBash,
                ShellKind::Wsl
            ]
        );
        assert!(found[0].default);
        assert_eq!(found.iter().filter(|c| c.default).count(), 1);
        assert_eq!(found[4].distro.as_deref(), Some("Ubuntu"));
        assert_eq!(found[3].path, root.join("Git").join("bin").join("bash.exe"));
    }

    #[test]
    fn windows_powershell_is_default_without_pwsh() {
        let dir = tempfile::tempdir().unwrap();
        let sys = dir.path().join("Windows");
        touch(
            &sys.join("System32")
                .join("WindowsPowerShell")
                .join("v1.0")
                .join("powershell.exe"),
        );
        let env = ShellEnv {
            system_root: Some(sys),
            ..Default::default()
        };
        let found = discover_in(&env, |_| Vec::new());
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].kind, ShellKind::Powershell);
        assert!(found[0].default);
    }

    #[test]
    fn kind_from_file_name() {
        assert_eq!(
            kind_of(&Path::new("x").join("PWSH.EXE")),
            Some(ShellKind::Pwsh)
        );
        assert_eq!(kind_of(Path::new("/bin/zsh")), None);
    }
}

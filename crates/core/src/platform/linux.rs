//! Linux behavior (feature 011).

use std::path::Path;

use super::Launch;

pub const DEFAULT_SHELL: &str = "/bin/bash";
pub const MACHINE_NAME: &str = "computer";
pub const GIT_INSTALL_HINT: &str =
    "Git is not installed. Install it with your package manager, for example: sudo apt install git";
pub const GIT_CANDIDATES: &[&str] = &["/usr/bin/git", "/usr/local/bin/git"];

pub fn git_shim() -> Option<(&'static Path, bool)> {
    None
}

pub const IOS_SIMULATORS: bool = false;
pub const BREW_PREFIXES: &[&str] = &[];

/// Android Studio's default, then the folders distribution packages use.
pub fn android_sdk_defaults(home: &Path) -> Vec<std::path::PathBuf> {
    vec![
        home.join("Android/Sdk"),
        "/opt/android-sdk".into(),
        "/usr/lib/android-sdk".into(),
    ]
}

/// The emulator writes discovery files under the runtime folder, or `/tmp/android-<user>`
/// without one (research R12).
pub fn emulator_discovery_dirs(
    _home: &Path,
    vars: &std::collections::HashMap<String, String>,
) -> Vec<std::path::PathBuf> {
    let mut dirs = Vec::new();
    if let Some(run) = vars.get("XDG_RUNTIME_DIR").filter(|v| !v.is_empty()) {
        dirs.push(std::path::Path::new(run).join("avd/running"));
    }
    if let Some(user) = vars.get("USER").filter(|v| !v.is_empty()) {
        dirs.push(format!("/tmp/android-{user}/avd/running").into());
    }
    dirs
}

pub fn open_url_command(url: &str) -> Launch {
    Launch {
        program: "xdg-open".into(),
        args: vec![url.into()],
    }
}

/// File managers differ in how they select a file, so the containing folder is opened.
pub fn reveal_command(path: &Path) -> Launch {
    let folder = path.parent().unwrap_or(path);
    Launch {
        program: "xdg-open".into(),
        args: vec![folder.into()],
    }
}

/// The distribution's `PRETTY_NAME` and the kernel release, for example
/// `Ubuntu 24.04.1 LTS (6.8.0-45-generic)`; either part alone when the other is missing.
pub fn os_version() -> Option<String> {
    let pretty = ["/etc/os-release", "/usr/lib/os-release"]
        .iter()
        .find_map(|p| super::linux_pretty_name(&std::fs::read_to_string(p).ok()?));
    let kernel = super::command_line("uname", &["-r"]);
    match (pretty, kernel) {
        (Some(p), Some(k)) => Some(format!("{p} ({k})")),
        (p, k) => p.or(k),
    }
}

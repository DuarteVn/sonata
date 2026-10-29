//! macOS behavior (feature 011 moved it here unchanged).

use std::path::Path;
use std::process::{Command, Stdio};

use super::Launch;

pub const DEFAULT_SHELL: &str = "/bin/zsh";
pub const MACHINE_NAME: &str = "Mac";
pub const GIT_INSTALL_HINT: &str =
    "Git is not installed. Install the Command Line Tools with: xcode-select --install";
pub const GIT_CANDIDATES: &[&str] = &["/opt/homebrew/bin/git", "/usr/local/bin/git"];

/// `/usr/bin/git` runs only once the Command Line Tools are installed; before that it opens an
/// install prompt.
pub fn git_shim() -> Option<(&'static Path, bool)> {
    let tools = Command::new("/usr/bin/xcode-select")
        .arg("-p")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|s| s.success());
    Some((Path::new("/usr/bin/git"), tools))
}

pub const IOS_SIMULATORS: bool = true;
pub const BREW_PREFIXES: &[&str] = &["/opt/homebrew", "/usr/local"];

/// The default of both Android Studio and a manual command-line tools install.
pub fn android_sdk_defaults(home: &Path) -> Vec<std::path::PathBuf> {
    vec![home.join("Library/Android/sdk")]
}

pub fn emulator_discovery_dirs(
    home: &Path,
    _vars: &std::collections::HashMap<String, String>,
) -> Vec<std::path::PathBuf> {
    vec![home.join("Library/Caches/TemporaryItems/avd/running")]
}

pub fn open_url_command(url: &str) -> Launch {
    Launch {
        program: "/usr/bin/open".into(),
        args: vec![url.into()],
    }
}

pub fn reveal_command(path: &Path) -> Launch {
    Launch {
        program: "/usr/bin/open".into(),
        args: vec!["-R".into(), path.into()],
    }
}

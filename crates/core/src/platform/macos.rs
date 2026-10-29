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

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

//! macOS behavior (feature 011 moved it here unchanged).

use std::path::Path;

use super::Launch;

pub const DEFAULT_SHELL: &str = "/bin/zsh";

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

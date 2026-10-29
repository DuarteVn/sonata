//! Linux behavior (feature 011).

use std::path::Path;

use super::Launch;

pub const DEFAULT_SHELL: &str = "/bin/bash";

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

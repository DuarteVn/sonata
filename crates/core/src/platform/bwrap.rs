//! bubblewrap arguments for custom usage providers on Linux (feature 011, research R9). Pure, so
//! it is tested on every host; only Linux runs it.

use std::path::Path;

pub const BWRAP: &str = "/usr/bin/bwrap";
pub const MISSING: &str =
    "Cannot run safely: bubblewrap (bwrap) is not installed. Install the bubblewrap package.";

/// Everything stays visible as on macOS ("allow default"), except Sonata's data (the providers
/// folder comes back read-only), the agents' credential files, the desktop keyrings, and the
/// runtime folder that holds the D-Bus and keyring sockets. `exists` decides which optional
/// entries apply. The list ends with `--`; the provider's command follows.
pub fn args(
    home: &Path,
    sonata_root: &Path,
    runtime_dir: Option<&Path>,
    exists: impl Fn(&Path) -> bool,
) -> Vec<String> {
    let s = |p: &Path| p.display().to_string();
    let mut out: Vec<String> = [
        "--die-with-parent",
        "--new-session",
        "--unshare-pid",
        "--unshare-ipc",
        "--dev-bind",
        "/",
        "/",
        "--proc",
        "/proc",
    ]
    .iter()
    .map(|a| a.to_string())
    .collect();
    out.extend(["--tmpfs".into(), s(sonata_root)]);
    let providers = sonata_root.join("usage-providers");
    if exists(&providers) {
        out.extend(["--ro-bind".into(), s(&providers), s(&providers)]);
    }
    for file in [".claude/.credentials.json", ".codex/auth.json"] {
        let file = home.join(file);
        if exists(&file) {
            out.extend(["--ro-bind".into(), "/dev/null".into(), s(&file)]);
        }
    }
    let hidden = [
        Some(home.join(".local/share/keyrings")),
        Some(home.join(".local/share/kwalletd")),
        runtime_dir.map(Path::to_path_buf),
    ];
    for dir in hidden.into_iter().flatten() {
        if exists(&dir) {
            out.extend(["--tmpfs".into(), s(&dir)]);
        }
    }
    out.push("--".into());
    out
}

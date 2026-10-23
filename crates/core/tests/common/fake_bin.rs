//! Fake executables for usage tests: no test runs the real `claude` or `codex`.

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

/// Writes `dir/name` as an executable `/bin/sh` script running `body`, and returns its path.
pub fn fake_bin(dir: &Path, name: &str, body: &str) -> PathBuf {
    std::fs::create_dir_all(dir).expect("fake bin dir");
    let path = dir.join(name);
    std::fs::write(&path, format!("#!/bin/sh\n{body}\n")).expect("write fake bin");
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).expect("chmod");
    path
}

/// Path of a usage fixture file.
pub fn usage_fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/usage")
        .join(name)
}

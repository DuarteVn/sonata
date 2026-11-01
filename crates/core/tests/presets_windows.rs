//! Preset detection on Windows looks at PATH with PATHEXT, without a shell (feature 012).
#![cfg(windows)]

use std::time::Duration;

use sonata_core::presets;

#[test]
fn agents_are_found_on_path_with_their_extensions() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("codex.cmd"), "@echo off\r\n").unwrap();
    std::fs::write(dir.path().join("claude.exe"), b"").unwrap();
    // Only test in this binary, so changing PATH cannot race another test.
    // SAFETY: no other thread reads the environment at this point.
    unsafe { std::env::set_var("PATH", dir.path()) };
    let found = presets::detect(std::path::Path::new("unused"), Duration::from_secs(2));
    let get = |id: &str| found.iter().find(|d| d.id == id).unwrap().found;
    assert_eq!(get("claude"), Some(true));
    assert_eq!(get("codex"), Some(true));
    assert_eq!(get("gemini"), Some(false));
    assert_eq!(get("shell"), Some(true));
}

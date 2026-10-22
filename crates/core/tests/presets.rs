use std::os::unix::fs::PermissionsExt;
use std::time::Duration;

use sonata_core::presets::{self, PRESETS};

fn fake_shell(dir: &std::path::Path, body: &str) -> std::path::PathBuf {
    let path = dir.join("fake-shell");
    std::fs::write(&path, format!("#!/bin/sh\n{body}\n")).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    path
}

#[test]
fn builtin_list() {
    let list: Vec<(&str, &str, Option<&str>)> =
        PRESETS.iter().map(|p| (p.id, p.name, p.command)).collect();
    assert_eq!(
        list,
        [
            ("claude", "Claude Code", Some("claude")),
            ("codex", "Codex", Some("codex")),
            ("opencode", "OpenCode", Some("opencode")),
            ("gemini", "Gemini CLI", Some("gemini")),
            ("shell", "Shell", None),
        ]
    );
}

#[test]
fn detection_reports_found_and_missing() {
    let tmp = tempfile::tempdir().unwrap();
    // Pretends only claude and gemini are installed, like `command -v` would print.
    let shell = fake_shell(
        tmp.path(),
        "echo /opt/bin/claude; echo /usr/local/bin/gemini",
    );
    let found = presets::detect(&shell, Duration::from_secs(2));
    let get = |id: &str| found.iter().find(|d| d.id == id).unwrap().found;
    assert_eq!(get("claude"), Some(true));
    assert_eq!(get("codex"), Some(false));
    assert_eq!(get("gemini"), Some(true));
    assert_eq!(get("shell"), Some(true));
}

#[test]
fn slow_shell_means_unknown() {
    let tmp = tempfile::tempdir().unwrap();
    let shell = fake_shell(tmp.path(), "sleep 5");
    let start = std::time::Instant::now();
    let found = presets::detect(&shell, Duration::from_millis(500));
    assert!(start.elapsed() < Duration::from_secs(2));
    assert!(
        found
            .iter()
            .filter(|d| d.id != "shell")
            .all(|d| d.found.is_none())
    );
}

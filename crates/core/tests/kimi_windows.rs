//! The Kimi Code integration is unavailable on Windows and changes nothing (feature 012).
#![cfg(windows)]

use sonata_core::agents::kimi::{WINDOWS_UNAVAILABLE, apply, integration};

#[test]
fn kimi_integration_says_why_and_never_writes() {
    let home = tempfile::tempdir().unwrap();
    let shell = std::path::Path::new("powershell.exe");
    let report = integration(home.path(), shell);
    assert!(!report.can_apply);
    assert_eq!(report.message.as_deref(), Some(WINDOWS_UNAVAILABLE));
    assert!(apply(home.path(), shell).is_err());
    assert!(!home.path().join("config.toml").exists());
}

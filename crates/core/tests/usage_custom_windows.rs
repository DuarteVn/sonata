//! Custom usage providers on Windows: never started, with a clear reason (feature 012, FR-025).
#![cfg(windows)]

use std::path::Path;

use sonata_core::usage::ReadingStatus;
use sonata_core::usage::custom::{WINDOWS_UNAVAILABLE, read_custom_with};

#[test]
fn custom_providers_never_run_on_windows() {
    let tmp = tempfile::tempdir().unwrap();
    let marker = tmp.path().join("ran.txt");
    // Even with an existing "sandbox" program, the command must not start.
    let fake_sandbox = std::env::var_os("ComSpec").unwrap();
    let argv = vec![
        "cmd".to_string(),
        "/C".to_string(),
        format!("echo x > \"{}\"", marker.display()),
    ];
    let r = read_custom_with(
        Path::new(&fake_sandbox),
        "mine",
        &argv,
        &[],
        tmp.path(),
        tmp.path(),
        chrono::Utc::now(),
    );
    assert_eq!(r.status, ReadingStatus::Unavailable);
    assert_eq!(r.message.as_deref(), Some(WINDOWS_UNAVAILABLE));
    std::thread::sleep(std::time::Duration::from_millis(300));
    assert!(!marker.exists(), "the provider command ran");
}

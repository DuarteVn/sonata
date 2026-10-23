mod common;

use std::time::Duration;

use common::fake_bin::fake_bin;
use sonata_core::usage::Detection;
use sonata_core::usage::detect::detect;

/// A stand-in login shell: drops `-l` and runs the script with `/bin/sh`.
fn fake_shell(dir: &std::path::Path) -> std::path::PathBuf {
    fake_bin(dir, "loginsh", r#"shift; exec /bin/sh "$@""#)
}

#[test]
fn finds_full_paths_and_the_login_path() {
    let dir = tempfile::tempdir().unwrap();
    let bin_dir = dir.path().join("bin");
    let tool = fake_bin(&bin_dir, "sonata-test-agent", "exit 0");
    let shell = fake_shell(dir.path());
    let found = detect(
        &shell,
        &["sonata-test-agent", "sonata-test-missing"],
        Some(&bin_dir),
        Duration::from_secs(5),
    );
    assert!(!found.timed_out);
    assert_eq!(
        found.command("sonata-test-agent"),
        (Detection::Found, Some(tool.clone()))
    );
    assert_eq!(
        found.command("sonata-test-missing"),
        (Detection::NotFound, None)
    );
    let path = found.env_value("PATH").unwrap();
    assert!(path.starts_with(&bin_dir.display().to_string()), "{path}");
}

#[test]
fn timeout_leaves_every_command_unknown() {
    let dir = tempfile::tempdir().unwrap();
    let shell = fake_bin(dir.path(), "slowsh", "sleep 5");
    let found = detect(&shell, &["claude"], None, Duration::from_millis(300));
    assert!(found.timed_out);
    assert_eq!(found.command("claude"), (Detection::Unknown, None));
}

#[test]
fn names_with_shell_characters_are_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let shell = fake_shell(dir.path());
    let found = detect(
        &shell,
        &["x; touch /tmp/pwned"],
        None,
        Duration::from_secs(5),
    );
    assert_eq!(
        found.command("x; touch /tmp/pwned"),
        (Detection::NotFound, None)
    );
}

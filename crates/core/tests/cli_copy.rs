//! On Linux the app keeps a copy of its CLI at `~/.local/bin/sonata` (feature 011, R7). It never
//! replaces a program that is not a Sonata CLI.
// Unix shells, modes, or sockets; Windows counterparts live in *_windows.rs (feature 012).
#![cfg(unix)]

use std::os::unix::fs::PermissionsExt;

use sonata_core::platform::{CLI_MARKER, install_cli};

fn cli(body: &str) -> Vec<u8> {
    format!("\x7fELF fake {CLI_MARKER} {body}").into_bytes()
}

#[test]
fn writes_a_missing_copy_with_exec_permission() {
    let tmp = tempfile::tempdir().unwrap();
    let bundled = tmp.path().join("bundled");
    std::fs::write(&bundled, cli("v1")).unwrap();
    let target = tmp.path().join("home/.local/bin/sonata");
    assert!(install_cli(&bundled, &target).unwrap());
    assert_eq!(std::fs::read(&target).unwrap(), cli("v1"));
    let mode = std::fs::metadata(&target).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode, 0o755);
}

#[test]
fn an_identical_copy_is_left_alone() {
    let tmp = tempfile::tempdir().unwrap();
    let bundled = tmp.path().join("bundled");
    let target = tmp.path().join("sonata");
    std::fs::write(&bundled, cli("v1")).unwrap();
    std::fs::write(&target, cli("v1")).unwrap();
    assert!(!install_cli(&bundled, &target).unwrap());
}

#[test]
fn an_older_sonata_cli_is_replaced() {
    let tmp = tempfile::tempdir().unwrap();
    let bundled = tmp.path().join("bundled");
    let target = tmp.path().join("sonata");
    std::fs::write(&bundled, cli("v2")).unwrap();
    std::fs::write(&target, cli("v1")).unwrap();
    assert!(install_cli(&bundled, &target).unwrap());
    assert_eq!(std::fs::read(&target).unwrap(), cli("v2"));
}

#[test]
fn another_program_with_the_same_name_is_never_replaced() {
    let tmp = tempfile::tempdir().unwrap();
    let bundled = tmp.path().join("bundled");
    let target = tmp.path().join("sonata");
    std::fs::write(&bundled, cli("v2")).unwrap();
    std::fs::write(&target, b"#!/bin/sh\necho someone else's sonata\n").unwrap();
    assert!(!install_cli(&bundled, &target).unwrap());
    assert_eq!(
        std::fs::read(&target).unwrap(),
        b"#!/bin/sh\necho someone else's sonata\n"
    );
}

#[test]
fn the_real_cli_carries_the_marker() {
    let cli = env!("CARGO_MANIFEST_DIR").to_string() + "/../cli/src";
    let found = std::fs::read_dir(&cli)
        .unwrap()
        .any(|e| std::fs::read_to_string(e.unwrap().path()).is_ok_and(|s| s.contains(CLI_MARKER)));
    assert!(found, "no file in {cli} uses {CLI_MARKER}");
}

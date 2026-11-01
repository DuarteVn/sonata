mod common;

use std::io::Write;

use common::inode;
use sonata_core::persistence::atomic;

#[test]
fn replaces_whole_file_with_new_inode() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("data.json");
    atomic::write(&path, b"first").unwrap();
    let before = inode(&path);
    atomic::write(&path, b"second, longer").unwrap();
    assert_eq!(std::fs::read(&path).unwrap(), b"second, longer");
    assert_ne!(inode(&path), before);
}

#[test]
fn failure_before_rename_keeps_old_file_and_no_temp() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("data.json");
    atomic::write(&path, b"original").unwrap();
    let result = atomic::write_with(&path, |file| {
        file.write_all(b"partial")?;
        Err(std::io::Error::other("simulated crash"))
    });
    assert!(result.is_err());
    assert_eq!(std::fs::read(&path).unwrap(), b"original");
    let entries: Vec<_> = std::fs::read_dir(dir.path()).unwrap().collect();
    assert_eq!(entries.len(), 1, "temporary file left behind");
}

#[test]
fn written_files_are_private() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("data.json");
    atomic::write(&path, b"x").unwrap();
    common::assert_private_file(&path);
}

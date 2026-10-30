//! Windows bodies of `sonata_core::platform` (feature 012, research R4–R6, R12, R14).
#![cfg(windows)]
mod common;

use std::path::Path;
use std::process::Command;

use sonata_core::paths::{DataRoot, enforce_private_tree};
use sonata_core::platform::{
    self, Kind, allowed_sids, is_executable, is_private, make_private, process_alive, simplify,
    user_sid,
};

const SYSTEM: &str = "S-1-5-18";

fn grant_everyone(path: &Path) {
    let status = Command::new("icacls")
        .arg(path)
        .args(["/grant", "*S-1-1-0:(R)"])
        .output()
        .expect("icacls");
    assert!(status.status.success(), "{:?}", status);
}

#[test]
fn private_folder_allows_only_the_user_and_system() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("box");
    std::fs::create_dir(&dir).unwrap();
    make_private(&dir, Kind::Dir).unwrap();
    let mut sids = allowed_sids(&dir).unwrap().expect("a list");
    sids.sort();
    let mut want = vec![user_sid().unwrap().to_string(), SYSTEM.to_string()];
    want.sort();
    assert_eq!(sids, want);
    assert!(is_private(&dir, Kind::Dir).unwrap());

    // New files inherit the folder's entries.
    let file = dir.join("a.json");
    std::fs::write(&file, b"{}").unwrap();
    assert!(is_private(&file, Kind::File).unwrap());
}

#[test]
fn an_everyone_entry_is_not_private_and_the_tree_pass_fixes_it() {
    let tmp = tempfile::tempdir().unwrap();
    let root = DataRoot::at(tmp.path().join("sonata")).unwrap();
    common::assert_private_dir(root.path());
    let file = root.path().join("loose.txt");
    std::fs::write(&file, b"x").unwrap();
    grant_everyone(&file);
    assert!(!is_private(&file, Kind::File).unwrap());
    assert!(enforce_private_tree(root.path()).unwrap() >= 1);
    assert!(is_private(&file, Kind::File).unwrap());
}

#[test]
fn verbatim_prefixes_are_removed_when_safe() {
    assert_eq!(simplify(Path::new(r"\\?\C:\x\y")), Path::new(r"C:\x\y"));
    assert_eq!(
        simplify(Path::new(r"\\?\UNC\srv\share\x")),
        Path::new(r"\\srv\share\x")
    );
    let long = format!(r"\\?\C:\{}", "a".repeat(300));
    assert_eq!(simplify(Path::new(&long)), Path::new(&long));
    assert_eq!(simplify(Path::new(r"C:\plain")), Path::new(r"C:\plain"));
    let canon = std::env::temp_dir().canonicalize().unwrap();
    assert!(!simplify(&canon).to_string_lossy().starts_with(r"\\?\"));
}

#[test]
fn executables_follow_pathext() {
    let tmp = tempfile::tempdir().unwrap();
    for name in ["a.exe", "b.cmd", "c.txt"] {
        std::fs::write(tmp.path().join(name), b"").unwrap();
    }
    assert!(is_executable(&tmp.path().join("a.exe")));
    assert!(is_executable(&tmp.path().join("b.cmd")));
    assert!(!is_executable(&tmp.path().join("c.txt")));
    assert!(!is_executable(&tmp.path().join("missing.exe")));
    let names = platform::executable_names("claude");
    assert!(names.contains(&"claude.exe".to_string()), "{names:?}");
    assert!(names.contains(&"claude.cmd".to_string()), "{names:?}");
}

#[test]
fn process_liveness() {
    assert!(process_alive(std::process::id()));
    let mut child = Command::new("cmd").args(["/C", "exit 0"]).spawn().unwrap();
    let pid = child.id();
    child.wait().unwrap();
    drop(child);
    assert!(!process_alive(pid));
}

#[test]
fn home_is_the_profile_folder() {
    let home = platform::home_dir().expect("home");
    assert_eq!(home, Path::new(&std::env::var_os("USERPROFILE").unwrap()));
}

// Unix shells, modes, or sockets; Windows counterparts live in *_windows.rs (feature 012).
#![cfg(unix)]
mod common;

use std::ffi::OsStr;
use std::os::unix::fs::PermissionsExt;
use std::time::Duration;

use common::{Sandbox, commit_file, git, git_repo};
use sonata_core::lanes::git::{
    Git, GitError, MIN_VERSION, discover, parse_status_z, parse_version, parse_worktrees_z,
};

#[test]
fn parses_versions() {
    assert_eq!(
        parse_version("git version 2.54.0 (Apple Git-157)"),
        Some((2, 54))
    );
    assert_eq!(parse_version("git version 2.37.1\n"), Some((2, 37)));
    assert_eq!(parse_version("git version 3.0.0.windows.1"), Some((3, 0)));
    assert_eq!(parse_version("nonsense"), None);
    assert!((2, 37) < MIN_VERSION && (2, 38) >= MIN_VERSION);
}

#[test]
fn command_scrubs_redirecting_environment() {
    let g = Git::new("git");
    let cmd = g.command(std::path::Path::new("/tmp"), &["status"]);
    let envs: Vec<(&OsStr, Option<&OsStr>)> = cmd.get_envs().collect();
    for removed in [
        "GIT_DIR",
        "GIT_WORK_TREE",
        "GIT_INDEX_FILE",
        "GIT_COMMON_DIR",
    ] {
        assert!(
            envs.contains(&(OsStr::new(removed), None)),
            "{removed} not removed"
        );
    }
    for (k, v) in [
        ("GIT_TERMINAL_PROMPT", "0"),
        ("GIT_OPTIONAL_LOCKS", "0"),
        ("LC_ALL", "C"),
    ] {
        assert!(envs.contains(&(OsStr::new(k), Some(OsStr::new(v)))), "{k}");
    }
    let args: Vec<&OsStr> = cmd.get_args().collect();
    assert_eq!(args, ["-C", "/tmp", "status"]);
}

#[test]
fn times_out() {
    let sb = Sandbox::new();
    let fake = sb.tmp.path().join("slow-git");
    std::fs::write(&fake, "#!/bin/sh\nsleep 5\n").unwrap();
    std::fs::set_permissions(&fake, std::fs::Permissions::from_mode(0o755)).unwrap();
    let start = std::time::Instant::now();
    let err = Git::new(&fake)
        .run_with(sb.tmp.path(), &["status"], Duration::from_millis(200))
        .unwrap_err();
    assert!(matches!(err, GitError::Timeout(_)), "{err:?}");
    assert!(start.elapsed() < Duration::from_secs(3));
}

#[test]
fn ok_returns_trimmed_stdout_or_stderr_error() {
    let sb = Sandbox::new();
    let repo = git_repo(&sb, "r");
    let g = Git::new("git");
    assert_eq!(
        g.ok(&repo, &["symbolic-ref", "--short", "HEAD"]).unwrap(),
        "main"
    );
    let err = g.ok(&repo, &["rev-parse", "--verify", "nope"]).unwrap_err();
    match err {
        GitError::Failed(msg) => assert!(msg.contains("fatal"), "{msg}"),
        other => panic!("{other:?}"),
    }
}

#[test]
fn parses_porcelain_v2_status() {
    let sb = Sandbox::new();
    let repo = git_repo(&sb, "r");
    commit_file(&repo, "old.txt", "keep\n", "old");
    std::fs::write(repo.join("a.txt"), "changed\n").unwrap();
    std::fs::write(repo.join("new file.txt"), "n\n").unwrap();
    git(&repo, &["mv", "old.txt", "renamed.txt"]);
    let out = Git::new("git")
        .run(
            &repo,
            &["status", "--porcelain=v2", "-z", "--untracked-files=normal"],
        )
        .unwrap();
    let mut paths = parse_status_z(&out.stdout);
    paths.sort();
    assert_eq!(paths, ["a.txt", "new file.txt", "renamed.txt"]);
}

#[test]
fn parses_worktree_list() {
    let sb = Sandbox::new();
    let repo = git_repo(&sb, "r");
    let extra = sb.tmp.path().join("extra wt");
    git(
        &repo,
        &["worktree", "add", "-q", "-b", "x", extra.to_str().unwrap()],
    );
    let out = Git::new("git")
        .run(&repo, &["worktree", "list", "--porcelain", "-z"])
        .unwrap();
    let list = parse_worktrees_z(&out.stdout);
    assert_eq!(list.len(), 2);
    assert_eq!(
        list[1].canonicalize().unwrap(),
        extra.canonicalize().unwrap()
    );
}

#[test]
fn discovers_a_supported_git() {
    let g = discover(std::path::Path::new("/bin/sh")).expect("git on this machine");
    assert!(g.version().unwrap() >= MIN_VERSION);
}

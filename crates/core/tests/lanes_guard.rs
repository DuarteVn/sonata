// Unix shells, modes, or sockets; Windows counterparts live in *_windows.rs (feature 012).
#![cfg(unix)]
mod common;

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use common::{Sandbox, git, git_repo, mode};
use sonata_core::lanes::guard::{GuardError, check_removable};
use uuid::Uuid;

/// A repository plus one real worktree at `<root>/lanes/<ws>/<name>`.
struct Fixture {
    _sb: Sandbox,
    repo: PathBuf,
    lanes_ws: PathBuf,
    lane: PathBuf,
}

fn fixture() -> Fixture {
    let sb = Sandbox::new();
    let repo = git_repo(&sb, "repo");
    let lanes_ws = sb.root.lane_workspace_dir(Uuid::new_v4());
    std::fs::create_dir_all(&lanes_ws).unwrap();
    let lane = lanes_ws.join("api");
    git(
        &repo,
        &[
            "worktree",
            "add",
            "-q",
            "-b",
            "lane/api",
            lane.to_str().unwrap(),
        ],
    );
    Fixture {
        _sb: sb,
        repo,
        lanes_ws,
        lane,
    }
}

fn worktrees(repo: &Path) -> Vec<PathBuf> {
    git(repo, &["worktree", "list", "--porcelain"])
        .lines()
        .filter_map(|l| l.strip_prefix("worktree "))
        .map(PathBuf::from)
        .collect()
}

#[test]
fn accepts_a_registered_lane_checkout() {
    let f = fixture();
    assert_eq!(
        check_removable(&f.lanes_ws, &f.lane, "api", &worktrees(&f.repo)),
        Ok(())
    );
}

#[test]
fn rejects_unsafe_paths() {
    let f = fixture();
    let wt = worktrees(&f.repo);
    let cases: Vec<(PathBuf, &str, GuardError)> = vec![
        (PathBuf::from("lanes/x/api"), "api", GuardError::Relative),
        (
            f.lanes_ws.join("api/../api"),
            "api",
            GuardError::ParentComponent,
        ),
        (f.repo.clone(), "repo", GuardError::OutsideLanes),
        (
            PathBuf::from(format!("{}-evil/api", f.lanes_ws.display())),
            "api",
            GuardError::OutsideLanes,
        ),
        (f.lanes_ws.join("api/src"), "src", GuardError::OutsideLanes),
        (f.lane.clone(), "other", GuardError::NameMismatch),
        (f.lanes_ws.join("gone"), "gone", GuardError::Missing),
    ];
    for (path, name, want) in cases {
        assert_eq!(
            check_removable(&f.lanes_ws, &path, name, &wt),
            Err(want),
            "{}",
            path.display()
        );
    }
}

#[test]
fn rejects_a_symlink_to_a_real_checkout() {
    let f = fixture();
    let link = f.lanes_ws.join("link");
    std::os::unix::fs::symlink(&f.repo, &link).unwrap();
    assert_eq!(
        check_removable(&f.lanes_ws, &link, "link", &worktrees(&f.repo)),
        Err(GuardError::Symlink)
    );
}

#[test]
fn rejects_a_plain_folder_git_does_not_list() {
    let f = fixture();
    let plain = f.lanes_ws.join("plain");
    std::fs::create_dir(&plain).unwrap();
    assert_eq!(
        check_removable(&f.lanes_ws, &plain, "plain", &worktrees(&f.repo)),
        Err(GuardError::NotAWorktree)
    );
}

#[test]
fn permission_pass_skips_lane_checkouts() {
    let f = fixture();
    let script = f.lane.join("run.sh");
    std::fs::write(&script, "#!/bin/sh\n").unwrap();
    std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
    std::fs::set_permissions(&f.lanes_ws, std::fs::Permissions::from_mode(0o755)).unwrap();
    let root = f.lanes_ws.parent().unwrap().parent().unwrap();
    sonata_core::paths::enforce_private_tree(root).unwrap();
    assert_eq!(mode(&script), 0o755);
    assert_eq!(mode(&f.lanes_ws), 0o700);
    assert_eq!(mode(f.lanes_ws.parent().unwrap()), 0o700);
}

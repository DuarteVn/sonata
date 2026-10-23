//! Lane operations: availability, create, preview, land, remove (research R5–R10).

use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::time::Duration;

use chrono::Utc;
use serde::Serialize;

use super::git::Git;
use super::naming;
use crate::error::{AppError, AppResult, ErrorCode};
use crate::model::node::Lane;

const ADD_TIMEOUT: Duration = Duration::from_secs(120);
pub const NOT_A_REPO: &str = "The workspace folder is not inside a git repository.";

/// Whether the workspace folder can host lanes, and why not.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Availability {
    pub available: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_branch: Option<String>,
}

/// The repository facts a new lane needs.
struct RepoInfo {
    top: PathBuf,
    branch: String,
    subdirectory: Option<PathBuf>,
}

/// The git operation that is half done in `git_dir`, if any.
pub fn operation_in_progress(git_dir: &Path) -> Option<&'static str> {
    [
        ("MERGE_HEAD", "merge"),
        ("CHERRY_PICK_HEAD", "cherry-pick"),
        ("REVERT_HEAD", "revert"),
        ("rebase-merge", "rebase"),
        ("rebase-apply", "rebase"),
    ]
    .into_iter()
    .find(|(file, _)| git_dir.join(file).exists())
    .map(|(_, op)| op)
}

fn repo_info(git: &Git, folder: &Path) -> Result<RepoInfo, String> {
    let top = git
        .ok(folder, &["rev-parse", "--show-toplevel"])
        .map_err(|_| NOT_A_REPO.to_string())?;
    let git_dir = git
        .ok(folder, &["rev-parse", "--absolute-git-dir"])
        .map_err(|e| e.to_string())?;
    if let Some(op) = operation_in_progress(Path::new(&git_dir)) {
        return Err(format!(
            "A {op} is in progress in the workspace folder. Finish or abort it first."
        ));
    }
    git.ok(folder, &["rev-parse", "--verify", "-q", "HEAD"])
        .map_err(|_| "The repository has no commits yet.".to_string())?;
    let branch = git
        .ok(folder, &["symbolic-ref", "-q", "--short", "HEAD"])
        .map_err(|_| {
            "The workspace folder has no branch checked out (detached HEAD).".to_string()
        })?;
    let prefix = git
        .ok(folder, &["rev-parse", "--show-prefix"])
        .map_err(|e| e.to_string())?;
    let prefix = prefix.trim_end_matches('/');
    Ok(RepoInfo {
        top: PathBuf::from(top),
        branch,
        subdirectory: (!prefix.is_empty()).then(|| PathBuf::from(prefix)),
    })
}

pub fn availability(git: Result<&Git, &str>, folder: &Path) -> Availability {
    let result = git
        .map_err(str::to_string)
        .and_then(|g| repo_info(g, folder));
    match result {
        Ok(info) => Availability {
            available: true,
            reason: None,
            base_branch: Some(info.branch),
        },
        Err(reason) => Availability {
            available: false,
            reason: Some(reason),
            base_branch: None,
        },
    }
}

pub fn branch_exists(git: &Git, repo: &Path, branch: &str) -> bool {
    git.run(
        repo,
        &[
            "show-ref",
            "--verify",
            "--quiet",
            &format!("refs/heads/{branch}"),
        ],
    )
    .is_ok_and(|o| o.success())
}

fn unavailable(message: impl Into<String>) -> AppError {
    AppError::new(ErrorCode::LaneUnavailable, message)
}

/// Creates the branch and checkout for a new terminal named `name` (research R5).
pub fn create(git: &Git, lanes_ws_dir: &Path, folder: &Path, name: &str) -> AppResult<Lane> {
    let info = repo_info(git, folder).map_err(unavailable)?;
    let head = git
        .ok(folder, &["rev-parse", "HEAD"])
        .map_err(|e| unavailable(e.to_string()))?;
    crate::paths::ensure_private_dir(lanes_ws_dir)?;
    let slug = naming::unique(&naming::slug(name), |s| {
        lanes_ws_dir.join(s).exists() || branch_exists(git, &info.top, &naming::branch(s))
    });
    let branch = naming::branch(&slug);
    git.ok(&info.top, &["check-ref-format", "--branch", &branch])
        .map_err(|_| unavailable(format!("\"{branch}\" is not a valid branch name.")))?;
    let path = lanes_ws_dir.join(&slug);
    let args: [&OsStr; 7] = [
        "worktree".as_ref(),
        "add".as_ref(),
        "-q".as_ref(),
        "-b".as_ref(),
        branch.as_ref(),
        path.as_os_str(),
        head.as_ref(),
    ];
    let failure = match git.run_with(&info.top, &args, ADD_TIMEOUT) {
        Ok(out) if out.success() => None,
        Ok(out) => Some(out.stderr.trim().to_string()),
        Err(e) => Some(e.to_string()),
    };
    if let Some(message) = failure {
        // Undo a branch that `worktree add` created before failing, if it holds nothing new.
        if git
            .ok(&info.top, &["rev-parse", "--verify", "-q", &branch])
            .is_ok_and(|sha| sha == head)
        {
            let _ = git.run(&info.top, &["branch", "-D", &branch]);
        }
        let _ = std::fs::remove_dir(lanes_ws_dir);
        return Err(unavailable(format!("Could not create the lane: {message}")));
    }
    Ok(Lane {
        branch,
        base_branch: info.branch,
        main_checkout: info.top,
        checkout_path: path,
        subdirectory: info.subdirectory,
        created_at: Utc::now(),
        landed_at: None,
    })
}

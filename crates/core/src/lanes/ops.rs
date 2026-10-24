//! Lane operations: availability, create, preview, land, remove (research R5–R10).

use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::time::Duration;

use chrono::Utc;
use serde::Serialize;

use super::git::Git;
use super::guard::{self, GuardError};
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

/// What a land or remove dialog shows (research R10).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Preview {
    pub branch: String,
    pub base_branch: String,
    /// Commits on the lane that the base branch lacks.
    pub commits: u32,
    /// Uncommitted files in the lane (first 20).
    pub changed_files: Vec<String>,
    pub changed_total: usize,
    /// Commits on the lane branch that no other branch contains.
    pub unmerged_commits: u32,
    /// Non-empty: the action cannot run, for these reasons.
    pub blockers: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Action {
    Land,
    Remove,
}

const PREVIEW_FILES: usize = 20;
const CONFLICT_FILES: usize = 10;

fn count(git: &Git, dir: &Path, args: &[&str]) -> u32 {
    git.ok(dir, args)
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0)
}

/// Uncommitted paths in `dir`; `untracked` decides whether new files count.
pub fn changed_paths(git: &Git, dir: &Path, untracked: bool) -> Result<Vec<String>, String> {
    let mode = if untracked {
        "--untracked-files=normal"
    } else {
        "--untracked-files=no"
    };
    let out = git
        .run(dir, &["status", "--porcelain=v2", "-z", mode])
        .map_err(|e| e.to_string())?;
    if !out.success() {
        return Err(out.stderr.trim().to_string());
    }
    Ok(super::git::parse_status_z(&out.stdout))
}

fn plural(n: usize, word: &str) -> String {
    if n == 1 {
        format!("1 {word}")
    } else {
        format!("{n} {word}s")
    }
}

pub fn preview(git: &Git, lane: &Lane, action: Action) -> Preview {
    let mut blockers = Vec::new();
    let main = &lane.main_checkout;
    let branch_ok = branch_exists(git, main, &lane.branch);
    if !branch_ok {
        blockers.push(format!("The branch {} no longer exists.", lane.branch));
    }
    let checkout_ok = lane.checkout_path.is_dir();
    let changed = if checkout_ok {
        changed_paths(git, &lane.checkout_path, true).unwrap_or_else(|e| {
            blockers.push(format!("Could not read the lane: {e}"));
            Vec::new()
        })
    } else {
        Vec::new()
    };
    let commits = if branch_ok {
        count(
            git,
            main,
            &[
                "rev-list",
                "--count",
                &format!("{}..{}", lane.base_branch, lane.branch),
            ],
        )
    } else {
        0
    };
    let unmerged_commits = if branch_ok {
        count(
            git,
            main,
            &[
                "rev-list",
                "--count",
                &lane.branch,
                "--not",
                &format!("--exclude={}", lane.branch),
                "--branches",
            ],
        )
    } else {
        0
    };
    if action == Action::Land {
        if !checkout_ok {
            blockers.push("The lane folder is missing.".into());
        } else if !changed.is_empty() {
            blockers.push(format!(
                "The lane has {}. Commit them first.",
                plural(changed.len(), "uncommitted file")
            ));
        }
        blockers.extend(main_checkout_blockers(git, lane));
    }
    Preview {
        branch: lane.branch.clone(),
        base_branch: lane.base_branch.clone(),
        commits,
        changed_total: changed.len(),
        changed_files: changed.into_iter().take(PREVIEW_FILES).collect(),
        unmerged_commits,
        blockers,
    }
}

/// Why the main checkout cannot receive a land right now.
fn main_checkout_blockers(git: &Git, lane: &Lane) -> Vec<String> {
    let main = &lane.main_checkout;
    if !main.is_dir() {
        return vec![format!(
            "The workspace's repository folder {} is missing.",
            main.display()
        )];
    }
    let mut out = Vec::new();
    if let Ok(dir) = git.ok(main, &["rev-parse", "--absolute-git-dir"])
        && let Some(op) = operation_in_progress(Path::new(&dir))
    {
        out.push(format!(
            "A {op} is in progress in the workspace folder. Finish or abort it first."
        ));
    }
    match git.ok(main, &["symbolic-ref", "-q", "--short", "HEAD"]) {
        Ok(current) if current == lane.base_branch => {}
        Ok(current) => out.push(format!(
            "The workspace folder is on {current}, not {}. Switch it back to {} first.",
            lane.base_branch, lane.base_branch
        )),
        Err(_) => out.push(format!(
            "The workspace folder has no branch checked out. Check out {} first.",
            lane.base_branch
        )),
    }
    match changed_paths(git, main, false) {
        Ok(c) if c.is_empty() => {}
        Ok(c) => out.push(format!(
            "The workspace folder has {}. Commit or stash them first.",
            plural(c.len(), "uncommitted change")
        )),
        Err(e) => out.push(format!("Could not read the workspace folder: {e}")),
    }
    out
}

fn blocked(message: impl Into<String>) -> AppError {
    AppError::new(ErrorCode::LaneBlocked, message)
}

/// One lock per repository, so lands into it run one at a time (FR-012).
fn repo_lock(git: &Git, main: &Path) -> std::sync::Arc<std::sync::Mutex<()>> {
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex, OnceLock};
    static LOCKS: OnceLock<Mutex<HashMap<PathBuf, Arc<Mutex<()>>>>> = OnceLock::new();
    let key = git
        .ok(
            main,
            &["rev-parse", "--path-format=absolute", "--git-common-dir"],
        )
        .map(PathBuf::from)
        .unwrap_or_else(|_| main.to_path_buf());
    let key = key.canonicalize().unwrap_or(key);
    LOCKS
        .get_or_init(Mutex::default)
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .entry(key)
        .or_default()
        .clone()
}

/// Conflicting paths from `merge-tree --write-tree --name-only -z` (exit code 1).
fn conflicted_paths(stdout: &[u8]) -> Vec<String> {
    let mut paths: Vec<String> = stdout
        .split(|b| *b == 0)
        .skip(1)
        .take_while(|f| !f.is_empty())
        .map(|f| String::from_utf8_lossy(f).into_owned())
        .collect();
    paths.dedup();
    paths
}

fn conflict_message(paths: &[String], base: &str) -> String {
    let shown = paths
        .iter()
        .take(CONFLICT_FILES)
        .cloned()
        .collect::<Vec<_>>()
        .join(", ");
    let more = paths.len().saturating_sub(CONFLICT_FILES);
    let more = if more > 0 {
        format!(" and {more} more")
    } else {
        String::new()
    };
    format!(
        "Land cancelled: conflicts in {shown}{more}. Nothing changed. Merge {base} into the \
         lane, resolve, commit, and land again."
    )
}

/// Merges the lane into its base branch in the main checkout (research R6). Returns the number
/// of commits brought in.
pub fn land(git: &Git, lane: &Lane) -> AppResult<u32> {
    let lock = repo_lock(git, &lane.main_checkout);
    let _guard = lock.lock().unwrap_or_else(|e| e.into_inner());
    let p = preview(git, lane, Action::Land);
    if !p.blockers.is_empty() {
        return Err(blocked(p.blockers.join(" ")));
    }
    if p.commits == 0 {
        return Err(blocked(format!(
            "Nothing to land: {} already has every commit of {}.",
            lane.base_branch, lane.branch
        )));
    }
    let main = &lane.main_checkout;
    let dry = git
        .run(
            main,
            &[
                "merge-tree",
                "--write-tree",
                "--name-only",
                "-z",
                &lane.base_branch,
                &lane.branch,
            ],
        )
        .map_err(|e| blocked(e.to_string()))?;
    match dry.code {
        Some(0) => {}
        Some(1) => {
            return Err(AppError::new(
                ErrorCode::LaneConflict,
                conflict_message(&conflicted_paths(&dry.stdout), &lane.base_branch),
            ));
        }
        _ => return Err(blocked(format!("Land failed: {}", dry.stderr.trim()))),
    }
    let head = git
        .ok(main, &["rev-parse", "HEAD"])
        .map_err(|e| blocked(e.to_string()))?;
    let message = format!("Merge lane {} into {}", lane.branch, lane.base_branch);
    let merged = git
        .run(
            main,
            &[
                "merge",
                "--no-ff",
                "--no-edit",
                "-m",
                &message,
                &lane.branch,
            ],
        )
        .map_err(|e| blocked(e.to_string()))?;
    if merged.success() {
        return Ok(p.commits);
    }
    if let Ok(dir) = git.ok(main, &["rev-parse", "--absolute-git-dir"])
        && Path::new(&dir).join("MERGE_HEAD").exists()
    {
        let _ = git.run(main, &["merge", "--abort"]);
    }
    let now = git.ok(main, &["rev-parse", "HEAD"]).unwrap_or_default();
    let detail = format!("{} {}", merged.text(), merged.stderr.trim());
    if now != head {
        tracing::error!(%head, %now, "HEAD moved during a failed land");
    }
    Err(blocked(format!("Land failed: {}", detail.trim())))
}

fn unsafe_path(lane: &Lane, e: GuardError) -> AppError {
    AppError::new(
        ErrorCode::LaneUnsafePath,
        format!("Refusing to delete {}: {e}.", lane.checkout_path.display()),
    )
}

/// Runs the removal guard. `Ok(true)`: the checkout exists and may be deleted; `Ok(false)`: it
/// is already gone.
pub fn check_checkout(git: &Git, lanes_ws_dir: &Path, lane: &Lane) -> AppResult<bool> {
    let worktrees = git
        .run(
            &lane.main_checkout,
            &["worktree", "list", "--porcelain", "-z"],
        )
        .map(|o| super::git::parse_worktrees_z(&o.stdout))
        .unwrap_or_default();
    match guard::check_removable(lanes_ws_dir, &lane.checkout_path, lane.slug(), &worktrees) {
        Ok(()) => Ok(true),
        Err(GuardError::Missing) => Ok(false),
        Err(e) => Err(unsafe_path(lane, e)),
    }
}

/// Checks, before anything stops or changes, that a lane may be removed: the path is safe and
/// uncommitted work is either absent or explicitly discarded (FR-014, FR-016).
pub fn prepare_removal(
    git: &Git,
    lanes_ws_dir: &Path,
    lane: &Lane,
    discard: bool,
) -> AppResult<()> {
    if !check_checkout(git, lanes_ws_dir, lane)? || discard {
        return Ok(());
    }
    let changed = changed_paths(git, &lane.checkout_path, true).map_err(blocked)?;
    if changed.is_empty() {
        return Ok(());
    }
    let shown = changed
        .iter()
        .take(CONFLICT_FILES)
        .cloned()
        .collect::<Vec<_>>()
        .join(", ");
    let more = changed.len().saturating_sub(CONFLICT_FILES);
    let more = if more > 0 {
        format!(" and {more} more")
    } else {
        String::new()
    };
    Err(AppError::new(
        ErrorCode::LaneHasChanges,
        format!(
            "The lane has {}: {shown}{more}. Confirm that they will be lost.",
            plural(changed.len(), "uncommitted file")
        ),
    ))
}

/// Removes the checkout and, when asked, the branch (research R7, R8). Call
/// [`prepare_removal`] first.
pub fn remove(
    git: &Git,
    lanes_ws_dir: &Path,
    lane: &Lane,
    discard: bool,
    delete_branch: bool,
) -> AppResult<()> {
    remove_checkout(git, lanes_ws_dir, lane, discard)?;
    if delete_branch {
        delete_lane_branch(git, lane)?;
    }
    Ok(())
}

fn delete_lane_branch(git: &Git, lane: &Lane) -> AppResult<()> {
    let main = &lane.main_checkout;
    if !lane.branch.starts_with(naming::BRANCH_PREFIX) || lane.branch == lane.base_branch {
        return Err(blocked(format!(
            "Refusing to delete {}: it is not a lane branch.",
            lane.branch
        )));
    }
    if !branch_exists(git, main, &lane.branch) {
        return Ok(());
    }
    if git
        .run(main, &["branch", "-d", &lane.branch])
        .is_ok_and(|o| o.success())
    {
        return Ok(());
    }
    git.ok(main, &["branch", "-D", &lane.branch])
        .map(|_| ())
        .map_err(|e| blocked(format!("Could not delete {}: {e}", lane.branch)))
}

/// Deletes a lane checkout through git after the removal guard passes (research R7). A folder
/// that is already gone only gets `worktree prune`.
pub fn remove_checkout(git: &Git, lanes_ws_dir: &Path, lane: &Lane, force: bool) -> AppResult<()> {
    let main = &lane.main_checkout;
    let path = &lane.checkout_path;
    if check_checkout(git, lanes_ws_dir, lane)? {
        let mut args: Vec<&OsStr> = vec!["worktree".as_ref(), "remove".as_ref()];
        if force {
            args.push("--force".as_ref());
        }
        args.push(path.as_os_str());
        let out = git.run(main, &args).map_err(|e| blocked(e.to_string()))?;
        if !out.success() {
            return Err(blocked(format!(
                "Could not remove the lane folder: {}",
                out.stderr.trim()
            )));
        }
    }
    let _ = git.run(main, &["worktree", "prune"]);
    let _ = std::fs::remove_dir(lanes_ws_dir);
    Ok(())
}

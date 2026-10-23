//! The check that runs before any lane checkout is deleted (research R7, FR-016).

use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum GuardError {
    #[error("the lane path is not absolute")]
    Relative,
    #[error("the lane path contains '..'")]
    ParentComponent,
    #[error("the lane path is not inside Sonata's lanes folder for this workspace")]
    OutsideLanes,
    #[error("the lane folder name does not match its branch")]
    NameMismatch,
    #[error("the lane path is a symbolic link")]
    Symlink,
    #[error("the lane folder no longer exists")]
    Missing,
    #[error("git does not list this folder as a checkout of the repository")]
    NotAWorktree,
}

/// Passes only for `<lanes_ws_dir>/<expected_name>`, a real folder that git lists as a worktree.
pub fn check_removable(
    lanes_ws_dir: &Path,
    recorded: &Path,
    expected_name: &str,
    worktrees: &[PathBuf],
) -> Result<(), GuardError> {
    if !recorded.is_absolute() {
        return Err(GuardError::Relative);
    }
    if crate::paths::reject_parent_components(recorded).is_err() {
        return Err(GuardError::ParentComponent);
    }
    if recorded.parent() != Some(lanes_ws_dir) {
        return Err(GuardError::OutsideLanes);
    }
    if recorded.file_name().and_then(|n| n.to_str()) != Some(expected_name) {
        return Err(GuardError::NameMismatch);
    }
    let meta = std::fs::symlink_metadata(recorded).map_err(|_| GuardError::Missing)?;
    if meta.file_type().is_symlink() {
        return Err(GuardError::Symlink);
    }
    if !meta.is_dir() {
        return Err(GuardError::NotAWorktree);
    }
    let real = recorded.canonicalize().map_err(|_| GuardError::Missing)?;
    let real_parent = lanes_ws_dir
        .canonicalize()
        .map_err(|_| GuardError::OutsideLanes)?;
    if real.parent() != Some(real_parent.as_path()) {
        return Err(GuardError::OutsideLanes);
    }
    let listed = worktrees
        .iter()
        .any(|w| w.canonicalize().is_ok_and(|w| w == real));
    if listed {
        Ok(())
    } else {
        Err(GuardError::NotAWorktree)
    }
}

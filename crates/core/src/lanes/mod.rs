//! Lanes (feature 008): a terminal's own git worktree and branch, landed back with a merge.
//! Every git call goes through the system `git` binary (constitution, research R1).

pub mod confirm;
pub mod git;
pub mod guard;
pub mod naming;
pub mod ops;
pub mod status;

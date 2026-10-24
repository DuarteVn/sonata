//! Lane status shown on terminal nodes (research R11, FR-018, FR-020).

use std::collections::HashMap;

use serde::Serialize;
use uuid::Uuid;

use super::git::Git;
use crate::model::node::Lane;

pub const POLL_INTERVAL: std::time::Duration = std::time::Duration::from_secs(3);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum LaneState {
    Ok,
    Landed,
    CheckoutMissing,
    BranchMissing,
    MainCheckoutMissing,
    /// A merge, rebase, or cherry-pick is in progress in the lane.
    Busy,
    GitUnavailable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LaneStatus {
    pub node_id: Uuid,
    pub state: LaneState,
    pub branch: String,
    pub base_branch: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub changed: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ahead: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub behind: Option<u32>,
}

pub fn check(git: Result<&Git, &str>, node_id: Uuid, lane: &Lane) -> LaneStatus {
    let mut status = LaneStatus {
        node_id,
        state: LaneState::Ok,
        branch: lane.branch.clone(),
        base_branch: lane.base_branch.clone(),
        changed: None,
        ahead: None,
        behind: None,
    };
    let state = |mut s: LaneStatus, state| {
        s.state = state;
        s
    };
    if lane.is_landed() {
        return state(status, LaneState::Landed);
    }
    let Ok(git) = git else {
        return state(status, LaneState::GitUnavailable);
    };
    if !lane.main_checkout.is_dir() {
        return state(status, LaneState::MainCheckoutMissing);
    }
    if !lane.checkout_path.is_dir() {
        return state(status, LaneState::CheckoutMissing);
    }
    if !super::ops::branch_exists(git, &lane.main_checkout, &lane.branch) {
        return state(status, LaneState::BranchMissing);
    }
    if let Ok(dir) = git.ok(&lane.checkout_path, &["rev-parse", "--absolute-git-dir"])
        && super::ops::operation_in_progress(std::path::Path::new(&dir)).is_some()
    {
        status.state = LaneState::Busy;
    }
    status.changed = super::ops::changed_paths(git, &lane.checkout_path, true)
        .ok()
        .map(|c| c.len());
    let range = format!("{}...{}", lane.base_branch, lane.branch);
    if let Ok(counts) = git.ok(
        &lane.main_checkout,
        &["rev-list", "--left-right", "--count", &range],
    ) {
        let mut parts = counts.split_whitespace().map(|n| n.parse().ok());
        status.behind = parts.next().flatten();
        status.ahead = parts.next().flatten();
    }
    status
}

/// Remembers the last status of each lane so only changes are sent.
#[derive(Debug, Default)]
pub struct StatusCache {
    last: HashMap<Uuid, LaneStatus>,
}

impl StatusCache {
    /// Stores `statuses` and returns the ones that differ from the last poll. Lanes that are no
    /// longer listed are forgotten.
    pub fn update(&mut self, statuses: Vec<LaneStatus>) -> Vec<LaneStatus> {
        let mut next = HashMap::with_capacity(statuses.len());
        let mut changed = Vec::new();
        for s in statuses {
            if self.last.get(&s.node_id) != Some(&s) {
                changed.push(s.clone());
            }
            next.insert(s.node_id, s);
        }
        self.last = next;
        changed
    }

    pub fn is_empty(&self) -> bool {
        self.last.is_empty()
    }

    pub fn get(&self, node: Uuid) -> Option<&LaneStatus> {
        self.last.get(&node)
    }

    /// Makes the next poll send these lanes again (a workspace was opened).
    pub fn forget(&mut self, nodes: &[Uuid]) {
        for n in nodes {
            self.last.remove(n);
        }
    }
}

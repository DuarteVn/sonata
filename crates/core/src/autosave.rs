//! Periodic saving of changed workspaces (FR-024).

use std::time::Duration;

use crate::model::preferences::Preferences;
use crate::workspace::manager::{SaveReport, WorkspaceManager};

pub fn interval(prefs: &Preferences) -> Duration {
    Duration::from_secs(u64::from(prefs.normalized().autosave_interval_seconds))
}

/// One autosave pass: writes only workspaces whose revision changed since their last save.
pub fn tick(manager: &mut WorkspaceManager) -> SaveReport {
    manager.save_dirty()
}

/// Final save on quit.
pub fn save_all(manager: &mut WorkspaceManager) -> SaveReport {
    manager.save_dirty()
}

use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const APP_STATE_TYPE: &str = "appState";
pub const APP_STATE_SCHEMA_VERSION: u32 = 1;
pub const RECENT_MAX: usize = 10;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AppState {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active_workspace_id: Option<Uuid>,
    pub recent_workspace_ids: Vec<Uuid>,
    pub workspace_order: Vec<Uuid>,
    pub clean_shutdown: bool,
    pub outlived_launch_window: bool,
    pub consecutive_early_crashes: u32,
}

impl Default for AppState {
    /// A first launch counts as following a clean shutdown.
    fn default() -> Self {
        Self {
            active_workspace_id: None,
            recent_workspace_ids: Vec::new(),
            workspace_order: Vec::new(),
            clean_shutdown: true,
            outlived_launch_window: true,
            consecutive_early_crashes: 0,
        }
    }
}

impl AppState {
    /// Moves `id` to the front of the recent list, keeping at most [`RECENT_MAX`] entries.
    pub fn touch_recent(&mut self, id: Uuid) {
        self.recent_workspace_ids.retain(|x| *x != id);
        self.recent_workspace_ids.insert(0, id);
        self.recent_workspace_ids.truncate(RECENT_MAX);
    }

    pub fn forget(&mut self, id: Uuid) {
        self.recent_workspace_ids.retain(|x| *x != id);
        self.workspace_order.retain(|x| *x != id);
        if self.active_workspace_id == Some(id) {
            self.active_workspace_id = None;
        }
    }
}

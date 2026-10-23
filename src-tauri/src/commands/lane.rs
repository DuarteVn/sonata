//! Lane commands (feature 008). Each forwards to `sonata_core::App`.

use std::sync::Arc;

use sonata_core::lanes::ops::Availability;
use sonata_core::{App, AppError};
use tauri::State;
use uuid::Uuid;

#[tauri::command]
pub fn lane_check(app: State<'_, Arc<App>>, workspace_id: Uuid) -> Result<Availability, AppError> {
    app.lane_check(workspace_id)
}

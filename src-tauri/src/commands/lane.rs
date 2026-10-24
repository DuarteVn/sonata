//! Lane commands (feature 008). Each forwards to `sonata_core::App`.

use std::sync::Arc;

use sonata_core::lanes::ops::{Action, Availability, Preview};
use sonata_core::{App, AppError};
use tauri::State;
use uuid::Uuid;

#[tauri::command]
pub fn lane_check(app: State<'_, Arc<App>>, workspace_id: Uuid) -> Result<Availability, AppError> {
    app.lane_check(workspace_id)
}

#[tauri::command]
pub fn lane_preview(
    app: State<'_, Arc<App>>,
    node_id: Uuid,
    action: Action,
) -> Result<Preview, AppError> {
    app.lane_preview(node_id, action)
}

#[tauri::command]
pub fn lane_land(
    app: State<'_, Arc<App>>,
    node_id: Uuid,
    confirmed: bool,
) -> Result<u32, AppError> {
    app.lane_land(node_id, confirmed)
}

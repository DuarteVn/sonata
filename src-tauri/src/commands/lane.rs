//! Lane commands (feature 008). Each forwards to `sonata_core::App`.

use std::sync::Arc;

use sonata_core::app::LaneRemoval;
use sonata_core::lanes::confirm::Answer;
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

#[tauri::command]
pub fn lane_remove(
    app: State<'_, Arc<App>>,
    node_id: Uuid,
    discard_changes: bool,
    delete_branch: bool,
    confirmed: bool,
) -> Result<(), AppError> {
    app.lane_remove(
        node_id,
        LaneRemoval {
            discard_changes,
            delete_branch,
        },
        confirmed,
    )
}

#[tauri::command]
pub fn lane_forget(app: State<'_, Arc<App>>, node_id: Uuid) -> Result<(), AppError> {
    app.lane_forget(node_id)
}

#[tauri::command]
pub fn lane_confirm_answer(
    app: State<'_, Arc<App>>,
    request_id: Uuid,
    accepted: bool,
    discard_changes: Option<bool>,
    delete_branch: Option<bool>,
) -> Result<(), AppError> {
    app.answer_lane_confirmation(
        request_id,
        Answer {
            accepted,
            discard_changes: discard_changes.unwrap_or(false),
            delete_branch: delete_branch.unwrap_or(false),
        },
    )
}

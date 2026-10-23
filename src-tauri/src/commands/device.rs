use std::path::PathBuf;
use std::sync::Arc;

use sonata_core::device::tools::ToolReport;
use sonata_core::model::node::{Frame, Node, Platform};
use sonata_core::{App, AppError};
use tauri::State;
use uuid::Uuid;

#[tauri::command]
pub fn node_create_device(
    app: State<'_, Arc<App>>,
    workspace_id: Uuid,
    frame: Frame,
) -> Result<Node, AppError> {
    app.create_device(workspace_id, frame)
}

/// SDK, virtual devices, and Xcode as found now; "Check again" calls it again (FR-002).
#[tauri::command]
pub async fn device_tools(app: State<'_, Arc<App>>) -> Result<ToolReport, AppError> {
    Ok(app.device_tools())
}

#[tauri::command]
pub async fn device_choose_sdk(
    app: State<'_, Arc<App>>,
    path: PathBuf,
) -> Result<ToolReport, AppError> {
    Ok(app.device_choose_sdk(&path))
}

#[tauri::command]
pub fn device_select(
    app: State<'_, Arc<App>>,
    node_id: Uuid,
    platform: Platform,
    device_id: String,
    name: Option<String>,
    os_version: Option<String>,
) -> Result<Node, AppError> {
    app.select_device(
        node_id,
        platform,
        &device_id,
        name.as_deref(),
        os_version.as_deref(),
    )
}

#[tauri::command]
pub fn device_open_help(app: State<'_, Arc<App>>, topic: String) -> Result<(), AppError> {
    app.device_open_help(&topic)
}

use std::path::PathBuf;
use std::sync::Arc;

use sonata_core::device::DeviceStateEvent;
use sonata_core::device::live::LiveInput;
use sonata_core::device::tools::ToolReport;
use sonata_core::model::node::{Frame, Node, Platform};
use sonata_core::{App, AppError};
use tauri::State;
use tauri::ipc::{Channel, InvokeResponseBody};
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

#[tauri::command]
pub fn device_state(app: State<'_, Arc<App>>, node_id: Uuid) -> DeviceStateEvent {
    app.device_state(node_id)
}

/// Starts the device; readiness arrives as `device://state` events.
#[tauri::command]
pub async fn device_boot(app: State<'_, Arc<App>>, node_id: Uuid) -> Result<(), AppError> {
    app.device_boot(node_id)
}

#[tauri::command]
pub async fn device_stop(app: State<'_, Arc<App>>, node_id: Uuid) -> Result<(), AppError> {
    app.device_stop(node_id)
}

/// Live screen frames as raw bytes: 12-byte header, then RGBA (contracts/ipc.md).
#[tauri::command]
pub async fn device_stream(
    app: State<'_, Arc<App>>,
    node_id: Uuid,
    width: u32,
    height: u32,
    channel: Channel<InvokeResponseBody>,
) -> Result<(), AppError> {
    app.device_stream(
        node_id,
        width,
        height,
        Box::new(move |bytes| {
            let _ = channel.send(InvokeResponseBody::Raw(bytes));
        }),
    )
}

#[tauri::command]
pub async fn device_unstream(
    app: State<'_, Arc<App>>,
    node_id: Uuid,
) -> Result<Option<PathBuf>, AppError> {
    app.device_unstream(node_id)
}

#[tauri::command]
pub fn device_frame_ack(app: State<'_, Arc<App>>, node_id: Uuid) {
    app.device_frame_ack(node_id);
}

#[tauri::command]
pub async fn device_input(
    app: State<'_, Arc<App>>,
    node_id: Uuid,
    input: LiveInput,
) -> Result<(), AppError> {
    app.device_input(node_id, input)
}

#[tauri::command]
pub async fn device_screenshot(
    app: State<'_, Arc<App>>,
    node_id: Uuid,
    out: PathBuf,
) -> Result<PathBuf, AppError> {
    app.device_screenshot(node_id, Some(&out))
}

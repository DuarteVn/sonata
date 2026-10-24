use std::path::PathBuf;
use std::sync::Arc;

use sonata_core::app::{LaneRemoval, NewTerminal, TerminalSettings};
use sonata_core::model::node::{Frame, Node};
use sonata_core::model::workspace::Camera;
use sonata_core::{App, AppError};
use tauri::State;
use uuid::Uuid;

#[tauri::command]
#[allow(clippy::too_many_arguments)] // one argument per field of the IPC contract
pub fn node_create_terminal(
    app: State<'_, Arc<App>>,
    workspace_id: Uuid,
    frame: Frame,
    name: Option<String>,
    launch_command: Option<String>,
    working_directory: Option<PathBuf>,
    settings: Option<TerminalSettings>,
    lane: Option<bool>,
) -> Result<Node, AppError> {
    app.create_terminal(
        workspace_id,
        NewTerminal {
            frame,
            name,
            launch_command,
            working_directory,
            settings: settings.unwrap_or_default(),
            lane: lane.unwrap_or(false),
        },
    )
}

#[tauri::command]
pub fn node_update_frame(
    app: State<'_, Arc<App>>,
    workspace_id: Uuid,
    node_id: Uuid,
    frame: Frame,
) -> Result<Frame, AppError> {
    app.update_frame(workspace_id, node_id, frame)
}

#[derive(serde::Serialize)]
pub struct ZOrder {
    z: i64,
}

#[tauri::command]
pub fn node_bring_to_front(
    app: State<'_, Arc<App>>,
    workspace_id: Uuid,
    node_id: Uuid,
) -> Result<ZOrder, AppError> {
    app.bring_to_front(workspace_id, node_id)
        .map(|z| ZOrder { z })
}

#[tauri::command]
pub fn node_delete(
    app: State<'_, Arc<App>>,
    workspace_id: Uuid,
    node_id: Uuid,
    confirmed: bool,
    lane_removal: Option<LaneRemoval>,
) -> Result<(), AppError> {
    match lane_removal {
        Some(removal) => app.delete_node_with_lane(workspace_id, node_id, confirmed, removal),
        None => app.delete_node(workspace_id, node_id, confirmed),
    }
}

#[tauri::command]
pub fn workspace_set_camera(
    app: State<'_, Arc<App>>,
    id: Uuid,
    x: f64,
    y: f64,
    zoom: f64,
) -> Result<Camera, AppError> {
    app.set_camera(id, x, y, zoom)
}

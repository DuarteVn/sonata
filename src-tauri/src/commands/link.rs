use std::sync::Arc;

use sonata_core::model::link::Link;
use sonata_core::{App, AppError};
use tauri::State;
use uuid::Uuid;

#[tauri::command]
pub fn link_create(
    app: State<'_, Arc<App>>,
    workspace_id: Uuid,
    a: Uuid,
    b: Uuid,
) -> Result<Link, AppError> {
    app.create_link(workspace_id, a, b)
}

#[tauri::command]
pub fn link_delete(
    app: State<'_, Arc<App>>,
    workspace_id: Uuid,
    link_id: Uuid,
) -> Result<(), AppError> {
    app.delete_link(workspace_id, link_id)
}

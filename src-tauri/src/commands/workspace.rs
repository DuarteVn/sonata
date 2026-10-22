use std::path::PathBuf;
use std::sync::Arc;

use sonata_core::model::workspace::Workspace;
use sonata_core::workspace::manager::{CreateWorkspace, WorkspaceSummary};
use sonata_core::{App, AppError};
use tauri::State;
use uuid::Uuid;

#[tauri::command]
pub fn workspace_create(
    app: State<'_, Arc<App>>,
    name: String,
    working_directory: PathBuf,
) -> Result<WorkspaceSummary, AppError> {
    app.create_workspace(CreateWorkspace {
        name,
        working_directory,
    })
}

#[tauri::command]
pub fn workspace_open(app: State<'_, Arc<App>>, id: Uuid) -> Result<Workspace, AppError> {
    app.open_workspace(id)
}

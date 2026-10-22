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

#[tauri::command]
pub fn workspace_rename(
    app: State<'_, Arc<App>>,
    id: Uuid,
    name: String,
) -> Result<WorkspaceSummary, AppError> {
    app.rename_workspace(id, &name)
}

#[tauri::command]
pub fn workspace_set_pinned(
    app: State<'_, Arc<App>>,
    id: Uuid,
    pinned: bool,
) -> Result<(), AppError> {
    app.set_pinned(id, pinned)
}

#[tauri::command]
pub fn workspace_reorder(app: State<'_, Arc<App>>, ordered_ids: Vec<Uuid>) -> Result<(), AppError> {
    app.reorder_workspaces(ordered_ids)
}

/// The UI shows a confirmation naming the workspace before calling this (FR-006).
#[tauri::command]
pub fn workspace_delete(
    app: State<'_, Arc<App>>,
    id: Uuid,
    confirmed: bool,
) -> Result<(), AppError> {
    if !confirmed {
        return Err(AppError::internal(
            "Deleting a workspace needs confirmation.",
        ));
    }
    app.delete_workspace(id)
}

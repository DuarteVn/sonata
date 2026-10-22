use std::sync::Arc;

use sonata_core::presets::PresetStatus;
use sonata_core::roles::Role;
use sonata_core::{App, AppError};
use tauri::State;
use uuid::Uuid;

#[tauri::command]
pub fn roles_list(app: State<'_, Arc<App>>) -> Vec<Role> {
    app.roles_list()
}

#[tauri::command]
pub fn role_save(app: State<'_, Arc<App>>, name: String, text: String) -> Result<Role, AppError> {
    app.role_save(&name, &text)
}

#[tauri::command]
pub fn role_delete(app: State<'_, Arc<App>>, id: Uuid) -> Result<(), AppError> {
    app.role_delete(id)
}

/// Runs the login shell once (up to 2 s), so it runs off the main thread.
#[tauri::command]
pub async fn presets_detect(app: State<'_, Arc<App>>) -> Result<Vec<PresetStatus>, AppError> {
    let app = app.inner().clone();
    tauri::async_runtime::spawn_blocking(move || app.presets_detect())
        .await
        .map_err(|e| AppError::internal(e.to_string()))
}

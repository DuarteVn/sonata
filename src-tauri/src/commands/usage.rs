//! Usage meter commands (feature 006). Readings block, so they run on the blocking pool.

use std::sync::Arc;

use sonata_core::usage::schedule::RefreshReason;
use sonata_core::usage::service::UsageState;
use sonata_core::{App, AppError};
use tauri::State;

#[tauri::command]
pub fn usage_state(app: State<'_, Arc<App>>) -> UsageState {
    app.usage_state()
}

#[tauri::command]
pub async fn usage_refresh(
    app: State<'_, Arc<App>>,
    provider_id: Option<String>,
    reason: RefreshReason,
) -> Result<UsageState, AppError> {
    let app = app.inner().clone();
    tauri::async_runtime::spawn_blocking(move || app.usage_refresh(reason, provider_id.as_deref()))
        .await
        .map_err(|e| AppError::internal(e.to_string()))
}

#[tauri::command]
pub fn usage_set_enabled(app: State<'_, Arc<App>>, enabled: bool) -> Result<UsageState, AppError> {
    app.usage_set_enabled(enabled)
}

#[tauri::command]
pub fn usage_set_warnings(app: State<'_, Arc<App>>, enabled: bool) -> Result<UsageState, AppError> {
    app.usage_set_warnings(enabled)
}

#[tauri::command]
pub fn usage_set_provider_enabled(
    app: State<'_, Arc<App>>,
    provider_id: String,
    enabled: bool,
) -> Result<UsageState, AppError> {
    app.usage_set_provider_enabled(&provider_id, enabled)
}

#[tauri::command]
pub fn usage_set_provider_color(
    app: State<'_, Arc<App>>,
    provider_id: String,
    color: String,
) -> Result<UsageState, AppError> {
    app.usage_set_provider_color(&provider_id, &color)
}

#[tauri::command]
pub fn usage_answer_consent(
    app: State<'_, Arc<App>>,
    provider_id: String,
    allow: bool,
    command: Vec<String>,
) -> Result<UsageState, AppError> {
    app.usage_answer_consent(&provider_id, allow, command)
}

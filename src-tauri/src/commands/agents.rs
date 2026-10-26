//! Kimi Code integration commands (feature 010). Status checks and changes start the login
//! shell and may run `kimi doctor`, so they run on the blocking pool.

use std::sync::Arc;

use sonata_core::agents::kimi::KimiIntegration;
use sonata_core::model::preferences::KimiNoticeDismissed;
use sonata_core::{App, AppError};
use tauri::{AppHandle, State};

async fn blocking<T: Send + 'static>(
    app: &State<'_, Arc<App>>,
    f: impl FnOnce(&App) -> Result<T, AppError> + Send + 'static,
) -> Result<T, AppError> {
    let app = app.inner().clone();
    tauri::async_runtime::spawn_blocking(move || f(&app))
        .await
        .map_err(|e| AppError::internal(e.to_string()))?
}

#[tauri::command]
pub async fn kimi_integration(app: State<'_, Arc<App>>) -> Result<KimiIntegration, AppError> {
    blocking(&app, |a| Ok(a.kimi_integration())).await
}

/// The UI asks the user to confirm before calling this (FR-010).
#[tauri::command]
pub async fn kimi_integration_apply(
    app: State<'_, Arc<App>>,
) -> Result<KimiIntegration, AppError> {
    blocking(&app, App::kimi_integration_apply).await
}

/// The UI asks the user to confirm before calling this (FR-010).
#[tauri::command]
pub async fn kimi_integration_remove(
    app: State<'_, Arc<App>>,
) -> Result<KimiIntegration, AppError> {
    blocking(&app, App::kimi_integration_remove).await
}

#[tauri::command]
pub fn dismiss_kimi_notice(
    app: State<'_, Arc<App>>,
    status: KimiNoticeDismissed,
) -> Result<(), AppError> {
    app.dismiss_kimi_notice(status)
}

#[tauri::command]
pub async fn kimi_watch(app: State<'_, Arc<App>>, on: bool) -> Result<(), AppError> {
    blocking(&app, move |a| {
        a.kimi_watch(on);
        Ok(())
    })
    .await
}

/// Opens the Settings window at Agents › Kimi Code (the notice's Set up action).
#[tauri::command]
pub fn kimi_open_settings(handle: AppHandle) -> Result<(), AppError> {
    crate::settings::open_section(&handle, "agents").map_err(|e| AppError::internal(e.to_string()))
}

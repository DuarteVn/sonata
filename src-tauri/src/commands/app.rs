use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use sonata_core::App;
use sonata_core::app::{Bootstrap, BusyWorkspace};
use tauri::{AppHandle, State};

/// Set once the user confirmed quitting, so the exit handler stops intercepting.
#[derive(Default)]
pub struct Quitting(pub AtomicBool);

#[tauri::command]
pub fn app_bootstrap(app: State<'_, Arc<App>>) -> Bootstrap {
    tracing::info!("frontend bootstrapped");
    app.bootstrap()
}

#[tauri::command]
pub fn app_quit_check(app: State<'_, Arc<App>>) -> Vec<BusyWorkspace> {
    app.quit_check()
}

/// Quits after saving. Without `confirmed`, refuses while programs are running (FR-018a).
#[tauri::command]
pub fn app_quit(
    handle: AppHandle,
    app: State<'_, Arc<App>>,
    quitting: State<'_, Quitting>,
    confirmed: bool,
) -> Vec<BusyWorkspace> {
    let busy = app.quit_check();
    if !busy.is_empty() && !confirmed {
        return busy;
    }
    quitting.0.store(true, Ordering::SeqCst);
    app.quit();
    handle.exit(0);
    Vec::new()
}

/// Shows a file of Sonata's data folder in Finder (used for load failures).
#[tauri::command]
pub fn app_reveal_path(
    app: State<'_, Arc<App>>,
    path: std::path::PathBuf,
) -> Result<(), sonata_core::AppError> {
    app.reveal_in_finder(&path)
}

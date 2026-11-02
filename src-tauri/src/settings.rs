//! The Settings window (feature 006): one instance, opened from the app menu or the Usage
//! panel. It loads the same frontend, which renders the settings view for this window.
//!
//! Callers must stay off the main thread: on Windows, building a webview window from a synchronous
//! command or an event handler deadlocks (see `WebviewWindowBuilder::new`). Use an async command, or
//! spawn from a menu handler.

use std::sync::Mutex;

use tauri::{AppHandle, Emitter, Manager, Runtime, WebviewUrl, WebviewWindowBuilder};

pub const LABEL: &str = "settings";

/// Async callers can race; without this, a double click builds the window twice and the second
/// build fails because the label is taken.
static OPENING: Mutex<()> = Mutex::new(());

pub fn open<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    open_at(app, None)
}

/// Opens the window showing `section` (`usage` or `agents`).
pub fn open_section<R: Runtime>(app: &AppHandle<R>, section: &str) -> tauri::Result<()> {
    open_at(app, Some(section))
}

fn open_at<R: Runtime>(app: &AppHandle<R>, section: Option<&str>) -> tauri::Result<()> {
    let _opening = OPENING.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(window) = app.get_webview_window(LABEL) {
        if let Some(section) = section {
            app.emit_to(LABEL, "settings://section", section)?;
        }
        window.unminimize()?;
        window.show()?;
        return window.set_focus();
    }
    WebviewWindowBuilder::new(app, LABEL, WebviewUrl::App("index.html".into()))
        .title("Settings")
        .inner_size(640.0, 540.0)
        .min_inner_size(520.0, 400.0)
        .initialization_script(format!(
            "window.__SONATA_VIEW__ = 'settings'; window.__SONATA_SECTION__ = {};",
            serde_json::to_string(section.unwrap_or("usage")).unwrap_or_default()
        ))
        .build()?
        .set_focus()
}

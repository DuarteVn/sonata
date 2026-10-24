//! The Settings window (feature 006): one instance, opened from the app menu or the Usage
//! panel. It loads the same frontend, which renders the settings view for this window.

use tauri::{AppHandle, Manager, Runtime, WebviewUrl, WebviewWindowBuilder};

pub const LABEL: &str = "settings";

pub fn open<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    if let Some(window) = app.get_webview_window(LABEL) {
        window.unminimize()?;
        window.show()?;
        return window.set_focus();
    }
    WebviewWindowBuilder::new(app, LABEL, WebviewUrl::App("index.html".into()))
        .title("Settings")
        .inner_size(640.0, 540.0)
        .min_inner_size(520.0, 400.0)
        .initialization_script("window.__SONATA_VIEW__ = 'settings';")
        .build()?
        .set_focus()
}

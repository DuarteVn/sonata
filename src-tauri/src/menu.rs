//! App menu. Replaces the default "Close Window" (Cmd+W) with "Close Item", which closes the
//! selected terminal or note instead of the window. Cmd+Q stays Quit. View → Toggle Sidebar
//! (Ctrl+Cmd+S) hides or shows the workspace list.

use tauri::menu::{Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::{AppHandle, Emitter, Runtime};

pub const CLOSE_ITEM_ID: &str = "close-item";
pub const CLOSE_ITEM_EVENT: &str = "app://close-selected";
pub const TOGGLE_SIDEBAR_ID: &str = "toggle-sidebar";
pub const TOGGLE_SIDEBAR_EVENT: &str = "app://toggle-sidebar";

pub fn build<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<Menu<R>> {
    let sep = || PredefinedMenuItem::separator(app);
    let app_menu = Submenu::with_items(
        app,
        "Sonata",
        true,
        &[
            &PredefinedMenuItem::about(app, Some("About Sonata"), None)?,
            &sep()?,
            &PredefinedMenuItem::services(app, None)?,
            &sep()?,
            &PredefinedMenuItem::hide(app, None)?,
            &PredefinedMenuItem::hide_others(app, None)?,
            &PredefinedMenuItem::show_all(app, None)?,
            &sep()?,
            &PredefinedMenuItem::quit(app, Some("Quit Sonata"))?,
        ],
    )?;
    let file = Submenu::with_items(
        app,
        "File",
        true,
        &[&MenuItem::with_id(
            app,
            CLOSE_ITEM_ID,
            "Close Item",
            true,
            Some("CmdOrCtrl+W"),
        )?],
    )?;
    // Needed so copy, paste, and select-all work in text fields and terminals.
    let edit = Submenu::with_items(
        app,
        "Edit",
        true,
        &[
            &PredefinedMenuItem::undo(app, None)?,
            &PredefinedMenuItem::redo(app, None)?,
            &sep()?,
            &PredefinedMenuItem::cut(app, None)?,
            &PredefinedMenuItem::copy(app, None)?,
            &PredefinedMenuItem::paste(app, None)?,
            &PredefinedMenuItem::select_all(app, None)?,
        ],
    )?;
    let view = Submenu::with_items(
        app,
        "View",
        true,
        &[&MenuItem::with_id(
            app,
            TOGGLE_SIDEBAR_ID,
            "Toggle Sidebar",
            true,
            Some("Cmd+Ctrl+S"),
        )?],
    )?;
    let window = Submenu::with_items(
        app,
        "Window",
        true,
        &[
            &PredefinedMenuItem::minimize(app, None)?,
            &PredefinedMenuItem::maximize(app, Some("Zoom"))?,
            &sep()?,
            &PredefinedMenuItem::fullscreen(app, None)?,
        ],
    )?;
    Menu::with_items(app, &[&app_menu, &file, &edit, &view, &window])
}

pub fn on_event<R: Runtime>(app: &AppHandle<R>, id: &str) {
    match id {
        CLOSE_ITEM_ID => {
            let _ = app.emit(CLOSE_ITEM_EVENT, ());
        }
        TOGGLE_SIDEBAR_ID => {
            let _ = app.emit(TOGGLE_SIDEBAR_EVENT, ());
        }
        _ => {}
    }
}

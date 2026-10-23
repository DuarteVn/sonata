//! App menu. Replaces the default "Close Window" (Cmd+W) with "Close Item", which closes the
//! selected terminal or note instead of the window. Cmd+Q stays Quit. View → Toggle Sidebar
//! (Ctrl+Cmd+S) hides or shows the workspace list. The app menu holds the update items
//! (feature 005).

use std::sync::Arc;

use sonata_core::App;
use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::{AppHandle, Emitter, Manager, Runtime};

pub const CLOSE_ITEM_ID: &str = "close-item";
pub const CLOSE_ITEM_EVENT: &str = "app://close-selected";
pub const TOGGLE_SIDEBAR_ID: &str = "toggle-sidebar";
pub const TOGGLE_SIDEBAR_EVENT: &str = "app://toggle-sidebar";
pub const CHECK_UPDATES_ID: &str = "check-updates";
pub const CHECK_UPDATES_EVENT: &str = "app://check-updates";
pub const AUTO_CHECK_ID: &str = "auto-check-updates";

pub fn build<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<Menu<R>> {
    let sep = || PredefinedMenuItem::separator(app);
    let app_menu = Submenu::with_items(
        app,
        "Sonata",
        true,
        &[
            &PredefinedMenuItem::about(app, Some("About Sonata"), None)?,
            &MenuItem::with_id(app, CHECK_UPDATES_ID, "Check for Updates…", true, None::<&str>)?,
            // Set from preferences once core has started (see `set_auto_check`).
            &CheckMenuItem::with_id(
                app,
                AUTO_CHECK_ID,
                "Check for Updates Automatically",
                true,
                true,
                None::<&str>,
            )?,
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
        CHECK_UPDATES_ID => {
            let _ = app.emit(CHECK_UPDATES_EVENT, ());
        }
        AUTO_CHECK_ID => {
            // The menu toggles the check mark itself; save what it now shows.
            let Some(enabled) = auto_check_item(app).and_then(|i| i.is_checked().ok()) else {
                return;
            };
            if let Some(core) = app.try_state::<Arc<App>>()
                && let Err(err) = core.set_auto_update_check(enabled)
            {
                tracing::warn!(error = %err, "could not save the update check preference");
            }
        }
        _ => {}
    }
}

fn auto_check_item<R: Runtime>(app: &AppHandle<R>) -> Option<CheckMenuItem<R>> {
    app.menu()?
        .get(AUTO_CHECK_ID)
        .and_then(|item| item.as_check_menuitem().cloned())
}

/// Shows the automatic update check preference in the menu.
pub fn set_auto_check<R: Runtime>(app: &AppHandle<R>, enabled: bool) {
    if let Some(item) = auto_check_item(app) {
        let _ = item.set_checked(enabled);
    }
}

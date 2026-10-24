//! Sonata desktop shell: wires `sonata_core::App` into Tauri.

mod browser;
mod commands;
mod events;
mod menu;
mod settings;

use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::Duration;

use sonata_core::App;
use sonata_core::paths::DataRoot;
use tauri::{Emitter, Manager, RunEvent, WindowEvent};

use crate::commands::app::Quitting;
use crate::events::TauriEventSink;

const QUIT_REQUESTED: &str = "app://quit-requested";

pub fn run() {
    let app = tauri::Builder::default()
        // A second launch focuses the running app instead (003 FR-031). Registered first.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.unminimize();
                let _ = w.set_focus();
            }
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(Quitting::default())
        .manage(commands::update::PendingUpdate::default())
        .menu(menu::build)
        .on_menu_event(|app, event| menu::on_event(app, event.id().as_ref()))
        .setup(|tauri_app| {
            let root = DataRoot::from_env()?;
            // The webview may read only Sonata's own data folder (note images, research R6).
            tauri_app
                .asset_protocol_scope()
                .allow_directory(root.path(), true)?;
            let log_guard = sonata_core::logging::init(&root)?;
            tauri_app.manage(log_guard);
            let sink = Arc::new(TauriEventSink {
                handle: tauri_app.handle().clone(),
            });
            let options = sonata_core::app::AppOptions {
                shell: None,
                cli_dir: cli_dir(tauri_app.handle()),
                socket: true,
                usage_extra_path: None,
            };
            let data_root = root.path().to_path_buf();
            let core = App::start_with(root, sink, options)?;
            let browsers = browser::Browsers::new(
                tauri_app.handle().clone(),
                Arc::downgrade(&core),
                &data_root,
            );
            core.set_browser_host(Arc::new(browser::Host(browsers.clone())));
            spawn_card_refresh(browsers.clone());
            tauri_app.manage(browsers);
            spawn_autosave(core.clone());
            spawn_summary_monitor(core.clone());
            spawn_launch_window_timer(core.clone());
            spawn_usage_loop(core.clone());
            menu::set_auto_check(tauri_app.handle(), core.auto_update_check());
            commands::update::spawn_background_checks(tauri_app.handle().clone(), core.clone());
            tauri_app.manage(core);
            Ok(())
        })
        .on_window_event(|window, event| {
            // Closing the only window quits the app; ask the UI first (FR-018a).
            if let WindowEvent::CloseRequested { api, .. } = event
                && window.label() == "main"
                && !window.state::<Quitting>().0.load(Ordering::SeqCst)
            {
                api.prevent_close();
                let _ = window.emit(QUIT_REQUESTED, ());
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::app::app_bootstrap,
            commands::app::app_quit_check,
            commands::app::app_quit,
            commands::app::app_reveal_path,
            commands::workspace::workspace_create,
            commands::workspace::workspace_open,
            commands::workspace::workspace_rename,
            commands::workspace::workspace_set_pinned,
            commands::workspace::workspace_reorder,
            commands::workspace::workspace_delete,
            commands::node::node_create_terminal,
            commands::node::node_update_frame,
            commands::node::node_bring_to_front,
            commands::node::node_delete,
            commands::node::workspace_set_camera,
            commands::browser::node_create_browser,
            commands::browser::browser_navigate,
            commands::browser::browser_action,
            commands::browser::browser_layout,
            commands::note::node_create_note,
            commands::note::note_read,
            commands::note::note_write,
            commands::note::note_set_style,
            commands::note::note_insert_image,
            commands::link::link_create,
            commands::link::link_delete,
            commands::role::roles_list,
            commands::role::role_save,
            commands::role::role_delete,
            commands::role::presets_detect,
            commands::terminal::terminal_attach,
            commands::terminal::terminal_detach,
            commands::terminal::terminal_write,
            commands::terminal::terminal_resize,
            commands::terminal::terminal_rename,
            commands::terminal::terminal_set_launch_command,
            commands::terminal::terminal_restart,
            commands::terminal::terminal_set_save_history,
            commands::terminal::terminal_clear_history,
            commands::terminal::terminal_preview_subscribe,
            commands::terminal::terminal_update_settings,
            commands::terminal::exchange_log,
            commands::update::update_state,
            commands::update::update_check,
            commands::update::update_dismiss,
            commands::update::update_download,
            commands::update::update_install,
            commands::update::update_set_auto_check,
            commands::usage::usage_state,
            commands::usage::usage_refresh,
            commands::usage::usage_set_enabled,
            commands::usage::usage_set_warnings,
            commands::usage::usage_set_provider_enabled,
            commands::usage::usage_set_provider_color,
            commands::usage::usage_answer_consent,
            commands::usage::usage_open_settings,
            commands::usage::usage_providers_folder,
            commands::lane::lane_forget,
            commands::lane::lane_remove,
            commands::lane::lane_land,
            commands::lane::lane_preview,
            commands::lane::lane_check,
        ])
        .build(tauri::generate_context!())
        .expect("error while building Sonata");

    app.run(|handle, event| match event {
        // Cmd+Q: let the UI check for running programs unless the user already confirmed.
        RunEvent::ExitRequested { api, code, .. } => {
            if code.is_none() && !handle.state::<Quitting>().0.load(Ordering::SeqCst) {
                api.prevent_exit();
                let _ = handle.emit(QUIT_REQUESTED, ());
            }
        }
        RunEvent::Exit => {
            if let Some(core) = handle.try_state::<Arc<App>>()
                && !handle.state::<Quitting>().0.load(Ordering::SeqCst)
            {
                core.quit();
            }
        }
        _ => {}
    });
}

fn spawn_autosave(core: Arc<App>) {
    let period = sonata_core::autosave::interval(&core.preferences());
    tauri::async_runtime::spawn(async move {
        let mut ticker = tokio::time::interval(period);
        ticker.tick().await;
        loop {
            ticker.tick().await;
            let core = core.clone();
            let _ = tauri::async_runtime::spawn_blocking(move || core.save_dirty()).await;
        }
    });
}

/// Keeps sidebar badges (busy terminal counts, missing folders) current.
fn spawn_summary_monitor(core: Arc<App>) {
    std::thread::Builder::new()
        .name("sonata-summary-monitor".into())
        .spawn(move || {
            let mut last = core.summaries();
            loop {
                std::thread::sleep(Duration::from_secs(1));
                core.notify_if_summaries_changed(&mut last);
            }
        })
        .expect("spawn summary monitor");
}

/// Runs due usage readings (feature 006) on its own thread, so slow agent commands never
/// hold up terminals or the UI (SC-004).
fn spawn_usage_loop(core: Arc<App>) {
    std::thread::Builder::new()
        .name("sonata-usage".into())
        .spawn(move || {
            loop {
                core.usage_tick();
                std::thread::sleep(Duration::from_secs(15));
            }
        })
        .expect("spawn usage loop");
}

/// Marks the launch as healthy once it outlives the early-crash window (FR-027).
fn spawn_launch_window_timer(core: Arc<App>) {
    std::thread::Builder::new()
        .name("sonata-launch-window".into())
        .spawn(move || {
            std::thread::sleep(sonata_core::crash_guard::EARLY_CRASH_WINDOW);
            core.mark_outlived_launch_window();
        })
        .expect("spawn launch window timer");
}

/// Folder with the `sonata` CLI: inside the app bundle, or the Cargo target folder in development.
fn cli_dir(app: &tauri::AppHandle) -> Option<std::path::PathBuf> {
    let bundled = app.path().resource_dir().ok().map(|d| d.join("bin"));
    if let Some(dir) = bundled.filter(|d| d.join("sonata").is_file()) {
        return Some(dir);
    }
    let dev = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../target/debug");
    dev.join("sonata")
        .is_file()
        .then(|| dev.canonicalize().unwrap_or(dev))
}

/// Refreshes the card images of live pages every few seconds (004 FR-008).
fn spawn_card_refresh(browsers: Arc<browser::Browsers>) {
    std::thread::Builder::new()
        .name("sonata-browser-cards".into())
        .spawn(move || {
            loop {
                std::thread::sleep(Duration::from_secs(5));
                browsers.refresh_live_cards();
            }
        })
        .expect("spawn card refresh");
}

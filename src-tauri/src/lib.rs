//! Sonata desktop shell: wires `sonata_core::App` into Tauri.

mod commands;
mod events;

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
        .plugin(tauri_plugin_dialog::init())
        .manage(Quitting::default())
        .setup(|tauri_app| {
            let root = DataRoot::from_env()?;
            let log_guard = sonata_core::logging::init(&root)?;
            tauri_app.manage(log_guard);
            let sink = Arc::new(TauriEventSink {
                handle: tauri_app.handle().clone(),
            });
            let core = App::start(root, sink)?;
            spawn_autosave(core.clone());
            spawn_summary_monitor(core.clone());
            spawn_launch_window_timer(core.clone());
            tauri_app.manage(core);
            Ok(())
        })
        .on_window_event(|window, event| {
            // Closing the only window quits the app; ask the UI first (FR-018a).
            if let WindowEvent::CloseRequested { api, .. } = event
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
            commands::note::node_create_note,
            commands::note::note_read,
            commands::note::note_write,
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

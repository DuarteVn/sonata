//! Sonata desktop shell: wires `sonata_core::App` into Tauri.

mod commands;
mod events;

use std::sync::Arc;

use sonata_core::App;
use sonata_core::paths::DataRoot;
use tauri::{Manager, RunEvent};

use crate::events::TauriEventSink;

pub fn run() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|tauri_app| {
            let root = DataRoot::from_env()?;
            let log_guard = sonata_core::logging::init(&root)?;
            tauri_app.manage(log_guard);
            let sink = Arc::new(TauriEventSink {
                handle: tauri_app.handle().clone(),
            });
            let core = App::start(root, sink)?;
            spawn_autosave(core.clone());
            tauri_app.manage(core);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::app::app_bootstrap,
            commands::workspace::workspace_create,
            commands::workspace::workspace_open,
        ])
        .build(tauri::generate_context!())
        .expect("error while building Sonata");

    app.run(|handle, event| {
        if let RunEvent::Exit = event
            && let Some(core) = handle.try_state::<Arc<App>>()
        {
            core.save_dirty();
        }
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

use std::sync::Arc;

use sonata_core::App;
use sonata_core::app::Bootstrap;
use tauri::State;

#[tauri::command]
pub fn app_bootstrap(app: State<'_, Arc<App>>) -> Bootstrap {
    app.bootstrap()
}

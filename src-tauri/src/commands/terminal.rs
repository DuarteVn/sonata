use std::sync::Arc;

use sonata_core::events::TerminalStateEvent;
use sonata_core::terminal::manager::OutputSink;
use sonata_core::{App, AppError};
use tauri::State;
use tauri::ipc::{Channel, InvokeResponseBody};
use uuid::Uuid;

/// Streams raw PTY bytes to the frontend as `ArrayBuffer`s.
struct ChannelSink(Channel<InvokeResponseBody>);

impl OutputSink for ChannelSink {
    fn send(&self, bytes: Vec<u8>) -> bool {
        self.0.send(InvokeResponseBody::Raw(bytes)).is_ok()
    }
}

#[tauri::command]
pub fn terminal_attach(
    app: State<'_, Arc<App>>,
    node_id: Uuid,
    channel: Channel<InvokeResponseBody>,
    cols: u16,
    rows: u16,
) -> Result<Option<TerminalStateEvent>, AppError> {
    let terminals = app.terminals();
    terminals.resize(node_id, rows, cols)?;
    terminals.attach(node_id, Box::new(ChannelSink(channel)))?;
    Ok(terminals.state_event(node_id))
}

#[tauri::command]
pub fn terminal_detach(app: State<'_, Arc<App>>, node_id: Uuid) {
    app.terminals().detach(node_id);
}

#[tauri::command]
pub fn terminal_write(
    app: State<'_, Arc<App>>,
    node_id: Uuid,
    data: String,
) -> Result<(), AppError> {
    Ok(app.terminals().write(node_id, data.as_bytes())?)
}

#[tauri::command]
pub fn terminal_resize(
    app: State<'_, Arc<App>>,
    node_id: Uuid,
    cols: u16,
    rows: u16,
) -> Result<(), AppError> {
    Ok(app.terminals().resize(node_id, rows, cols)?)
}

#[tauri::command]
pub fn terminal_rename(
    app: State<'_, Arc<App>>,
    node_id: Uuid,
    name: String,
) -> Result<String, AppError> {
    let ws = app.workspace_of_node(node_id)?;
    app.rename_terminal(ws, node_id, &name)
}

#[tauri::command]
pub fn terminal_set_launch_command(
    app: State<'_, Arc<App>>,
    node_id: Uuid,
    launch_command: Option<String>,
) -> Result<(), AppError> {
    let ws = app.workspace_of_node(node_id)?;
    app.set_launch_command(ws, node_id, launch_command)
}

#[tauri::command]
pub fn terminal_restart(
    app: State<'_, Arc<App>>,
    node_id: Uuid,
    confirmed: bool,
) -> Result<(), AppError> {
    app.restart_terminal(node_id, confirmed)
}

/// Forgets the terminal's agent conversation and restarts it (feature 009).
#[tauri::command]
pub fn terminal_start_fresh(
    app: State<'_, Arc<App>>,
    node_id: Uuid,
    confirmed: bool,
) -> Result<(), AppError> {
    app.start_fresh_conversation(node_id, confirmed)
}

#[tauri::command]
pub fn terminal_set_save_history(
    app: State<'_, Arc<App>>,
    node_id: Uuid,
    enabled: bool,
) -> Result<(), AppError> {
    app.set_save_history(node_id, enabled)
}

#[tauri::command]
pub fn terminal_clear_history(app: State<'_, Arc<App>>, node_id: Uuid) -> Result<(), AppError> {
    app.clear_history(node_id)
}

#[tauri::command]
pub fn terminal_preview_subscribe(app: State<'_, Arc<App>>, node_ids: Vec<Uuid>) {
    app.subscribe_previews(node_ids);
}

#[tauri::command]
pub fn terminal_update_settings(
    app: State<'_, Arc<App>>,
    node_id: Uuid,
    settings: sonata_core::app::TerminalSettings,
) -> Result<sonata_core::app::SettingsUpdate, AppError> {
    app.update_terminal_settings(node_id, settings)
}

#[tauri::command]
pub fn exchange_log(
    app: State<'_, Arc<App>>,
    node_id: Uuid,
) -> Vec<sonata_core::messaging::broker::LogEntry> {
    app.exchange_log(node_id)
}

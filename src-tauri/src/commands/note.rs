use std::sync::Arc;

use sonata_core::model::node::{Frame, Node};
use sonata_core::notes::store::NoteView;
use sonata_core::{App, AppError};
use tauri::State;
use uuid::Uuid;

#[tauri::command]
pub fn node_create_note(
    app: State<'_, Arc<App>>,
    workspace_id: Uuid,
    frame: Frame,
) -> Result<Node, AppError> {
    app.create_note(workspace_id, frame)
}

#[tauri::command]
pub fn note_read(app: State<'_, Arc<App>>, node_id: Uuid) -> Result<NoteView, AppError> {
    app.note_read(node_id)
}

/// `flush` writes the file now (on blur); otherwise autosave writes it.
#[tauri::command]
pub fn note_write(
    app: State<'_, Arc<App>>,
    node_id: Uuid,
    text: String,
    flush: bool,
) -> Result<(), AppError> {
    app.note_write(node_id, &text, flush)
}

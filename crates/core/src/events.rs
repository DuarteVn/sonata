//! Events sent from core to the frontend. Names match `contracts/ipc.md`.

use serde::Serialize;
use serde_json::Value;
use uuid::Uuid;

use crate::workspace::manager::WorkspaceSummary;

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum RunState {
    Starting,
    Running,
    Exited,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminalStateEvent {
    pub node_id: Uuid,
    pub run: RunState,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
    pub busy: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warning: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewEvent {
    pub node_id: Uuid,
    pub lines: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<(u16, u16)>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NoteChangedEvent {
    pub node_id: Uuid,
    pub text: String,
    pub sync: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NoteConflictEvent {
    pub node_id: Uuid,
    pub conflict_path: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CoreEvent {
    WorkspacesChanged(Vec<WorkspaceSummary>),
    TerminalState(TerminalStateEvent),
    TerminalPreview(PreviewEvent),
    NoteChanged(NoteChangedEvent),
    NoteConflict(NoteConflictEvent),
    PersistenceWarning(String),
}

impl CoreEvent {
    pub fn name(&self) -> &'static str {
        match self {
            Self::WorkspacesChanged(_) => "workspaces://changed",
            Self::TerminalState(_) => "terminal://state",
            Self::TerminalPreview(_) => "terminal://preview",
            Self::NoteChanged(_) => "note://changed",
            Self::NoteConflict(_) => "note://conflict",
            Self::PersistenceWarning(_) => "persistence://warning",
        }
    }

    pub fn payload(&self) -> Value {
        let v = match self {
            Self::WorkspacesChanged(s) => serde_json::to_value(s),
            Self::TerminalState(e) => serde_json::to_value(e),
            Self::TerminalPreview(e) => serde_json::to_value(e),
            Self::NoteChanged(e) => serde_json::to_value(e),
            Self::NoteConflict(e) => serde_json::to_value(e),
            Self::PersistenceWarning(m) => Ok(serde_json::json!({ "message": m })),
        };
        v.unwrap_or(Value::Null)
    }
}

/// Receives core events. The Tauri shell forwards them to the window; tests collect them.
pub trait EventSink: Send + Sync + 'static {
    fn emit(&self, event: CoreEvent);
}

/// Drops every event. Useful for tests that do not inspect events.
pub struct NullSink;

impl EventSink for NullSink {
    fn emit(&self, _event: CoreEvent) {}
}

/// Records events for tests.
#[derive(Default)]
pub struct RecordingSink {
    events: std::sync::Mutex<Vec<CoreEvent>>,
}

impl RecordingSink {
    pub fn take(&self) -> Vec<CoreEvent> {
        std::mem::take(&mut self.events.lock().expect("events lock"))
    }
}

impl EventSink for RecordingSink {
    fn emit(&self, event: CoreEvent) {
        self.events.lock().expect("events lock").push(event);
    }
}

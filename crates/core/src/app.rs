//! The facade the desktop shell talks to. Owns workspaces, preferences, and the event sink.

use std::sync::{Arc, Mutex, MutexGuard};

use serde::Serialize;
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::events::{CoreEvent, EventSink};
use crate::model::preferences::Preferences;
use crate::model::workspace::Workspace;
use crate::paths::DataRoot;
use crate::persistence::store::{LoadFailure, Store};
use crate::workspace::manager::{CreateWorkspace, SaveReport, WorkspaceManager, WorkspaceSummary};

/// Everything the frontend needs at startup (`app_bootstrap`).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Bootstrap {
    pub workspaces: Vec<WorkspaceSummary>,
    pub load_failures: Vec<LoadFailure>,
    pub active_workspace_id: Option<Uuid>,
    pub crash_guard_tripped: bool,
    pub recent_workspace_ids: Vec<Uuid>,
    pub preferences: Preferences,
    pub warnings: Vec<String>,
}

pub struct App {
    workspaces: Mutex<WorkspaceManager>,
    preferences: Preferences,
    events: Arc<dyn EventSink>,
    startup_warnings: Mutex<Vec<String>>,
}

impl App {
    pub fn start(root: DataRoot, events: Arc<dyn EventSink>) -> std::io::Result<Arc<Self>> {
        let store = Store::new(root);
        let prefs = store.load_preferences();
        let mut manager = WorkspaceManager::load(store)?;
        let mut warnings = manager.take_warnings();
        warnings.extend(prefs.warning);
        Ok(Arc::new(Self {
            workspaces: Mutex::new(manager),
            preferences: prefs.value,
            events,
            startup_warnings: Mutex::new(warnings),
        }))
    }

    pub fn preferences(&self) -> Preferences {
        self.preferences
    }

    pub fn workspaces(&self) -> MutexGuard<'_, WorkspaceManager> {
        self.workspaces.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn emit(&self, event: CoreEvent) {
        self.events.emit(event);
    }

    pub fn bootstrap(&self) -> Bootstrap {
        let m = self.workspaces();
        let active = m
            .app_state()
            .active_workspace_id
            .filter(|id| m.workspace(*id).is_some());
        Bootstrap {
            workspaces: m.summaries(),
            load_failures: m.failures().to_vec(),
            active_workspace_id: active,
            crash_guard_tripped: false,
            recent_workspace_ids: m.app_state().recent_workspace_ids.clone(),
            preferences: self.preferences,
            warnings: std::mem::take(
                &mut self
                    .startup_warnings
                    .lock()
                    .unwrap_or_else(|e| e.into_inner()),
            ),
        }
    }

    pub fn create_workspace(&self, req: CreateWorkspace) -> AppResult<WorkspaceSummary> {
        let summary = self.workspaces().create(req)?;
        self.notify_workspaces_changed();
        Ok(summary)
    }

    pub fn open_workspace(&self, id: Uuid) -> AppResult<Workspace> {
        let ws = self.workspaces().open(id)?.clone();
        self.notify_workspaces_changed();
        Ok(ws)
    }

    pub fn save_dirty(&self) -> SaveReport {
        let report = self.workspaces().save_dirty();
        for (_, err) in &report.errors {
            self.emit(CoreEvent::PersistenceWarning(format!(
                "Could not save: {err}"
            )));
        }
        report
    }

    pub fn notify_workspaces_changed(&self) {
        let summaries = self.workspaces().summaries();
        self.emit(CoreEvent::WorkspacesChanged(summaries));
    }
}

impl From<std::sync::PoisonError<MutexGuard<'_, WorkspaceManager>>> for AppError {
    fn from(_: std::sync::PoisonError<MutexGuard<'_, WorkspaceManager>>) -> Self {
        AppError::internal("internal state lock poisoned")
    }
}

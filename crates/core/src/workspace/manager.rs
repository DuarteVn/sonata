//! In-memory workspaces plus their persistence state. Holds no terminal processes; the
//! [`crate::App`] facade combines this with the terminal manager.

use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};

use chrono::Utc;
use serde::Serialize;
use uuid::Uuid;

use super::palette;
use crate::error::{AppError, AppResult, ErrorCode};
use crate::model::app_state::AppState;
use crate::model::workspace::{Camera, NAME_MAX_CHARS, Workspace};
use crate::persistence::store::{LoadFailure, Store};

#[derive(Debug, Clone)]
pub struct CreateWorkspace {
    pub name: String,
    pub working_directory: PathBuf,
}

/// What the sidebar shows for one workspace (`contracts/ipc.md`).
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceSummary {
    pub id: Uuid,
    pub name: String,
    pub icon: String,
    pub color: String,
    pub working_directory: PathBuf,
    pub working_directory_missing: bool,
    pub is_pinned: bool,
    pub is_loaded: bool,
    pub busy_terminal_count: usize,
}

#[derive(Debug)]
struct Entry {
    ws: Workspace,
    revision: u64,
    saved_revision: u64,
    loaded: bool,
}

#[derive(Debug)]
pub struct WorkspaceManager {
    store: Store,
    entries: HashMap<Uuid, Entry>,
    failures: Vec<LoadFailure>,
    app_state: AppState,
    app_state_dirty: bool,
    warnings: Vec<String>,
}

/// Outcome of a save pass. Errors carry the workspace id (`None` for app state).
#[derive(Debug, Default)]
pub struct SaveReport {
    pub saved: Vec<Uuid>,
    pub errors: Vec<(Option<Uuid>, String)>,
}

impl WorkspaceManager {
    pub fn load(store: Store) -> io::Result<Self> {
        let discovery = store.discover()?;
        let app_state = store.load_app_state();
        let mut warnings: Vec<String> = app_state.warning.into_iter().collect();
        let entries = discovery
            .workspaces
            .into_iter()
            .map(|ws| {
                (
                    ws.id,
                    Entry {
                        ws,
                        revision: 0,
                        saved_revision: 0,
                        loaded: false,
                    },
                )
            })
            .collect();
        warnings.extend(
            discovery
                .failures
                .iter()
                .map(|f| format!("Workspace folder {} could not be loaded.", f.folder_name)),
        );
        Ok(Self {
            store,
            entries,
            failures: discovery.failures,
            app_state: app_state.value,
            app_state_dirty: false,
            warnings,
        })
    }

    pub fn store(&self) -> &Store {
        &self.store
    }

    pub fn failures(&self) -> &[LoadFailure] {
        &self.failures
    }

    pub fn take_warnings(&mut self) -> Vec<String> {
        std::mem::take(&mut self.warnings)
    }

    pub fn app_state(&self) -> &AppState {
        &self.app_state
    }

    pub fn app_state_mut(&mut self) -> &mut AppState {
        self.app_state_dirty = true;
        &mut self.app_state
    }

    pub fn workspace(&self, id: Uuid) -> Option<&Workspace> {
        self.entries.get(&id).map(|e| &e.ws)
    }

    pub fn ids(&self) -> Vec<Uuid> {
        self.entries.keys().copied().collect()
    }

    pub fn is_loaded(&self, id: Uuid) -> bool {
        self.entries.get(&id).is_some_and(|e| e.loaded)
    }

    /// Applies `f` to a workspace and marks it changed.
    pub fn update<R>(
        &mut self,
        id: Uuid,
        f: impl FnOnce(&mut Workspace) -> AppResult<R>,
    ) -> AppResult<R> {
        let entry = self
            .entries
            .get_mut(&id)
            .ok_or_else(|| AppError::workspace_not_found(id))?;
        let out = f(&mut entry.ws)?;
        entry.revision += 1;
        Ok(out)
    }

    pub fn create(&mut self, req: CreateWorkspace) -> AppResult<WorkspaceSummary> {
        let name = self.validate_name(&req.name, None)?;
        validate_directory(&req.working_directory)?;
        let now = Utc::now();
        let ws = Workspace {
            id: Uuid::new_v4(),
            name,
            icon: palette::random_icon(),
            color: palette::random_color(self.entries.values().map(|e| e.ws.color.as_str())),
            working_directory: req.working_directory,
            is_pinned: false,
            created_at: now,
            last_opened_at: now,
            camera: Camera::default(),
            nodes: Vec::new(),
        };
        self.store.save_workspace(&ws)?;
        let id = ws.id;
        self.entries.insert(
            id,
            Entry {
                ws,
                revision: 0,
                saved_revision: 0,
                loaded: false,
            },
        );
        self.app_state_mut().workspace_order.push(id);
        self.save_app_state()?;
        tracing::info!(%id, "workspace created");
        Ok(self.summary(id).expect("just inserted"))
    }

    /// Trimmed, 1–80 characters, unique ignoring case. `except` skips one workspace (rename).
    pub fn validate_name(&self, raw: &str, except: Option<Uuid>) -> AppResult<String> {
        let name = raw.trim();
        if name.is_empty() {
            return Err(AppError::new(
                ErrorCode::NameEmpty,
                "Enter a workspace name.",
            ));
        }
        if name.chars().count() > NAME_MAX_CHARS {
            return Err(AppError::new(
                ErrorCode::NameTooLong,
                format!("Workspace names can have at most {NAME_MAX_CHARS} characters."),
            ));
        }
        let lower = name.to_lowercase();
        let taken = self
            .entries
            .values()
            .any(|e| Some(e.ws.id) != except && e.ws.name.trim().to_lowercase() == lower);
        if taken {
            return Err(AppError::new(
                ErrorCode::NameTaken,
                format!("A workspace named \"{name}\" already exists."),
            ));
        }
        Ok(name.to_string())
    }

    /// Marks a workspace as opened this session and makes it active.
    pub fn open(&mut self, id: Uuid) -> AppResult<&Workspace> {
        let entry = self
            .entries
            .get_mut(&id)
            .ok_or_else(|| AppError::workspace_not_found(id))?;
        entry.loaded = true;
        entry.ws.last_opened_at = Utc::now();
        entry.revision += 1;
        let state = self.app_state_mut();
        state.active_workspace_id = Some(id);
        state.touch_recent(id);
        Ok(&self.entries[&id].ws)
    }

    pub fn set_camera(&mut self, id: Uuid, x: f64, y: f64, zoom: f64) -> AppResult<Camera> {
        self.update(id, |ws| {
            ws.camera = Camera { x, y, zoom }.clamped();
            Ok(ws.camera)
        })
    }

    pub fn summary(&self, id: Uuid) -> Option<WorkspaceSummary> {
        self.entries.get(&id).map(|e| WorkspaceSummary {
            id,
            name: e.ws.name.clone(),
            icon: e.ws.icon.clone(),
            color: e.ws.color.clone(),
            working_directory: e.ws.working_directory.clone(),
            working_directory_missing: !e.ws.working_directory.is_dir(),
            is_pinned: e.ws.is_pinned,
            is_loaded: e.loaded,
            busy_terminal_count: 0,
        })
    }

    /// Sidebar order: pinned first, then `workspaceOrder`; ids missing from the order are
    /// appended by creation time.
    pub fn summaries(&self) -> Vec<WorkspaceSummary> {
        let order = &self.app_state.workspace_order;
        let position = |id: &Uuid| order.iter().position(|x| x == id).unwrap_or(usize::MAX);
        let mut ids: Vec<&Entry> = self.entries.values().collect();
        ids.sort_by(|a, b| {
            b.ws.is_pinned
                .cmp(&a.ws.is_pinned)
                .then(position(&a.ws.id).cmp(&position(&b.ws.id)))
                .then(a.ws.created_at.cmp(&b.ws.created_at))
        });
        ids.iter().filter_map(|e| self.summary(e.ws.id)).collect()
    }

    pub fn dirty_ids(&self) -> Vec<Uuid> {
        let mut ids: Vec<Uuid> = self
            .entries
            .values()
            .filter(|e| e.revision != e.saved_revision)
            .map(|e| e.ws.id)
            .collect();
        ids.sort();
        ids
    }

    pub fn save(&mut self, id: Uuid) -> io::Result<()> {
        let Some(entry) = self.entries.get_mut(&id) else {
            return Ok(());
        };
        let revision = entry.revision;
        self.store.save_workspace(&entry.ws)?;
        entry.saved_revision = revision;
        Ok(())
    }

    pub fn save_app_state(&mut self) -> io::Result<()> {
        self.store.save_app_state(&self.app_state)?;
        self.app_state_dirty = false;
        Ok(())
    }

    /// Saves every changed workspace and the app state if it changed.
    pub fn save_dirty(&mut self) -> SaveReport {
        let mut report = SaveReport::default();
        for id in self.dirty_ids() {
            match self.save(id) {
                Ok(()) => report.saved.push(id),
                Err(e) => {
                    tracing::error!(%id, error = %e, "workspace save failed");
                    report.errors.push((Some(id), e.to_string()));
                }
            }
        }
        if self.app_state_dirty
            && let Err(e) = self.save_app_state()
        {
            tracing::error!(error = %e, "app state save failed");
            report.errors.push((None, e.to_string()));
        }
        report
    }
}

/// The working directory must be an absolute path to an existing folder.
pub fn validate_directory(dir: &Path) -> AppResult<()> {
    if !dir.is_absolute() || !dir.exists() {
        return Err(AppError::new(
            ErrorCode::DirectoryMissing,
            format!("The folder {} does not exist.", dir.display()),
        ));
    }
    if !dir.is_dir() {
        return Err(AppError::new(
            ErrorCode::NotADirectory,
            format!("{} is not a folder.", dir.display()),
        ));
    }
    Ok(())
}

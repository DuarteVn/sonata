//! The facade the desktop shell talks to. Owns workspaces, preferences, and the event sink.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard};

use chrono::Utc;
use serde::Serialize;
use uuid::Uuid;

use crate::crash_guard;
use crate::error::{AppError, AppResult, ErrorCode};
use crate::events::{CoreEvent, EventSink};
use crate::model::node::{Frame, Node, NodeContent, TERMINAL_NAME_MAX_CHARS, TerminalContent};
use crate::model::preferences::Preferences;
use crate::model::workspace::{Camera, Workspace};
use crate::paths::{self, DataRoot};
use crate::persistence::store::{LoadFailure, Store};
use crate::terminal::history;
use crate::terminal::manager::{SpawnSpec, TerminalManager};
use crate::workspace::manager::{CreateWorkspace, SaveReport, WorkspaceManager, WorkspaceSummary};

/// A workspace with programs running, listed in the quit dialog (FR-018a).
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BusyWorkspace {
    pub workspace_id: Uuid,
    pub workspace_name: String,
    pub count: usize,
}

/// Startup options. Tests override the shell; the app uses the user's login shell.
#[derive(Debug, Clone, Default)]
pub struct AppOptions {
    pub shell: Option<PathBuf>,
}

/// Input for `node_create_terminal`.
#[derive(Debug, Clone)]
pub struct NewTerminal {
    pub frame: Frame,
    pub name: Option<String>,
    pub launch_command: Option<String>,
    pub working_directory: Option<PathBuf>,
}

impl Default for NewTerminal {
    fn default() -> Self {
        Self {
            frame: Frame {
                x: 0.0,
                y: 0.0,
                width: 640.0,
                height: 400.0,
            },
            name: None,
            launch_command: None,
            working_directory: None,
        }
    }
}

/// Approximate terminal grid for a node frame, used until a view reports its real size.
pub fn grid_for_frame(frame: &Frame, font_size: u32) -> (u16, u16) {
    let font = f64::from(font_size);
    let cols = ((frame.width - 16.0) / (font * 0.6))
        .floor()
        .clamp(20.0, 500.0) as u16;
    let rows = ((frame.height - 36.0) / (font * 1.3))
        .floor()
        .clamp(5.0, 300.0) as u16;
    (rows, cols)
}

fn clean_optional(value: Option<String>) -> Option<String> {
    value
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

fn terminal_name(raw: &str) -> AppResult<String> {
    let name = raw.trim();
    if name.is_empty() {
        return Err(AppError::new(
            ErrorCode::NameEmpty,
            "Enter a terminal name.",
        ));
    }
    if name.chars().count() > TERMINAL_NAME_MAX_CHARS {
        return Err(AppError::new(
            ErrorCode::NameTooLong,
            format!("Terminal names can have at most {TERMINAL_NAME_MAX_CHARS} characters."),
        ));
    }
    Ok(name.to_string())
}

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
    terminals: TerminalManager,
    preferences: Preferences,
    options: AppOptions,
    events: Arc<dyn EventSink>,
    startup_warnings: Mutex<Vec<String>>,
    crash_guard_tripped: bool,
    /// Output sequence number of each terminal at its last history save.
    history_saved_seq: Mutex<HashMap<Uuid, u64>>,
}

impl App {
    pub fn start(root: DataRoot, events: Arc<dyn EventSink>) -> std::io::Result<Arc<Self>> {
        Self::start_with(root, events, AppOptions::default())
    }

    pub fn start_with(
        root: DataRoot,
        events: Arc<dyn EventSink>,
        options: AppOptions,
    ) -> std::io::Result<Arc<Self>> {
        let mut warnings = Vec::new();
        match paths::enforce_private_tree(root.path()) {
            Ok(0) => {}
            Ok(n) => warnings.push(format!(
                "Fixed permissions on {n} item(s) in {} so only you can read them.",
                root.path().display()
            )),
            Err(e) => warnings.push(format!("Could not check permissions of Sonata's data: {e}")),
        }
        let store = Store::new(root);
        let prefs = store.load_preferences();
        let mut manager = WorkspaceManager::load(store)?;
        warnings.extend(manager.take_warnings());
        warnings.extend(prefs.warning);

        let tripped = crash_guard::on_launch(manager.app_state_mut());
        if let Err(e) = manager.save_app_state() {
            warnings.push(format!("Could not save app state: {e}"));
        }
        if tripped {
            tracing::warn!("crash-loop guard tripped; not restoring the active workspace");
            warnings.push(
                "Sonata crashed twice right after starting, so it did not reopen your last \
                 workspace. Open it from the sidebar when you are ready."
                    .into(),
            );
        }
        Ok(Arc::new(Self {
            workspaces: Mutex::new(manager),
            terminals: TerminalManager::new(events.clone()),
            preferences: prefs.value,
            options,
            events,
            startup_warnings: Mutex::new(warnings),
            crash_guard_tripped: tripped,
            history_saved_seq: Mutex::default(),
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
            .filter(|id| m.workspace(*id).is_some() && !self.crash_guard_tripped);
        let workspaces = self.decorate(m.summaries(), &m);
        Bootstrap {
            workspaces,
            load_failures: m.failures().to_vec(),
            active_workspace_id: active,
            crash_guard_tripped: self.crash_guard_tripped,
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

    pub fn terminals(&self) -> &TerminalManager {
        &self.terminals
    }

    /// Opens a workspace. The first open in a session starts its terminals (Unloaded → Loaded).
    pub fn open_workspace(&self, id: Uuid) -> AppResult<Workspace> {
        let (ws, first_open) = {
            let mut m = self.workspaces();
            let first_open = !m.is_loaded(id);
            (m.open(id)?.clone(), first_open)
        };
        if first_open {
            for node in &ws.nodes {
                if node.terminal().is_some() && !self.terminals.contains(node.id) {
                    self.start_terminal(&ws, node);
                }
            }
        }
        self.notify_workspaces_changed();
        Ok(ws)
    }

    fn spawn_spec(&self, ws: &Workspace, node: &Node) -> Option<SpawnSpec> {
        let term = node.terminal()?;
        let (rows, cols) = grid_for_frame(&node.frame, self.preferences.terminal_font_size);
        Some(SpawnSpec {
            node_id: node.id,
            cwd: term
                .working_directory
                .clone()
                .unwrap_or_else(|| ws.working_directory.clone()),
            launch_command: term.launch_command.clone(),
            rows,
            cols,
            history_max_rows: self.preferences.history_max_rows as usize,
            shell: self.options.shell.clone(),
            restored_history: None,
        })
    }

    fn start_terminal(&self, ws: &Workspace, node: &Node) {
        let Some(mut spec) = self.spawn_spec(ws, node) else {
            return;
        };
        if node.terminal().is_some_and(|t| t.save_history) {
            let path = self.history_path(ws.id, node.id);
            spec.restored_history = history::load(&path, history::MAX_BYTES as u64);
        }
        if let Err(err) = self.terminals.start(spec) {
            tracing::error!(node = %node.id, error = %err, "terminal failed to start");
            self.emit(CoreEvent::PersistenceWarning(format!(
                "A terminal could not start: {err}"
            )));
        }
    }

    fn require_loaded(&self, ws: Uuid) -> AppResult<()> {
        let m = self.workspaces();
        if m.workspace(ws).is_none() {
            return Err(AppError::workspace_not_found(ws));
        }
        if !m.is_loaded(ws) {
            return Err(AppError::new(
                ErrorCode::WorkspaceNotLoaded,
                "Open the workspace first.",
            ));
        }
        Ok(())
    }

    pub fn create_terminal(&self, ws_id: Uuid, req: NewTerminal) -> AppResult<Node> {
        self.require_loaded(ws_id)?;
        let name = match req.name {
            Some(n) if !n.trim().is_empty() => terminal_name(&n)?,
            _ => {
                let m = self.workspaces();
                let count = m.workspace(ws_id).map_or(0, |w| {
                    w.nodes.iter().filter(|n| n.terminal().is_some()).count()
                });
                format!("Terminal {}", count + 1)
            }
        };
        let now = Utc::now();
        let (ws, node) = self.workspaces().update(ws_id, |ws| {
            let mut node = Node {
                id: Uuid::new_v4(),
                frame: req.frame,
                z: ws.max_z() + 1,
                created_at: now,
                modified_at: now,
                content: NodeContent::Terminal(TerminalContent {
                    name,
                    launch_command: clean_optional(req.launch_command),
                    working_directory: req.working_directory,
                    save_history: true,
                }),
            };
            node.frame = node.frame.clamped(node.min_size());
            ws.nodes.push(node.clone());
            Ok((ws.clone(), node))
        })?;
        self.start_terminal(&ws, &node);
        Ok(node)
    }

    fn update_node<R>(
        &self,
        ws: Uuid,
        node: Uuid,
        f: impl FnOnce(&mut Workspace, Uuid) -> AppResult<R>,
    ) -> AppResult<R> {
        self.workspaces().update(ws, |w| {
            if w.node(node).is_none() {
                return Err(AppError::node_not_found(node));
            }
            let out = f(w, node)?;
            if let Some(n) = w.node_mut(node) {
                n.modified_at = Utc::now();
            }
            Ok(out)
        })
    }

    pub fn update_frame(&self, ws: Uuid, node: Uuid, frame: Frame) -> AppResult<Frame> {
        self.update_node(ws, node, |w, id| {
            let n = w.node_mut(id).expect("checked");
            n.frame = frame.clamped(n.min_size());
            Ok(n.frame)
        })
    }

    pub fn bring_to_front(&self, ws: Uuid, node: Uuid) -> AppResult<i64> {
        self.update_node(ws, node, |w, id| {
            let z = w.max_z() + 1;
            w.node_mut(id).expect("checked").z = z;
            Ok(z)
        })
    }

    /// Deletes a node. A busy terminal needs `confirmed` (FR-018).
    pub fn delete_node(&self, ws: Uuid, node: Uuid, confirmed: bool) -> AppResult<()> {
        let is_terminal = {
            let m = self.workspaces();
            let w = m
                .workspace(ws)
                .ok_or_else(|| AppError::workspace_not_found(ws))?;
            w.node(node)
                .ok_or_else(|| AppError::node_not_found(node))?
                .terminal()
                .is_some()
        };
        if is_terminal && !confirmed && self.terminals.is_busy(node) {
            return Err(AppError::new(
                ErrorCode::TerminalBusy,
                "A program is still running in this terminal. Close it anyway?",
            ));
        }
        self.terminals.stop(node);
        self.workspaces().update(ws, |w| {
            w.nodes.retain(|n| n.id != node);
            Ok(())
        })
    }

    pub fn set_camera(&self, ws: Uuid, x: f64, y: f64, zoom: f64) -> AppResult<Camera> {
        self.workspaces().set_camera(ws, x, y, zoom)
    }

    pub fn rename_terminal(&self, ws: Uuid, node: Uuid, name: &str) -> AppResult<String> {
        let name = terminal_name(name)?;
        self.update_node(ws, node, |w, id| {
            let t = w
                .node_mut(id)
                .and_then(Node::terminal_mut)
                .ok_or_else(|| AppError::node_not_found(id))?;
            t.name = name.clone();
            Ok(name.clone())
        })
    }

    pub fn set_launch_command(
        &self,
        ws: Uuid,
        node: Uuid,
        command: Option<String>,
    ) -> AppResult<()> {
        let command = clean_optional(command);
        self.update_node(ws, node, |w, id| {
            let t = w
                .node_mut(id)
                .and_then(Node::terminal_mut)
                .ok_or_else(|| AppError::node_not_found(id))?;
            t.launch_command = command.clone();
            Ok(())
        })?;
        self.terminals.set_launch_command(node, command);
        Ok(())
    }

    /// The workspace that contains `node`.
    pub fn workspace_of_node(&self, node: Uuid) -> AppResult<Uuid> {
        let m = self.workspaces();
        m.ids()
            .into_iter()
            .find(|id| m.workspace(*id).is_some_and(|w| w.node(node).is_some()))
            .ok_or_else(|| AppError::node_not_found(node))
    }

    pub fn restart_terminal(&self, node: Uuid) -> AppResult<()> {
        self.terminals.restart(node)?;
        Ok(())
    }

    pub fn shutdown_terminals(&self) {
        self.terminals.stop_all();
    }

    /// Autosave pass: changed workspaces plus terminal histories with new output.
    pub fn save_dirty(&self) -> SaveReport {
        let mut report = self.workspaces().save_dirty();
        report
            .errors
            .extend(self.save_histories().into_iter().map(|e| (None, e)));
        for (_, err) in &report.errors {
            self.emit(CoreEvent::PersistenceWarning(format!(
                "Could not save: {err}"
            )));
        }
        report
    }

    /// Opens Finder at a path inside the data folder. Paths elsewhere are refused.
    pub fn reveal_in_finder(&self, path: &std::path::Path) -> AppResult<()> {
        let root = self.workspaces().store().root().path().to_path_buf();
        paths::reject_parent_components(path).map_err(|e| AppError::internal(e.to_string()))?;
        if !path.starts_with(&root) {
            return Err(AppError::internal(
                "Only files in Sonata's data folder can be revealed.",
            ));
        }
        std::process::Command::new("/usr/bin/open")
            .arg("-R")
            .arg(path)
            .spawn()?;
        Ok(())
    }

    /// Records that the app outlived the early-crash window (research R9).
    pub fn mark_outlived_launch_window(&self) {
        let mut m = self.workspaces();
        crash_guard::on_outlived_launch_window(m.app_state_mut());
        if let Err(e) = m.save_app_state() {
            tracing::warn!(error = %e, "could not save app state");
        }
    }

    fn history_path(&self, ws: Uuid, node: Uuid) -> PathBuf {
        self.workspaces()
            .store()
            .root()
            .terminals_dir(ws)
            .join(format!("{node}.history"))
    }

    /// Writes history for terminals whose output changed since the last save.
    fn save_histories(&self) -> Vec<String> {
        let targets: Vec<(Uuid, Uuid)> = {
            let m = self.workspaces();
            m.ids()
                .into_iter()
                .filter_map(|id| m.workspace(id))
                .flat_map(|ws| {
                    ws.nodes
                        .iter()
                        .filter(|n| n.terminal().is_some_and(|t| t.save_history))
                        .map(move |n| (ws.id, n.id))
                })
                .collect()
        };
        let max_rows = self.preferences.history_max_rows as usize;
        let mut errors = Vec::new();
        for (ws, node) in targets {
            let Some(seq) = self.terminals.output_seq(node) else {
                continue;
            };
            if self.saved_seq().get(&node) == Some(&seq) {
                continue;
            }
            let Some(rows) = self.terminals.history_rows(node, max_rows) else {
                continue;
            };
            let path = self.history_path(ws, node);
            match history::save(&path, &rows, max_rows, history::MAX_BYTES) {
                Ok(()) => {
                    self.saved_seq().insert(node, seq);
                }
                Err(e) => errors.push(format!("terminal history {}: {e}", path.display())),
            }
        }
        errors
    }

    fn saved_seq(&self) -> MutexGuard<'_, HashMap<Uuid, u64>> {
        self.history_saved_seq
            .lock()
            .unwrap_or_else(|e| e.into_inner())
    }

    pub fn set_save_history(&self, node: Uuid, enabled: bool) -> AppResult<()> {
        let ws = self.workspace_of_node(node)?;
        self.update_node(ws, node, |w, id| {
            let t = w
                .node_mut(id)
                .and_then(Node::terminal_mut)
                .ok_or_else(|| AppError::node_not_found(id))?;
            t.save_history = enabled;
            Ok(())
        })?;
        if !enabled {
            history::delete(&self.history_path(ws, node))?;
            self.saved_seq().remove(&node);
        }
        Ok(())
    }

    /// Deletes the saved history file. It is written again only after new output.
    pub fn clear_history(&self, node: Uuid) -> AppResult<()> {
        let ws = self.workspace_of_node(node)?;
        history::delete(&self.history_path(ws, node))?;
        if let Some(seq) = self.terminals.output_seq(node) {
            self.saved_seq().insert(node, seq);
        }
        Ok(())
    }

    /// Sidebar summaries with live data (busy terminal counts) filled in.
    pub fn summaries(&self) -> Vec<WorkspaceSummary> {
        let m = self.workspaces();
        self.decorate(m.summaries(), &m)
    }

    fn busy_count(&self, ws: &Workspace) -> usize {
        ws.nodes
            .iter()
            .filter(|n| n.terminal().is_some() && self.terminals.is_busy(n.id))
            .count()
    }

    fn decorate(
        &self,
        mut list: Vec<WorkspaceSummary>,
        m: &WorkspaceManager,
    ) -> Vec<WorkspaceSummary> {
        for s in &mut list {
            if let Some(ws) = m.workspace(s.id) {
                s.busy_terminal_count = self.busy_count(ws);
            }
        }
        list
    }

    pub fn notify_workspaces_changed(&self) {
        self.emit(CoreEvent::WorkspacesChanged(self.summaries()));
    }

    /// Emits `workspaces://changed` only when the summaries differ from the last call.
    pub fn notify_if_summaries_changed(&self, last: &mut Vec<WorkspaceSummary>) {
        let now = self.summaries();
        if *last != now {
            *last = now.clone();
            self.emit(CoreEvent::WorkspacesChanged(now));
        }
    }

    pub fn rename_workspace(&self, id: Uuid, name: &str) -> AppResult<WorkspaceSummary> {
        let summary = self.workspaces().rename(id, name)?;
        self.notify_workspaces_changed();
        Ok(summary)
    }

    pub fn set_pinned(&self, id: Uuid, pinned: bool) -> AppResult<()> {
        self.workspaces().set_pinned(id, pinned)?;
        self.notify_workspaces_changed();
        Ok(())
    }

    pub fn reorder_workspaces(&self, ordered: Vec<Uuid>) -> AppResult<()> {
        self.workspaces().reorder(ordered);
        self.notify_workspaces_changed();
        Ok(())
    }

    /// Stops the workspace's terminals and deletes its Sonata data (FR-006). The UI asks for
    /// confirmation first.
    pub fn delete_workspace(&self, id: Uuid) -> AppResult<()> {
        let node_ids: Vec<Uuid> = {
            let m = self.workspaces();
            let ws = m
                .workspace(id)
                .ok_or_else(|| AppError::workspace_not_found(id))?;
            ws.nodes.iter().map(|n| n.id).collect()
        };
        for node in node_ids {
            self.terminals.stop(node);
        }
        self.workspaces().delete(id)?;
        self.notify_workspaces_changed();
        Ok(())
    }

    /// History files of terminals that have history saving turned off.
    fn history_files_to_clean(&self) -> Vec<(Uuid, PathBuf)> {
        let m = self.workspaces();
        let root = m.store().root().clone();
        m.ids()
            .into_iter()
            .filter_map(|id| m.workspace(id))
            .flat_map(|ws| {
                let root = root.clone();
                ws.nodes
                    .iter()
                    .filter(|n| n.terminal().is_some_and(|t| !t.save_history))
                    .map(move |n| {
                        (
                            n.id,
                            root.terminals_dir(ws.id).join(format!("{}.history", n.id)),
                        )
                    })
            })
            .collect()
    }

    /// Workspaces with terminals running something other than an idle shell (FR-018a).
    pub fn quit_check(&self) -> Vec<BusyWorkspace> {
        let m = self.workspaces();
        m.summaries()
            .into_iter()
            .filter_map(|s| {
                let ws = m.workspace(s.id)?;
                let count = self.busy_count(ws);
                (count > 0).then_some(BusyWorkspace {
                    workspace_id: s.id,
                    workspace_name: s.name,
                    count,
                })
            })
            .collect()
    }

    /// Saves everything, marks a clean shutdown, and stops all terminals.
    pub fn quit(&self) {
        self.save_dirty();
        for (_, path) in self.history_files_to_clean() {
            let _ = history::delete(&path);
        }
        {
            let mut m = self.workspaces();
            m.app_state_mut().clean_shutdown = true;
            if let Err(err) = m.save_app_state() {
                tracing::error!(error = %err, "could not save app state on quit");
            }
        }
        self.terminals.stop_all();
    }
}

impl From<std::sync::PoisonError<MutexGuard<'_, WorkspaceManager>>> for AppError {
    fn from(_: std::sync::PoisonError<MutexGuard<'_, WorkspaceManager>>) -> Self {
        AppError::internal("internal state lock poisoned")
    }
}

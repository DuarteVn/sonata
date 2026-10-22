//! The facade the desktop shell talks to. Owns workspaces, preferences, and the event sink.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard, Weak};

use chrono::Utc;
use serde::Serialize;
use uuid::Uuid;

use crate::crash_guard;
use crate::error::{AppError, AppResult, ErrorCode};
use crate::events::{CoreEvent, EventSink};
use crate::model::link::Link;
use crate::model::node::NOTE_TEXT_SIZES;
use crate::model::node::{
    FONT_SIZE_RANGE, Frame, Node, NodeContent, ROLE_TEXT_MAX_CHARS, RoleRef,
    TERMINAL_NAME_MAX_CHARS, TerminalContent,
};
use crate::model::node::{NOTE_MIN_SIZE, NoteContent};
use crate::model::preferences::Preferences;
use crate::model::workspace::{Camera, Workspace};
use crate::notes::images;
use crate::notes::store::{NoteStore, NoteView};
use crate::notes::watcher::{self, NotesWatcher};
use crate::paths::{self, DataRoot};
use crate::persistence::store::{LoadFailure, Store};
use crate::presets::{self, PresetStatus};
use crate::roles::{Role, RoleLibrary};
use crate::terminal::activity::ACTIVITY_INTERVAL;
use crate::terminal::history;
use crate::terminal::manager::{SpawnSpec, TerminalManager};
use crate::terminal::preview::{PREVIEW_INTERVAL, PreviewState};
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
    pub settings: TerminalSettings,
}

/// Terminal options from the New/Edit Terminal dialog. For nullable fields, `None` leaves the
/// value unchanged and `Some(None)` clears it.
#[derive(Debug, Clone, Default, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminalSettings {
    pub name: Option<String>,
    #[serde(default, deserialize_with = "double_option")]
    pub launch_command: Option<Option<String>>,
    #[serde(default, deserialize_with = "double_option")]
    pub working_directory: Option<Option<PathBuf>>,
    #[serde(default, deserialize_with = "double_option")]
    pub accent_color: Option<Option<String>>,
    #[serde(default, deserialize_with = "double_option")]
    pub font_size: Option<Option<u32>>,
    #[serde(default, deserialize_with = "double_option")]
    pub role: Option<Option<RoleRef>>,
    pub monitor_activity: Option<bool>,
    pub orchestrator: Option<bool>,
}

/// Absent → `None`; `null` → `Some(None)`; value → `Some(Some(v))`.
fn double_option<'de, T, D>(de: D) -> Result<Option<Option<T>>, D::Error>
where
    T: serde::Deserialize<'de>,
    D: serde::Deserializer<'de>,
{
    <Option<T> as serde::Deserialize>::deserialize(de).map(Some)
}

/// Result of `terminal_update_settings`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsUpdate {
    pub node: Node,
    /// Command or folder changed while running: they apply at the next start or restart.
    pub applies_on_restart: bool,
}

/// Validates and applies settings. Returns true when the command or folder changed.
fn apply_settings(t: &mut TerminalContent, s: &TerminalSettings) -> AppResult<bool> {
    if let Some(name) = &s.name {
        t.name = terminal_name(name)?;
    }
    let before = (t.launch_command.clone(), t.working_directory.clone());
    if let Some(cmd) = &s.launch_command {
        t.launch_command = clean_optional(cmd.clone());
    }
    if let Some(dir) = &s.working_directory {
        if let Some(d) = dir {
            crate::workspace::manager::validate_directory(d)?;
        }
        t.working_directory = dir.clone();
    }
    if let Some(color) = &s.accent_color {
        if let Some(c) = color
            && !crate::workspace::palette::COLORS.contains(&c.as_str())
        {
            return Err(AppError::new(
                ErrorCode::Internal,
                format!("Unknown accent color {c}."),
            ));
        }
        t.accent_color = color.clone();
    }
    if let Some(size) = s.font_size {
        if let Some(n) = size
            && !FONT_SIZE_RANGE.contains(&n)
        {
            return Err(AppError::new(
                ErrorCode::Internal,
                "Font size must be between 9 and 24.",
            ));
        }
        t.font_size = size;
    }
    if let Some(role) = &s.role {
        t.role = match role {
            Some(r) if !r.text.trim().is_empty() => {
                if r.text.chars().count() > ROLE_TEXT_MAX_CHARS {
                    return Err(AppError::new(
                        ErrorCode::NameTooLong,
                        "Roles can have at most 20,000 characters.",
                    ));
                }
                Some(r.clone())
            }
            _ => None,
        };
    }
    if let Some(v) = s.monitor_activity {
        t.monitor_activity = v;
    }
    if let Some(v) = s.orchestrator {
        t.orchestrator = v;
    }
    Ok(before != (t.launch_command.clone(), t.working_directory.clone()))
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
            settings: TerminalSettings::default(),
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
    notes: Mutex<NoteStore>,
    note_watchers: Mutex<HashMap<Uuid, NotesWatcher>>,
    previews: Mutex<PreviewState>,
    roles: Mutex<RoleLibrary>,
    activity_last: Mutex<HashMap<Uuid, bool>>,
    this: Weak<App>,
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
        let notes = NoteStore::new(root.clone());
        let (roles, roles_warning) = RoleLibrary::load(root.path().join("roles.json"));
        warnings.extend(roles_warning);
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
        Ok(Arc::new_cyclic(|this| Self {
            workspaces: Mutex::new(manager),
            terminals: TerminalManager::new(events.clone()),
            preferences: prefs.value,
            options,
            events,
            startup_warnings: Mutex::new(warnings),
            crash_guard_tripped: tripped,
            history_saved_seq: Mutex::default(),
            notes: Mutex::new(notes),
            note_watchers: Mutex::default(),
            previews: Mutex::default(),
            roles: Mutex::new(roles),
            activity_last: Mutex::default(),
            this: this.clone(),
        }))
        .inspect(|app| {
            app.spawn_preview_loop();
            app.spawn_activity_loop();
        })
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
            self.watch_notes(id);
        }
        self.notify_workspaces_changed();
        Ok(ws)
    }

    fn spawn_spec(&self, ws: &Workspace, node: &Node) -> Option<SpawnSpec> {
        let term = node.terminal()?;
        let font = term
            .font_size
            .unwrap_or(self.preferences.terminal_font_size);
        let (rows, cols) = grid_for_frame(&node.frame, font);
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
                content: NodeContent::Terminal({
                    let mut t = TerminalContent::new(name);
                    t.launch_command = clean_optional(req.launch_command);
                    t.working_directory = req.working_directory;
                    let mut settings = req.settings;
                    settings.name = None;
                    apply_settings(&mut t, &settings)?;
                    t
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
        let (is_terminal, is_note) = {
            let m = self.workspaces();
            let w = m
                .workspace(ws)
                .ok_or_else(|| AppError::workspace_not_found(ws))?;
            let n = w.node(node).ok_or_else(|| AppError::node_not_found(node))?;
            (n.terminal().is_some(), n.is_note())
        };
        if is_terminal && !confirmed && self.terminals.is_busy(node) {
            return Err(AppError::new(
                ErrorCode::TerminalBusy,
                "A program is still running in this terminal. Close it anyway?",
            ));
        }
        self.terminals.stop(node);
        if is_note {
            self.note_store().delete(ws, node)?;
        }
        self.workspaces().update(ws, |w| {
            w.nodes.retain(|n| n.id != node);
            w.links.retain(|l| !l.touches(node));
            Ok(())
        })
    }

    /// Sets which terminals get text previews (the UI sends the ones near the viewport that
    /// are not live). New ones get a preview at once, then at most one per second.
    pub fn subscribe_previews(&self, ids: Vec<Uuid>) {
        let events = {
            let mut p = self.previews.lock().unwrap_or_else(|e| e.into_inner());
            let added = p.set(ids);
            p.due(&self.terminals, Some(&added))
        };
        for e in events {
            self.emit(CoreEvent::TerminalPreview(e));
        }
    }

    fn spawn_preview_loop(&self) {
        let this = self.this.clone();
        std::thread::Builder::new()
            .name("sonata-previews".into())
            .spawn(move || {
                loop {
                    std::thread::sleep(PREVIEW_INTERVAL);
                    let Some(app) = this.upgrade() else { break };
                    let events = app
                        .previews
                        .lock()
                        .unwrap_or_else(|e| e.into_inner())
                        .due(&app.terminals, None);
                    for e in events {
                        app.emit(CoreEvent::TerminalPreview(e));
                    }
                }
            })
            .expect("spawn preview thread");
    }

    // ---- Links (feature 002) ----

    /// Links two different nodes of a workspace. Undirected and unique per pair.
    pub fn create_link(&self, ws: Uuid, x: Uuid, y: Uuid) -> AppResult<Link> {
        if x == y {
            return Err(AppError::new(
                ErrorCode::LinkSelf,
                "A node cannot be linked to itself.",
            ));
        }
        self.workspaces().update(ws, |w| {
            for id in [x, y] {
                if w.node(id).is_none() {
                    return Err(AppError::node_not_found(id));
                }
            }
            if w.links.iter().any(|l| l.joins(x, y)) {
                return Err(AppError::new(
                    ErrorCode::LinkExists,
                    "These two are already linked.",
                ));
            }
            let link = Link::new(x, y);
            w.links.push(link.clone());
            Ok(link)
        })
    }

    pub fn delete_link(&self, ws: Uuid, link: Uuid) -> AppResult<()> {
        self.workspaces().update(ws, |w| {
            w.links.retain(|l| l.id != link);
            Ok(())
        })
    }

    pub fn links_of(&self, ws: Uuid, node: Uuid) -> Vec<Link> {
        self.workspaces()
            .workspace(ws)
            .map(|w| {
                w.links
                    .iter()
                    .filter(|l| l.touches(node))
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }

    // ---- Roles and presets (feature 002) ----

    fn role_library(&self) -> MutexGuard<'_, RoleLibrary> {
        self.roles.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn roles_list(&self) -> Vec<Role> {
        self.role_library().list()
    }

    pub fn role_save(&self, name: &str, text: &str) -> AppResult<Role> {
        self.role_library().save(name, text)
    }

    pub fn role_delete(&self, id: Uuid) -> AppResult<()> {
        self.role_library().delete(id)
    }

    pub fn presets_detect(&self) -> Vec<PresetStatus> {
        let shell = self
            .options
            .shell
            .clone()
            .unwrap_or_else(crate::terminal::pty::login_shell);
        presets::detect(&shell, presets::DETECT_TIMEOUT)
    }

    // ---- Note style and images (feature 002) ----

    pub fn note_set_style(&self, node: Uuid, style: NoteContent) -> AppResult<()> {
        if !NOTE_TEXT_SIZES.contains(&style.text_size) {
            return Err(AppError::new(
                ErrorCode::Internal,
                "Text size must be between 1 and 5.",
            ));
        }
        let ws = self.workspace_of_node(node)?;
        self.update_node(ws, node, |w, id| {
            let n = w
                .node_mut(id)
                .and_then(Node::note_style_mut)
                .ok_or_else(|| AppError::node_not_found(id))?;
            *n = style.clone();
            Ok(())
        })
    }

    /// Copies a picture next to the workspace's notes and returns the Markdown to insert.
    pub fn note_insert_image(&self, node: Uuid, source: &std::path::Path) -> AppResult<String> {
        let ws = self.workspace_of_node(node)?;
        let root = self.workspaces().store().root().clone();
        images::insert(&root, ws, source)
    }

    // ---- Activity (feature 002) ----

    /// Emits `terminal://state` when a monitored terminal switches between busy and idle.
    fn spawn_activity_loop(&self) {
        let this = self.this.clone();
        std::thread::Builder::new()
            .name("sonata-activity".into())
            .spawn(move || {
                loop {
                    std::thread::sleep(ACTIVITY_INTERVAL);
                    let Some(app) = this.upgrade() else { break };
                    app.check_activity();
                }
            })
            .expect("spawn activity thread");
    }

    fn check_activity(&self) {
        let monitored: Vec<Uuid> = {
            let m = self.workspaces();
            m.ids()
                .into_iter()
                .filter(|id| m.is_loaded(*id))
                .filter_map(|id| m.workspace(id))
                .flat_map(|w| {
                    w.nodes
                        .iter()
                        .filter(|n| n.terminal().is_some_and(|t| t.monitor_activity))
                        .map(|n| n.id)
                })
                .collect()
        };
        let mut changed = Vec::new();
        {
            let mut last = self.activity_last.lock().unwrap_or_else(|e| e.into_inner());
            last.retain(|id, _| monitored.contains(id));
            for id in monitored {
                let busy = self.terminals.is_busy(id);
                if last.insert(id, busy).is_some_and(|prev| prev != busy) {
                    changed.push(id);
                }
            }
        }
        for id in changed {
            if let Some(e) = self.terminals.state_event(id) {
                self.emit(CoreEvent::TerminalState(e));
            }
        }
    }

    fn note_store(&self) -> MutexGuard<'_, NoteStore> {
        self.notes.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn create_note(&self, ws_id: Uuid, frame: Frame) -> AppResult<Node> {
        self.require_loaded(ws_id)?;
        let now = Utc::now();
        let node = Node {
            id: Uuid::new_v4(),
            frame: frame.clamped(NOTE_MIN_SIZE),
            z: 0,
            created_at: now,
            modified_at: now,
            content: NodeContent::Note(NoteContent::default()),
        };
        self.note_store().create(ws_id, node.id)?;
        self.workspaces().update(ws_id, |ws| {
            let mut node = node.clone();
            node.z = ws.max_z() + 1;
            ws.nodes.push(node.clone());
            Ok(node)
        })
    }

    pub fn note_read(&self, node: Uuid) -> AppResult<NoteView> {
        let ws = self.workspace_of_node(node)?;
        Ok(self.note_store().read(ws, node))
    }

    /// Updates a note's text; with `flush` it is written now, otherwise at the next save.
    pub fn note_write(&self, node: Uuid, text: &str, flush: bool) -> AppResult<()> {
        let ws = self.workspace_of_node(node)?;
        self.note_store().write(ws, node, text, flush)?;
        Ok(())
    }

    /// Starts watching a loaded workspace's notes folder for outside edits (FR-020a).
    fn watch_notes(&self, ws: Uuid) {
        let dir = self.workspaces().store().root().notes_dir(ws);
        if let Err(e) = paths::ensure_private_dir(&dir) {
            tracing::warn!(error = %e, "could not create notes folder");
            return;
        }
        let this = self.this.clone();
        let result = watcher::watch(&dir, move |node| {
            let Some(app) = this.upgrade() else { return };
            let event = app.note_store().external_change(ws, node);
            if let Some(event) = event {
                app.emit(event);
            }
        });
        match result {
            Ok(w) => {
                self.note_watchers
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .insert(ws, w);
            }
            Err(e) => tracing::warn!(error = %e, "could not watch notes folder"),
        }
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

    /// Restarts a terminal with its current settings. A busy terminal needs `confirmed`.
    pub fn restart_terminal(&self, node: Uuid, confirmed: bool) -> AppResult<()> {
        if !confirmed && self.terminals.is_busy(node) {
            return Err(AppError::new(
                ErrorCode::TerminalBusy,
                "A program is still running in this terminal. Restart it anyway?",
            ));
        }
        self.terminals.restart(node)?;
        Ok(())
    }

    pub fn update_terminal_settings(
        &self,
        node: Uuid,
        settings: TerminalSettings,
    ) -> AppResult<SettingsUpdate> {
        let ws = self.workspace_of_node(node)?;
        let (updated, spawn_changed, default_dir) = self.update_node(ws, node, |w, id| {
            let default_dir = w.working_directory.clone();
            let n = w.node_mut(id).expect("checked");
            let t = n
                .terminal_mut()
                .ok_or_else(|| AppError::node_not_found(id))?;
            let changed = apply_settings(t, &settings)?;
            Ok((n.clone(), changed, default_dir))
        })?;
        let t = updated.terminal().expect("terminal");
        let cwd = t.working_directory.clone().unwrap_or(default_dir);
        self.terminals
            .set_spawn_settings(node, t.launch_command.clone(), cwd);
        let running = matches!(
            self.terminals.status(node),
            Some(crate::terminal::manager::RunStatus::Running { .. })
        );
        Ok(SettingsUpdate {
            node: updated,
            applies_on_restart: spawn_changed && running,
        })
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
        let note_errors = self.note_store().save_dirty();
        report
            .errors
            .extend(note_errors.into_iter().map(|e| (None, e)));
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
        self.note_watchers
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(&id);
        self.note_store().forget_workspace(id);
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
        self.note_watchers
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clear();
    }
}

impl From<std::sync::PoisonError<MutexGuard<'_, WorkspaceManager>>> for AppError {
    fn from(_: std::sync::PoisonError<MutexGuard<'_, WorkspaceManager>>) -> Self {
        AppError::internal("internal state lock poisoned")
    }
}

//! The facade the desktop shell talks to. Owns workspaces, preferences, and the event sink.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard, Weak};
use std::time::Instant;

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
use crate::update::{UpdateSchedule, UpdateState};
use crate::usage::schedule::RefreshReason;
use crate::usage::service::{UsageService, UsageState};
use crate::usage::system::SystemBackend;
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
    /// Folder holding the `sonata` CLI; put first on every terminal's PATH (feature 003).
    pub cli_dir: Option<PathBuf>,
    /// Start the CLI socket server (feature 003). Off in tests that do not need it.
    pub socket: bool,
    /// Folder put first on `PATH` when looking for agent commands (feature 006). Tests point
    /// it at fake `claude`/`codex` scripts.
    pub usage_extra_path: Option<PathBuf>,
}

/// The choices of the remove-lane confirmation (FR-014, FR-015).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LaneRemoval {
    pub discard_changes: bool,
    pub delete_branch: bool,
}

/// Where a recruited terminal runs (feature 008 adds the lane).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecruitPlace {
    WorkspaceFolder,
    Folder(PathBuf),
    Lane,
}

/// Input for `node_create_terminal`.
#[derive(Debug, Clone)]
pub struct NewTerminal {
    pub frame: Frame,
    pub name: Option<String>,
    pub launch_command: Option<String>,
    pub working_directory: Option<PathBuf>,
    pub settings: TerminalSettings,
    /// Run in a new lane (feature 008); `working_directory` is then ignored.
    pub lane: bool,
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
    #[serde(default, deserialize_with = "double_option")]
    pub send_instructions: Option<Option<bool>>,
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
    if let Some(v) = s.send_instructions {
        t.send_instructions = v;
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
            lane: false,
        }
    }
}

fn lane_info(lane: &crate::model::node::Lane) -> crate::messaging::instructions::LaneInfo<'_> {
    crate::messaging::instructions::LaneInfo {
        branch: &lane.branch,
        base_branch: &lane.base_branch,
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
    broker: crate::messaging::broker::Broker,
    server: Mutex<Option<crate::messaging::server::Server>>,
    socket_path: Option<PathBuf>,
    data_root: DataRoot,
    browser_host: Mutex<Option<Arc<dyn crate::browser::BrowserHost>>>,
    /// Update schedule and banner state (feature 005).
    updates: Mutex<(UpdateSchedule, UpdateState)>,
    /// Usage meter (feature 006).
    usage: Arc<UsageService>,
    /// The system git, found on first use (feature 008, research R2).
    git: std::sync::OnceLock<Result<crate::lanes::git::Git, String>>,
    lane_statuses: Mutex<crate::lanes::status::StatusCache>,
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
        let data_root = root.clone();
        let socket_path = if options.socket {
            Some(crate::messaging::server::socket_path(root.path())?)
        } else {
            None
        };
        let (roles, roles_warning) = RoleLibrary::load(root.path().join("roles.json"));
        warnings.extend(roles_warning);
        let store = Store::new(root);
        let prefs = store.load_preferences();
        let mut manager = WorkspaceManager::load(store)?;
        warnings.extend(manager.take_warnings());
        warnings.extend(prefs.warning);

        let usage = Arc::new(UsageService::new(
            data_root.clone(),
            prefs.value.usage.clone(),
            Arc::new(SystemBackend {
                shell: options
                    .shell
                    .clone()
                    .unwrap_or_else(crate::terminal::pty::login_shell),
                extra_path: options.usage_extra_path.clone(),
                root: data_root.clone(),
            }),
            events.clone(),
        ));
        warnings.extend(usage.take_warnings());

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
            preferences: prefs.value.clone(),
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
            broker: crate::messaging::broker::Broker::default(),
            server: Mutex::new(None),
            socket_path,
            data_root,
            browser_host: Mutex::new(None),
            updates: Mutex::new((
                UpdateSchedule::new(prefs.value.auto_update_check),
                UpdateState::default(),
            )),
            usage,
            git: std::sync::OnceLock::new(),
            lane_statuses: Mutex::default(),
        }))
        .inspect(|app| {
            app.spawn_preview_loop();
            app.spawn_activity_loop();
            app.spawn_lane_status_loop();
            app.start_messaging();
        })
    }

    pub fn preferences(&self) -> Preferences {
        self.preferences.clone()
    }

    fn updates(&self) -> MutexGuard<'_, (UpdateSchedule, UpdateState)> {
        self.updates.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn update_state(&self) -> UpdateState {
        self.updates().1.clone()
    }

    /// Applies one change to the update state and emits the result when it changed.
    pub fn update(&self, change: impl FnOnce(&mut UpdateState)) -> UpdateState {
        let (before, after) = {
            let mut guard = self.updates();
            let before = guard.1.clone();
            change(&mut guard.1);
            (before, guard.1.clone())
        };
        if before != after {
            self.events.emit(CoreEvent::UpdateState(after.clone()));
        }
        after
    }

    pub fn auto_update_check(&self) -> bool {
        self.updates().0.enabled()
    }

    pub fn update_check_due(&self, now: Instant) -> bool {
        self.updates().0.due(now)
    }

    pub fn mark_update_checked(&self, now: Instant) {
        self.updates().0.mark_checked(now);
    }

    /// Turns automatic update checks on or off and saves the choice in preferences.
    /// `self.preferences` keeps its startup copy; only the schedule reads this flag later.
    pub fn set_auto_update_check(&self, enabled: bool) -> std::io::Result<()> {
        let store = Store::new(self.data_root.clone());
        let mut prefs = store.load_preferences().value;
        prefs.auto_update_check = enabled;
        store.save_preferences(&prefs)?;
        self.updates().0.set_enabled(enabled);
        Ok(())
    }

    pub fn usage_state(&self) -> UsageState {
        self.usage.state()
    }

    /// Runs due automatic usage readings; blocks while they run (call from a worker thread).
    pub fn usage_tick(&self) {
        self.usage.tick();
    }

    /// Blocks while the readings run.
    pub fn usage_refresh(&self, reason: RefreshReason, provider: Option<&str>) -> UsageState {
        self.usage.refresh(reason, provider)
    }

    pub fn usage_set_enabled(&self, enabled: bool) -> AppResult<UsageState> {
        let state = self.usage.set_enabled(enabled)?;
        if enabled {
            self.usage_refresh_in_background(RefreshReason::Auto, None);
        }
        Ok(state)
    }

    pub fn usage_set_warnings(&self, enabled: bool) -> AppResult<UsageState> {
        self.usage.set_warnings(enabled)
    }

    pub fn usage_set_provider_enabled(&self, id: &str, enabled: bool) -> AppResult<UsageState> {
        let state = self.usage.set_provider_enabled(id, enabled)?;
        if enabled {
            self.usage_refresh_in_background(RefreshReason::Manual, Some(id.to_string()));
        }
        Ok(state)
    }

    pub fn usage_set_provider_color(&self, id: &str, color: &str) -> AppResult<UsageState> {
        self.usage.set_provider_color(id, color)
    }

    /// Records a consent answer; an allowed provider is read right away in the background.
    pub fn usage_answer_consent(
        &self,
        id: &str,
        allow: bool,
        command: Vec<String>,
    ) -> AppResult<UsageState> {
        let state = self.usage.answer_consent(id, allow, command)?;
        if allow {
            self.usage_refresh_in_background(RefreshReason::Manual, Some(id.to_string()));
        }
        Ok(state)
    }

    /// The custom usage providers folder (feature 006), created when missing.
    pub fn usage_providers_folder(&self) -> AppResult<PathBuf> {
        let dir = self.data_root.usage_providers_dir();
        crate::usage::custom::ensure_folder(&dir)?;
        Ok(dir)
    }

    fn usage_refresh_in_background(&self, reason: RefreshReason, provider: Option<String>) {
        let usage = self.usage.clone();
        std::thread::spawn(move || {
            usage.refresh(reason, provider.as_deref());
        });
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
            preferences: self.preferences.clone(),
            warnings: std::mem::take(
                &mut self
                    .startup_warnings
                    .lock()
                    .unwrap_or_else(|e| e.into_inner()),
            ),
        }
    }

    /// The system git, or why lanes are unavailable (research R2).
    pub fn git(&self) -> Result<&crate::lanes::git::Git, &str> {
        self.git
            .get_or_init(|| {
                let shell = self
                    .options
                    .shell
                    .clone()
                    .or_else(|| std::env::var_os("SHELL").map(PathBuf::from))
                    .unwrap_or_else(|| PathBuf::from("/bin/zsh"));
                crate::lanes::git::discover(&shell)
            })
            .as_ref()
            .map_err(String::as_str)
    }

    /// Whether terminals of this workspace can run in a lane (FR-001).
    pub fn lane_check(&self, ws: Uuid) -> AppResult<crate::lanes::ops::Availability> {
        let folder = self
            .workspaces()
            .workspace(ws)
            .map(|w| w.working_directory.clone())
            .ok_or_else(|| AppError::workspace_not_found(ws))?;
        Ok(crate::lanes::ops::availability(self.git(), &folder))
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
        let ids: Vec<Uuid> = ws.nodes.iter().map(|n| n.id).collect();
        self.lane_statuses
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .forget(&ids);
        self.refresh_lane_statuses_soon();
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
        let lane_dir = term
            .lane
            .as_ref()
            .filter(|l| !l.is_landed())
            .map(|l| l.working_directory());
        Some(SpawnSpec {
            node_id: node.id,
            cwd: lane_dir
                .or_else(|| term.working_directory.clone())
                .unwrap_or_else(|| ws.working_directory.clone()),
            launch_command: term.launch_command.clone(),
            rows,
            cols,
            history_max_rows: self.preferences.history_max_rows as usize,
            shell: self.options.shell.clone(),
            restored_history: None,
            session_env: self.session_env(),
            wrap_agents: self.socket_path.is_some(),
            zsh_dir: self
                .socket_path
                .as_ref()
                .map(|_| self.data_root.path().join("run").join("zsh")),
        })
    }

    /// Environment shared by every terminal when the CLI channel runs.
    fn session_env(&self) -> Vec<(String, String)> {
        let Some(socket) = &self.socket_path else {
            return Vec::new();
        };
        let mut env = vec![("SONATA_SOCKET".to_string(), socket.display().to_string())];
        if let Some(dir) = &self.options.cli_dir {
            let path =
                std::env::var("PATH").unwrap_or_else(|_| "/usr/bin:/bin:/usr/sbin:/sbin".into());
            env.push(("PATH".into(), format!("{}:{path}", dir.display())));
        }
        env
    }

    fn start_terminal(&self, ws: &Workspace, node: &Node) {
        // A landed terminal stays stopped until restarted; a lane whose folder is gone cannot
        // start (feature 008, FR-011, FR-020).
        if let Some(l) = node.terminal().and_then(|t| t.lane.as_deref())
            && (l.is_landed() || !l.checkout_path.is_dir())
        {
            return;
        }
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
            return;
        }
        self.maybe_type_instructions(ws, node);
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
        let lane = if req.lane {
            let folder = self
                .workspaces()
                .workspace(ws_id)
                .map(|w| w.working_directory.clone())
                .ok_or_else(|| AppError::workspace_not_found(ws_id))?;
            let git = self
                .git()
                .map_err(|m| AppError::new(ErrorCode::LaneUnavailable, m))?;
            Some(crate::lanes::ops::create(
                git,
                &self.data_root.lane_workspace_dir(ws_id),
                &folder,
                &name,
            )?)
        } else {
            None
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
                    if let Some(lane) = lane {
                        t.working_directory = None;
                        t.lane = Some(Box::new(lane));
                    }
                    t
                }),
            };
            node.frame = node.frame.clamped(node.min_size());
            ws.nodes.push(node.clone());
            Ok((ws.clone(), node))
        })?;
        self.start_terminal(&ws, &node);
        if req.lane {
            self.refresh_lane_statuses_soon();
        }
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
        if is_terminal && self.has_lane_checkout(ws, node) {
            return Err(AppError::new(
                ErrorCode::LaneConfirmationNeeded,
                "This terminal works in a lane. Choose what happens to the lane first.",
            ));
        }
        if is_terminal && !confirmed && self.terminals.is_busy(node) {
            return Err(AppError::new(
                ErrorCode::TerminalBusy,
                "A program is still running in this terminal. Close it anyway?",
            ));
        }
        self.broker.cancel_from(self, node);
        self.terminals.stop(node);
        if let Some(host) = self.browser_host() {
            host.close(node);
        }
        let _ = std::fs::remove_dir_all(self.run_dir(node));
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
        if self.clear_landed_lane(node)? || !self.terminals.contains(node) {
            return self.start_fresh(node);
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
        self.remove_workspace_lanes(id)?;
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
        let browsers: Vec<Uuid> = self
            .workspaces()
            .workspace(id)
            .map(|w| {
                w.nodes
                    .iter()
                    .filter(|n| n.browser().is_some())
                    .map(|n| n.id)
                    .collect()
            })
            .unwrap_or_default();
        if let Some(host) = self.browser_host() {
            host.forget_workspace(id, &browsers);
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
        self.broker.shutdown(self);
        self.server.lock().unwrap_or_else(|e| e.into_inner()).take();
        let _ = std::fs::remove_dir_all(self.data_root.path().join("run").join("sessions"));
        let _ = std::fs::remove_dir_all(self.data_root.path().join("run").join("shots"));
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

// ---- Agent messaging (feature 003) ----
impl App {
    pub fn broker(&self) -> &crate::messaging::broker::Broker {
        &self.broker
    }

    pub fn socket_path(&self) -> Option<&std::path::Path> {
        self.socket_path.as_deref()
    }

    fn start_messaging(&self) {
        let Some(path) = self.socket_path.clone() else {
            return;
        };
        let zsh_dir = self.data_root.path().join("run").join("zsh");
        if let Err(e) = crate::terminal::zsh::write_files(&zsh_dir) {
            tracing::warn!(error = %e, "could not write zsh helper files");
        }
        match crate::messaging::server::Server::start(self.this.clone(), path) {
            Ok(server) => *self.server.lock().unwrap_or_else(|e| e.into_inner()) = Some(server),
            Err(e) => {
                tracing::error!(error = %e, "could not start the CLI socket");
                self.emit(CoreEvent::PersistenceWarning(format!(
                    "Agent messaging is off: {e}"
                )));
                return;
            }
        }
        let this = self.this.clone();
        std::thread::Builder::new()
            .name("sonata-broker".into())
            .spawn(move || {
                loop {
                    std::thread::sleep(crate::messaging::broker::TICK);
                    let Some(app) = this.upgrade() else { break };
                    app.broker.tick(&app);
                }
            })
            .expect("spawn broker thread");
    }

    pub fn node_name(&self, id: Uuid) -> Option<String> {
        let ws = self.workspace_of_node(id).ok()?;
        let m = self.workspaces();
        let n = m.workspace(ws)?.node(id)?.clone();
        drop(m);
        Some(match &n.content {
            NodeContent::Terminal(t) => t.name.clone(),
            NodeContent::Browser(b) => crate::messaging::commands::browser_title(b),
            NodeContent::Note(_) => crate::messaging::commands::note_title(
                &self.note_read(id).map(|v| v.text).unwrap_or_default(),
            ),
        })
    }

    /// Running and quiet for `quiet` after printing at least once.
    pub fn terminal_ready(&self, id: Uuid, quiet: std::time::Duration) -> bool {
        self.terminals.output_seq(id).unwrap_or(0) > 0
            && self.terminals.quiet_for(id).is_some_and(|q| q >= quiet)
    }

    /// Starts the terminals of a workspace that is not open, without switching to it.
    pub fn load_workspace_terminals(&self, ws_id: Uuid) {
        let ws = {
            let mut m = self.workspaces();
            if !m.mark_loaded(ws_id) {
                return;
            }
            m.workspace(ws_id).cloned()
        };
        let Some(ws) = ws else { return };
        for node in &ws.nodes {
            if node.terminal().is_some() && !self.terminals.contains(node.id) {
                self.start_terminal(&ws, node);
            }
        }
        self.watch_notes(ws_id);
        self.notify_workspaces_changed();
    }

    fn run_dir(&self, node: Uuid) -> PathBuf {
        self.data_root
            .path()
            .join("run")
            .join("sessions")
            .join(node.to_string())
    }

    /// Writes this terminal's session instructions and hook settings (research 003 R4).
    pub fn prepare_agent_session(
        &self,
        node: Uuid,
        hook: &str,
    ) -> AppResult<crate::messaging::instructions::SessionFiles> {
        let ws_id = self.workspace_of_node(node)?;
        let (name, workspace, role, orchestrator, lane) = {
            let m = self.workspaces();
            let ws = m
                .workspace(ws_id)
                .ok_or_else(|| AppError::workspace_not_found(ws_id))?;
            let t = ws
                .node(node)
                .and_then(|n| n.terminal().cloned())
                .ok_or_else(|| AppError::node_not_found(node))?;
            (
                t.name,
                ws.name.clone(),
                t.role.map(|r| r.text),
                t.orchestrator,
                t.lane.filter(|l| !l.is_landed()).map(|l| *l),
            )
        };
        let info = crate::messaging::instructions::SessionInfo {
            name: &name,
            workspace: &workspace,
            role: role.as_deref(),
            orchestrator,
            lane: lane.as_ref().map(lane_info),
        };
        Ok(crate::messaging::instructions::write_session(
            &self.run_dir(node),
            &info,
            hook,
        )?)
    }

    /// Types a short instruction into agents that cannot get session instructions (FR-019).
    fn maybe_type_instructions(&self, ws: &Workspace, node: &Node) {
        let Some(t) = node.terminal() else { return };
        if self.socket_path.is_none() {
            return;
        }
        let kind = crate::messaging::commands::target_kind(node);
        let first = t
            .launch_command
            .as_deref()
            .and_then(|c| c.split_whitespace().next());
        let automatic = match kind {
            crate::messaging::broker::TargetKind::Shell => false,
            crate::messaging::broker::TargetKind::Other => true,
            // Codex has no verified session-instructions option yet (research 003 R4).
            crate::messaging::broker::TargetKind::Hooked => first == Some("codex"),
        };
        if !t.send_instructions.unwrap_or(automatic) {
            return;
        }
        let info = crate::messaging::instructions::SessionInfo {
            name: &t.name,
            workspace: &ws.name,
            role: t.role.as_ref().map(|r| r.text.as_str()),
            orchestrator: t.orchestrator,
            lane: t.lane.as_deref().filter(|l| !l.is_landed()).map(lane_info),
        };
        let text = crate::messaging::instructions::typed_instructions(&info);
        let this = self.this.clone();
        let id = node.id;
        std::thread::spawn(move || {
            let start = std::time::Instant::now();
            // Give the agent time to start, then wait for its screen to settle.
            std::thread::sleep(std::time::Duration::from_secs(2));
            while start.elapsed() < crate::messaging::broker::READY_LIMIT {
                let Some(app) = this.upgrade() else { return };
                if app.terminal_ready(id, std::time::Duration::from_secs(2)) {
                    let _ = app.terminals.deliver(id, &text);
                    return;
                }
                drop(app);
                std::thread::sleep(std::time::Duration::from_millis(300));
            }
        });
    }

    /// Writes a note on behalf of an agent and shows the change on the canvas (FR-024).
    pub fn note_write_from_agent(&self, node: Uuid, text: &str, append: bool) -> AppResult<()> {
        let current = self.note_read(node)?;
        let next = if append {
            let sep = if current.text.is_empty() || current.text.ends_with('\n') {
                ""
            } else {
                "\n"
            };
            format!("{}{sep}{text}", current.text)
        } else {
            text.to_string()
        };
        if matches!(current.sync, crate::notes::store::NoteSync::Dirty) {
            // The user is editing: keep their text and leave the agent's version as a conflict copy.
            let ws = self.workspace_of_node(node)?;
            let path = self.note_store().conflict_path(ws, node);
            crate::persistence::atomic::write(&path, next.as_bytes())?;
            self.emit(CoreEvent::NoteConflict(crate::events::NoteConflictEvent {
                node_id: node,
                conflict_path: path.display().to_string(),
            }));
            return Ok(());
        }
        self.note_write(node, &next, true)?;
        self.emit(CoreEvent::NoteChanged(crate::events::NoteChangedEvent {
            node_id: node,
            text: next,
            sync: "clean".into(),
        }));
        Ok(())
    }

    pub fn notify_nodes_changed(&self, ws: Uuid) {
        let snapshot = self
            .workspaces()
            .workspace(ws)
            .map(|w| (w.nodes.clone(), w.links.clone()));
        if let Some((nodes, links)) = snapshot {
            self.emit(CoreEvent::NodesChanged(crate::events::NodesChangedEvent {
                workspace_id: ws,
                nodes,
                links,
            }));
        }
    }

    /// A free 640×400 spot near `anchor` (research 003 R10).
    fn free_spot(&self, ws: Uuid, anchor: &Node) -> Frame {
        self.free_spot_sized(ws, anchor, (640.0, 400.0))
    }

    /// A free spot of `size` near `anchor`, trying right, below, then left.
    fn free_spot_sized(&self, ws: Uuid, anchor: &Node, size: (f64, f64)) -> Frame {
        let nodes: Vec<Frame> = self
            .workspaces()
            .workspace(ws)
            .map(|w| w.nodes.iter().map(|n| n.frame).collect())
            .unwrap_or_default();
        let ((w, h), gap) = (size, 24.0);
        let overlaps = |f: &Frame| {
            nodes.iter().any(|o| {
                f.x < o.x + o.width + gap
                    && o.x < f.x + f.width + gap
                    && f.y < o.y + o.height + gap
                    && o.y < f.y + f.height + gap
            })
        };
        let a = anchor.frame;
        for step in 0..40 {
            let d = f64::from(step) * 40.0;
            for f in [
                Frame {
                    x: a.x + a.width + gap * 2.0 + d,
                    y: a.y,
                    width: w,
                    height: h,
                },
                Frame {
                    x: a.x,
                    y: a.y + a.height + gap * 2.0 + d,
                    width: w,
                    height: h,
                },
                Frame {
                    x: a.x - w - gap * 2.0 - d,
                    y: a.y,
                    width: w,
                    height: h,
                },
            ] {
                if !overlaps(&f) {
                    return f;
                }
            }
        }
        Frame {
            x: a.x + a.width + gap * 2.0,
            y: a.y,
            width: w,
            height: h,
        }
    }

    /// Creates a terminal linked to the orchestrator (FR-021). Returns its final name.
    pub fn recruit(
        &self,
        orchestrator: Uuid,
        what: &str,
        name: Option<&str>,
        role: Option<&str>,
        role_text: Option<&str>,
        place: RecruitPlace,
    ) -> AppResult<(String, Option<crate::model::node::Lane>)> {
        let (folder, lane) = match place {
            RecruitPlace::WorkspaceFolder => (None, false),
            RecruitPlace::Folder(f) => (Some(f), false),
            RecruitPlace::Lane => (None, true),
        };
        let ws = self.workspace_of_node(orchestrator)?;
        let anchor = self
            .workspaces()
            .workspace(ws)
            .and_then(|w| w.node(orchestrator).cloned())
            .ok_or_else(|| AppError::node_not_found(orchestrator))?;
        let preset = crate::presets::PRESETS
            .iter()
            .find(|p| p.id == what.to_lowercase());
        let command = match preset {
            Some(p) => p.command.map(str::to_string),
            None => Some(what.to_string()),
        };
        let base = name.map(str::to_string).unwrap_or_else(|| {
            preset
                .map(|p| p.name.to_string())
                .unwrap_or_else(|| "Worker".into())
        });
        let taken: Vec<String> = self
            .links_of(ws, orchestrator)
            .iter()
            .filter_map(|l| l.other(orchestrator))
            .filter_map(|id| self.node_name(id))
            .map(|n| n.to_lowercase())
            .collect();
        let mut final_name = base.clone();
        let mut i = 2;
        while taken.contains(&final_name.to_lowercase()) {
            final_name = format!("{base} {i}");
            i += 1;
        }
        let role = match (role, role_text) {
            (Some(r), _) => {
                let lib = self
                    .roles_list()
                    .into_iter()
                    .find(|x| x.name.eq_ignore_ascii_case(r));
                Some(
                    lib.map(|x| RoleRef {
                        name: Some(x.name),
                        text: x.text,
                    })
                    .ok_or_else(|| {
                        AppError::new(
                            ErrorCode::NodeNotFound,
                            format!("No role named \"{r}\" in the role library."),
                        )
                    })?,
                )
            }
            (None, Some(t)) => Some(RoleRef {
                name: None,
                text: t.to_string(),
            }),
            _ => None,
        };
        if let Some(f) = &folder {
            crate::workspace::manager::validate_directory(f)?;
        }
        let frame = self.free_spot(ws, &anchor);
        let node = self.create_terminal(
            ws,
            NewTerminal {
                frame,
                name: Some(final_name.clone()),
                launch_command: command,
                working_directory: folder,
                settings: TerminalSettings {
                    role: Some(role),
                    ..Default::default()
                },
                lane,
            },
        )?;
        self.update_node(ws, node.id, |w, id| {
            if let Some(t) = w.node_mut(id).and_then(Node::terminal_mut) {
                t.recruited_by = Some(orchestrator);
            }
            Ok(())
        })?;
        self.create_link(ws, orchestrator, node.id)?;
        self.notify_nodes_changed(ws);
        Ok((
            final_name,
            node.terminal().and_then(|t| t.lane.as_deref().cloned()),
        ))
    }

    /// Orchestrator: a note next to it, linked to it, starting with `# title` and `text`.
    pub fn orchestrator_new_note(
        &self,
        orchestrator: Uuid,
        title: &str,
        text: &str,
    ) -> AppResult<Node> {
        let ws = self.workspace_of_node(orchestrator)?;
        let anchor = self
            .workspaces()
            .workspace(ws)
            .and_then(|w| w.node(orchestrator).cloned())
            .ok_or_else(|| AppError::node_not_found(orchestrator))?;
        let frame = self.free_spot_sized(ws, &anchor, (320.0, 240.0));
        let note = self.create_note(ws, frame)?;
        let mut body = format!("# {}\n", title.trim());
        if !text.trim().is_empty() {
            body.push_str(&format!("\n{}\n", text.trim_end()));
        }
        self.note_write(note.id, &body, true)?;
        self.create_link(ws, orchestrator, note.id)?;
        self.emit(CoreEvent::NoteChanged(crate::events::NoteChangedEvent {
            node_id: note.id,
            text: body,
            sync: "clean".into(),
        }));
        self.notify_nodes_changed(ws);
        Ok(note)
    }

    /// Orchestrator: a browser next to it, linked to it.
    pub fn orchestrator_new_browser(
        &self,
        orchestrator: Uuid,
        url: Option<&str>,
    ) -> AppResult<Node> {
        let ws = self.workspace_of_node(orchestrator)?;
        let anchor = self
            .workspaces()
            .workspace(ws)
            .and_then(|w| w.node(orchestrator).cloned())
            .ok_or_else(|| AppError::node_not_found(orchestrator))?;
        let frame = self.free_spot_sized(ws, &anchor, crate::browser::DEFAULT_SIZE);
        let node = self.create_browser(ws, frame, url)?;
        self.create_link(ws, orchestrator, node.id)?;
        self.notify_nodes_changed(ws);
        Ok(node)
    }

    /// Closes a recruited terminal and removes it with its links (FR-022).
    pub fn dismiss(&self, node: Uuid) -> AppResult<()> {
        let ws = self.workspace_of_node(node)?;
        self.delete_node(ws, node, true)?;
        self.notify_nodes_changed(ws);
        Ok(())
    }

    // ---- Browser nodes (feature 004) ----

    /// Registers the shell's page driver once the window exists.
    pub fn set_browser_host(&self, host: Arc<dyn crate::browser::BrowserHost>) {
        *self.browser_host.lock().unwrap_or_else(|e| e.into_inner()) = Some(host);
    }

    pub fn browser_host(&self) -> Option<Arc<dyn crate::browser::BrowserHost>> {
        self.browser_host
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    pub fn create_browser(&self, ws_id: Uuid, frame: Frame, url: Option<&str>) -> AppResult<Node> {
        self.require_loaded(ws_id)?;
        let url = match url {
            Some(u) if !u.trim().is_empty() => crate::browser::normalize_url(u)?,
            _ => "about:blank".to_string(),
        };
        let now = Utc::now();
        self.workspaces().update(ws_id, |ws| {
            let mut node = Node {
                id: Uuid::new_v4(),
                frame,
                z: ws.max_z() + 1,
                created_at: now,
                modified_at: now,
                content: NodeContent::Browser(crate::model::node::BrowserContent {
                    url,
                    title: None,
                }),
            };
            node.frame = node.frame.clamped(node.min_size());
            ws.nodes.push(node.clone());
            Ok(node)
        })
    }

    /// The shell reports a finished page load; saved so the node reopens it (FR-005).
    pub fn update_browser_page(&self, node: Uuid, url: &str, title: Option<&str>) -> AppResult<()> {
        let ws = self.workspace_of_node(node)?;
        let unchanged = self
            .workspaces()
            .workspace(ws)
            .and_then(|w| w.node(node))
            .and_then(|n| n.browser())
            .is_some_and(|b| b.url == url && b.title.as_deref() == title);
        if unchanged {
            return Ok(());
        }
        self.update_node(ws, node, |w, id| {
            if let Some(b) = w.node_mut(id).and_then(Node::browser_mut) {
                b.url = url.to_string();
                b.title = title.map(str::to_string).filter(|t| !t.is_empty());
            }
            Ok(())
        })
    }

    /// Where agent screenshots go when no `--out` is given (private, removed on quit).
    pub fn screenshot_path(&self) -> PathBuf {
        let dir = self.data_root.path().join("run").join("shots");
        let _ = paths::ensure_private_dir(&dir);
        dir.join(format!("{}.png", Uuid::new_v4()))
    }

    /// Shows "controlled by" on the browser and a busy link while an agent drives it (FR-019).
    pub fn mark_browser_control(&self, from: Uuid, browser: Uuid) -> (Uuid, Uuid, Uuid) {
        let id = Uuid::new_v4();
        self.emit(CoreEvent::AskState(crate::events::AskEvent {
            ask_id: id,
            from,
            to: browser,
            state: crate::messaging::broker::AskState::Waiting,
        }));
        (id, from, browser)
    }

    pub fn end_browser_control(&self, (id, from, browser): (Uuid, Uuid, Uuid)) {
        self.emit(CoreEvent::AskState(crate::events::AskEvent {
            ask_id: id,
            from,
            to: browser,
            state: crate::messaging::broker::AskState::Answered,
        }));
    }

    pub fn exchange_log(&self, node: Uuid) -> Vec<crate::messaging::broker::LogEntry> {
        self.broker.log(node)
    }
}

// ---- Lanes (feature 008) ----

fn confirmation_required(action: &str) -> AppError {
    AppError::new(
        ErrorCode::ConfirmationRequired,
        format!("Confirm the {action} first."),
    )
}

impl App {
    fn lane_git(&self) -> AppResult<&crate::lanes::git::Git> {
        self.git()
            .map_err(|m| AppError::new(ErrorCode::LaneUnavailable, m))
    }

    /// The terminal's lane while it is active (not landed).
    fn active_lane(&self, node: Uuid) -> AppResult<(Uuid, crate::model::node::Lane)> {
        let ws = self.workspace_of_node(node)?;
        let lane = self
            .workspaces()
            .workspace(ws)
            .and_then(|w| w.node(node))
            .and_then(Node::terminal)
            .and_then(|t| t.lane.as_deref().cloned());
        match lane {
            Some(l) if !l.is_landed() => Ok((ws, l)),
            _ => Err(AppError::new(
                ErrorCode::LaneBlocked,
                "This terminal has no active lane.",
            )),
        }
    }

    fn set_lane(
        &self,
        ws: Uuid,
        node: Uuid,
        lane: Option<crate::model::node::Lane>,
    ) -> AppResult<()> {
        self.update_node(ws, node, |w, id| {
            if let Some(t) = w.node_mut(id).and_then(Node::terminal_mut) {
                t.lane = lane.map(Box::new);
            }
            Ok(())
        })
    }

    /// Clears a landed lane so the terminal runs in the workspace folder again.
    fn clear_landed_lane(&self, node: Uuid) -> AppResult<bool> {
        let ws = self.workspace_of_node(node)?;
        let landed = self
            .workspaces()
            .workspace(ws)
            .and_then(|w| w.node(node))
            .and_then(Node::terminal)
            .and_then(|t| t.lane.as_deref())
            .is_some_and(|l| l.is_landed());
        if landed {
            self.set_lane(ws, node, None)?;
        }
        Ok(landed)
    }

    /// Stops a terminal before its checkout goes away.
    fn stop_for_lane(&self, node: Uuid) {
        self.broker.cancel_from(self, node);
        self.terminals.stop(node);
        self.emit(CoreEvent::TerminalState(
            crate::events::TerminalStateEvent {
                node_id: node,
                run: crate::events::RunState::Exited,
                exit_code: None,
                busy: false,
                warning: None,
            },
        ));
    }

    pub fn lane_preview(
        &self,
        node: Uuid,
        action: crate::lanes::ops::Action,
    ) -> AppResult<crate::lanes::ops::Preview> {
        let (_, lane) = self.active_lane(node)?;
        Ok(crate::lanes::ops::preview(self.lane_git()?, &lane, action))
    }

    /// Merges a terminal's lane into its base branch after human confirmation (FR-008, FR-011).
    pub fn lane_land(&self, node: Uuid, confirmed: bool) -> AppResult<u32> {
        if !confirmed {
            return Err(confirmation_required("land"));
        }
        let (ws, lane) = self.active_lane(node)?;
        let commits = crate::lanes::ops::land(self.lane_git()?, &lane)?;
        self.finish_land(ws, node, &lane);
        Ok(commits)
    }

    /// Removes a terminal's lane after human confirmation; the terminal restarts in the
    /// workspace folder (FR-013 to FR-016).
    pub fn lane_remove(&self, node: Uuid, removal: LaneRemoval, confirmed: bool) -> AppResult<()> {
        if !confirmed {
            return Err(confirmation_required("removal"));
        }
        let (ws, lane) = self.active_lane(node)?;
        self.remove_lane(ws, node, &lane, removal)?;
        self.refresh_lane_statuses_soon();
        self.start_fresh(node)
    }

    /// Stops the terminal and removes the lane; the caller decides what runs next.
    fn remove_lane(
        &self,
        ws: Uuid,
        node: Uuid,
        lane: &crate::model::node::Lane,
        removal: LaneRemoval,
    ) -> AppResult<()> {
        let git = self.lane_git()?;
        let dir = self.data_root.lane_workspace_dir(ws);
        crate::lanes::ops::prepare_removal(git, &dir, lane, removal.discard_changes)?;
        self.stop_for_lane(node);
        crate::lanes::ops::remove(
            git,
            &dir,
            lane,
            removal.discard_changes,
            removal.delete_branch,
        )?;
        self.set_lane(ws, node, None)
    }

    /// Drops the record of a lane whose folder is gone; the branch is kept (FR-020).
    pub fn lane_forget(&self, node: Uuid) -> AppResult<()> {
        let ws = self.workspace_of_node(node)?;
        let lane = self
            .workspaces()
            .workspace(ws)
            .and_then(|w| w.node(node))
            .and_then(Node::terminal)
            .and_then(|t| t.lane.as_deref().cloned())
            .ok_or_else(|| AppError::new(ErrorCode::LaneBlocked, "This terminal has no lane."))?;
        if lane.checkout_path.exists() {
            return Err(AppError::new(
                ErrorCode::LaneBlocked,
                "The lane folder still exists. Use Remove lane instead.",
            ));
        }
        if let Ok(git) = self.lane_git() {
            let _ = git.run(&lane.main_checkout, &["worktree", "prune"]);
        }
        self.set_lane(ws, node, None)?;
        self.notify_nodes_changed(ws);
        self.refresh_lane_statuses_soon();
        Ok(())
    }

    /// True when deleting this terminal would delete a lane folder.
    fn has_lane_checkout(&self, ws: Uuid, node: Uuid) -> bool {
        self.workspaces()
            .workspace(ws)
            .and_then(|w| w.node(node))
            .and_then(Node::terminal)
            .and_then(|t| t.lane.as_deref())
            .is_some_and(|l| !l.is_landed() && l.checkout_path.exists())
    }

    /// Deletes a terminal that has a lane, removing the lane under the dialog's choices (FR-017).
    pub fn delete_node_with_lane(
        &self,
        ws: Uuid,
        node: Uuid,
        confirmed: bool,
        removal: LaneRemoval,
    ) -> AppResult<()> {
        if !confirmed && self.terminals.is_busy(node) {
            return Err(AppError::new(
                ErrorCode::TerminalBusy,
                "A program is still running in this terminal. Close it anyway?",
            ));
        }
        if let Ok((_, lane)) = self.active_lane(node) {
            self.remove_lane(ws, node, &lane, removal)?;
        }
        self.delete_node(ws, node, true)
    }

    /// Starts a stopped or landed terminal again with its current settings.
    fn start_fresh(&self, node: Uuid) -> AppResult<()> {
        let ws_id = self.workspace_of_node(node)?;
        let (ws, n) = {
            let m = self.workspaces();
            let ws = m
                .workspace(ws_id)
                .cloned()
                .ok_or_else(|| AppError::workspace_not_found(ws_id))?;
            let n = ws
                .node(node)
                .cloned()
                .ok_or_else(|| AppError::node_not_found(node))?;
            (ws, n)
        };
        self.terminals.stop(node);
        self.start_terminal(&ws, &n);
        self.notify_nodes_changed(ws_id);
        Ok(())
    }

    /// Before a workspace is deleted: refuse when a lane holds uncommitted work, otherwise
    /// remove every lane checkout (research R14).
    fn remove_workspace_lanes(&self, ws: Uuid) -> AppResult<()> {
        let lanes: Vec<(Uuid, String, crate::model::node::Lane)> = self
            .workspaces()
            .workspace(ws)
            .map(|w| {
                w.nodes
                    .iter()
                    .filter_map(|n| {
                        let t = n.terminal()?;
                        let l = t.lane.as_deref()?;
                        (!l.is_landed()).then(|| (n.id, t.name.clone(), l.clone()))
                    })
                    .collect()
            })
            .unwrap_or_default();
        if lanes.is_empty() {
            return Ok(());
        }
        let git = self.lane_git()?;
        let dir = self.data_root.lane_workspace_dir(ws);
        let mut dirty = Vec::new();
        for (_, name, lane) in &lanes {
            match crate::lanes::ops::prepare_removal(git, &dir, lane, false) {
                Ok(()) => {}
                Err(e) if e.code == ErrorCode::LaneHasChanges => {
                    dirty.push(format!("\"{name}\" ({})", lane.branch));
                }
                Err(e) => return Err(e),
            }
        }
        if !dirty.is_empty() {
            return Err(AppError::new(
                ErrorCode::LaneHasChanges,
                format!(
                    "These lanes have uncommitted work: {}. Land, commit, or remove them first.",
                    dirty.join(", ")
                ),
            ));
        }
        for (node, _, lane) in &lanes {
            self.stop_for_lane(*node);
            crate::lanes::ops::remove(git, &dir, lane, false, false)?;
        }
        Ok(())
    }

    fn spawn_lane_status_loop(&self) {
        let this = self.this.clone();
        std::thread::Builder::new()
            .name("sonata-lanes".into())
            .spawn(move || {
                loop {
                    std::thread::sleep(crate::lanes::status::POLL_INTERVAL);
                    let Some(app) = this.upgrade() else { break };
                    app.check_lane_statuses();
                }
            })
            .expect("spawn lane status thread");
    }

    /// Checks every lane of the loaded workspaces and sends the ones that changed (FR-018).
    pub fn check_lane_statuses(&self) {
        let lanes: Vec<(Uuid, crate::model::node::Lane)> = {
            let m = self.workspaces();
            m.ids()
                .into_iter()
                .filter(|id| m.is_loaded(*id))
                .filter_map(|id| m.workspace(id))
                .flat_map(|w| w.nodes.iter())
                .filter_map(|n| Some((n.id, n.terminal()?.lane.as_deref()?.clone())))
                .collect()
        };
        if lanes.is_empty()
            && self
                .lane_statuses
                .lock()
                .map(|c| c.is_empty())
                .unwrap_or(true)
        {
            return;
        }
        let git = self.git();
        let statuses = lanes
            .iter()
            .map(|(id, lane)| crate::lanes::status::check(git, *id, lane))
            .collect();
        let changed = self
            .lane_statuses
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .update(statuses);
        for s in changed {
            self.emit(CoreEvent::LaneStatus(s));
        }
    }

    /// Checks lanes now on a background thread, after a lane was created, landed, or removed.
    fn refresh_lane_statuses_soon(&self) {
        let this = self.this.clone();
        std::thread::spawn(move || {
            if let Some(app) = this.upgrade() {
                app.check_lane_statuses();
            }
        });
    }

    /// The last known status of a terminal's lane, checked now.
    pub fn lane_status(&self, node: Uuid) -> AppResult<crate::lanes::status::LaneStatus> {
        let ws = self.workspace_of_node(node)?;
        let lane = self
            .workspaces()
            .workspace(ws)
            .and_then(|w| w.node(node))
            .and_then(Node::terminal)
            .and_then(|t| t.lane.as_deref().cloned())
            .ok_or_else(|| AppError::new(ErrorCode::NodeNotFound, "This terminal has no lane."))?;
        Ok(crate::lanes::status::check(self.git(), node, &lane))
    }

    /// After a land: stop the terminal, remove the checkout, mark the lane landed (FR-011).
    fn finish_land(&self, ws: Uuid, node: Uuid, lane: &crate::model::node::Lane) {
        self.stop_for_lane(node);
        let removed = self.lane_git().and_then(|g| {
            crate::lanes::ops::remove_checkout(
                g,
                &self.data_root.lane_workspace_dir(ws),
                lane,
                false,
            )
        });
        match removed {
            Ok(()) => {
                let mut landed = lane.clone();
                landed.landed_at = Some(Utc::now());
                if let Err(e) = self.set_lane(ws, node, Some(landed)) {
                    tracing::error!(error = %e, "could not mark the lane landed");
                }
            }
            Err(e) => self.emit(CoreEvent::PersistenceWarning(format!(
                "{} was landed, but its folder could not be removed: {} Remove the lane from \
                 the terminal menu.",
                lane.branch, e.message
            ))),
        }
        self.notify_nodes_changed(ws);
        self.refresh_lane_statuses_soon();
    }
}

impl From<std::sync::PoisonError<MutexGuard<'_, WorkspaceManager>>> for AppError {
    fn from(_: std::sync::PoisonError<MutexGuard<'_, WorkspaceManager>>) -> Self {
        AppError::internal("internal state lock poisoned")
    }
}

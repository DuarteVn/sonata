//! Running terminals: PTY processes, their screen models, and attached views.

use std::collections::HashMap;
use std::io::{self, Read, Write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex, Weak};
use std::time::Duration;

use portable_pty::{ChildKiller, MasterPty, PtySize};
use uuid::Uuid;

use super::busy;
use super::pty::{self, SpawnOptions};
use super::screen::{ScreenModel, Vt100Screen};
use crate::events::{CoreEvent, EventSink, RunState, TerminalStateEvent};

/// Live output is flushed to views at least this often…
pub const FLUSH_INTERVAL: Duration = Duration::from_millis(8);
/// …or as soon as this many bytes are pending. No message is larger.
pub const MAX_CHUNK: usize = 32 * 1024;

/// Receives terminal output for one attached view. Returns `false` once disconnected.
pub trait OutputSink: Send {
    fn send(&self, bytes: Vec<u8>) -> bool;
}

#[derive(Debug, Clone)]
pub struct SpawnSpec {
    pub node_id: Uuid,
    pub cwd: PathBuf,
    pub launch_command: Option<String>,
    pub rows: u16,
    pub cols: u16,
    pub history_max_rows: usize,
    /// Overrides the login shell (tests use `/bin/sh`).
    pub shell: Option<PathBuf>,
    /// Earlier output fed into the screen before the shell starts (FR-015).
    pub restored_history: Option<Vec<u8>>,
    /// Environment shared by all terminals (socket path, CLI folder on PATH). The per-start
    /// identity (`SONATA_TOKEN`) is added by the manager.
    pub session_env: Vec<(String, String)>,
    pub wrap_agents: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunStatus {
    Starting,
    Running { pid: u32 },
    Exited { code: i32 },
}

#[derive(Debug, Clone, Default)]
pub struct StartInfo {
    pub warning: Option<String>,
}

struct Terminal {
    spec: SpawnSpec,
    screen: Box<dyn ScreenModel>,
    status: RunStatus,
    shell_pid: Option<u32>,
    master: Option<Box<dyn MasterPty + Send>>,
    writer: Option<Box<dyn Write + Send>>,
    killer: Option<Box<dyn ChildKiller + Send + Sync>>,
    sink: Option<Box<dyn OutputSink>>,
    pending: Vec<u8>,
    /// Bumped on restart and stop so threads of an older process stop touching state.
    generation: u64,
    warning: Option<String>,
    /// Incremented on every chunk of output; lets callers skip saving unchanged history.
    output_seq: u64,
    last_output: std::time::Instant,
    /// Secret identity of the current start (feature 003, research R2). Never written to disk.
    token: String,
}

impl Terminal {
    fn flush(&mut self) {
        if self.pending.is_empty() {
            return;
        }
        let Some(sink) = &self.sink else {
            self.pending.clear();
            return;
        };
        let mut connected = true;
        for chunk in self.pending.chunks(MAX_CHUNK) {
            if !sink.send(chunk.to_vec()) {
                connected = false;
                break;
            }
        }
        self.pending.clear();
        if !connected {
            self.sink = None;
        }
    }

    fn busy(&self) -> bool {
        matches!(self.status, RunStatus::Running { .. })
            && self
                .master
                .as_deref()
                .is_some_and(|m| busy::is_busy(m, self.shell_pid))
    }
}

type Shared = Arc<Mutex<Terminal>>;
type Registry = Arc<Mutex<HashMap<Uuid, Shared>>>;

fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

pub struct TerminalManager {
    terminals: Registry,
    events: Arc<dyn EventSink>,
}

impl TerminalManager {
    pub fn new(events: Arc<dyn EventSink>) -> Self {
        let terminals: Registry = Arc::default();
        spawn_flusher(Arc::downgrade(&terminals));
        Self { terminals, events }
    }

    fn get(&self, id: Uuid) -> io::Result<Shared> {
        lock(&self.terminals)
            .get(&id)
            .cloned()
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, format!("no terminal {id}")))
    }

    pub fn contains(&self, id: Uuid) -> bool {
        lock(&self.terminals).contains_key(&id)
    }

    /// Creates the screen model, feeds restored history, and starts the shell.
    pub fn start(&self, spec: SpawnSpec) -> io::Result<StartInfo> {
        let id = spec.node_id;
        let mut screen: Box<dyn ScreenModel> = Box::new(Vt100Screen::new(
            spec.rows,
            spec.cols,
            spec.history_max_rows,
        ));
        if let Some(history) = &spec.restored_history {
            screen.process(history);
        }
        let terminal = Arc::new(Mutex::new(Terminal {
            spec,
            screen,
            status: RunStatus::Starting,
            shell_pid: None,
            master: None,
            writer: None,
            killer: None,
            sink: None,
            pending: Vec::new(),
            generation: 0,
            warning: None,
            output_seq: 0,
            last_output: std::time::Instant::now(),
            token: String::new(),
        }));
        lock(&self.terminals).insert(id, terminal.clone());
        self.launch(id, &terminal)
    }

    fn launch(&self, id: Uuid, terminal: &Shared) -> io::Result<StartInfo> {
        let (spec, generation, token) = {
            let mut t = lock(terminal);
            t.generation += 1;
            t.status = RunStatus::Starting;
            t.token = new_token();
            (t.spec.clone(), t.generation, t.token.clone())
        };
        let mut env = spec.session_env.clone();
        if !env.is_empty() {
            env.push(("SONATA_TOKEN".into(), token));
        }
        let (rows, cols) = lock(terminal).screen.size();
        let spawned = pty::spawn(SpawnOptions {
            node_id: id,
            cwd: &spec.cwd,
            shell: spec.shell.as_deref(),
            launch_command: spec.launch_command.as_deref(),
            rows,
            cols,
            env: &env,
            wrap_agents: spec.wrap_agents,
        })?;
        let pid = spawned.pid;
        {
            let mut t = lock(terminal);
            t.shell_pid = pid;
            t.master = Some(spawned.master);
            t.writer = Some(spawned.writer);
            t.killer = Some(spawned.killer);
            t.status = RunStatus::Running {
                pid: pid.unwrap_or_default(),
            };
            t.warning = spawned.warning.clone();
        }
        spawn_reader(Arc::downgrade(terminal), generation, spawned.reader);
        spawn_waiter(
            Arc::downgrade(terminal),
            generation,
            spawned.child,
            self.events.clone(),
            id,
        );
        self.emit_state(id);
        Ok(StartInfo {
            warning: spawned.warning,
        })
    }

    /// Starts a new shell for a terminal whose program exited (FR-017). Earlier output stays.
    pub fn restart(&self, id: Uuid) -> io::Result<StartInfo> {
        let terminal = self.get(id)?;
        {
            let mut t = lock(&terminal);
            if let Some(mut killer) = t.killer.take() {
                let _ = killer.kill();
            }
            t.master = None;
            t.writer = None;
            t.spec.restored_history = None;
        }
        self.launch(id, &terminal)
    }

    /// Kills the process and forgets the terminal.
    pub fn stop(&self, id: Uuid) {
        let Some(terminal) = lock(&self.terminals).remove(&id) else {
            return;
        };
        let mut t = lock(&terminal);
        t.generation += 1;
        if let Some(mut killer) = t.killer.take() {
            let _ = killer.kill();
        }
        t.master = None;
        t.writer = None;
        t.sink = None;
    }

    pub fn stop_all(&self) {
        let ids: Vec<Uuid> = lock(&self.terminals).keys().copied().collect();
        for id in ids {
            self.stop(id);
        }
    }

    /// Sends the replay, then streams live output to `sink` until detached.
    pub fn attach(&self, id: Uuid, sink: Box<dyn OutputSink>) -> io::Result<()> {
        let terminal = self.get(id)?;
        let mut t = lock(&terminal);
        t.pending.clear();
        if sink.send(t.screen.replay()) {
            t.sink = Some(sink);
        }
        Ok(())
    }

    pub fn detach(&self, id: Uuid) {
        if let Ok(terminal) = self.get(id) {
            let mut t = lock(&terminal);
            t.sink = None;
            t.pending.clear();
        }
    }

    pub fn write(&self, id: Uuid, bytes: &[u8]) -> io::Result<()> {
        let terminal = self.get(id)?;
        let mut t = lock(&terminal);
        let writer = t
            .writer
            .as_mut()
            .ok_or_else(|| io::Error::other("terminal is not running"))?;
        writer.write_all(bytes)?;
        writer.flush()
    }

    pub fn resize(&self, id: Uuid, rows: u16, cols: u16) -> io::Result<()> {
        let (rows, cols) = (rows.max(2), cols.max(2));
        let terminal = self.get(id)?;
        let mut t = lock(&terminal);
        t.screen.resize(rows, cols);
        t.spec.rows = rows;
        t.spec.cols = cols;
        if let Some(master) = &t.master {
            master
                .resize(PtySize {
                    rows,
                    cols,
                    pixel_width: 0,
                    pixel_height: 0,
                })
                .map_err(io::Error::other)?;
        }
        Ok(())
    }

    /// Command and folder used at the next start or restart.
    pub fn set_spawn_settings(&self, id: Uuid, command: Option<String>, cwd: PathBuf) {
        if let Ok(terminal) = self.get(id) {
            let mut t = lock(&terminal);
            t.spec.launch_command = command;
            t.spec.cwd = cwd;
        }
    }

    pub fn set_launch_command(&self, id: Uuid, command: Option<String>) {
        if let Ok(terminal) = self.get(id) {
            lock(&terminal).spec.launch_command = command;
        }
    }

    pub fn status(&self, id: Uuid) -> Option<RunStatus> {
        self.get(id).ok().map(|t| lock(&t).status)
    }

    pub fn is_busy(&self, id: Uuid) -> bool {
        self.get(id).is_ok_and(|t| lock(&t).busy())
    }

    pub fn screen_text(&self, id: Uuid) -> Option<String> {
        self.get(id)
            .ok()
            .map(|t| lock(&t).screen.preview_lines().join("\n"))
    }

    pub fn preview(&self, id: Uuid) -> Option<(Vec<String>, (u16, u16))> {
        self.get(id).ok().map(|t| {
            let t = lock(&t);
            (t.screen.preview_lines(), t.screen.cursor())
        })
    }

    /// The secret of the terminal's current start (the CLI reads it from the environment).
    pub fn session_token(&self, id: Uuid) -> Option<String> {
        self.get(id).ok().map(|t| lock(&t).token.clone())
    }

    /// True when `token` is the secret of this terminal's current start.
    pub fn check_token(&self, id: Uuid, token: &str) -> bool {
        self.get(id).is_ok_and(|t| {
            let t = lock(&t);
            !t.token.is_empty() && constant_time_eq(t.token.as_bytes(), token.as_bytes())
        })
    }

    pub fn absolute_row(&self, id: Uuid) -> Option<u64> {
        self.get(id).ok().map(|t| lock(&t).screen.absolute_row())
    }

    pub fn rows_since(&self, id: Uuid, from: u64) -> Option<(Vec<String>, bool)> {
        self.get(id).ok().map(|t| {
            let t = lock(&t);
            if t.screen.alternate_screen() {
                (
                    t.screen
                        .preview_lines()
                        .into_iter()
                        .map(|l| l.trim_end().to_string())
                        .collect(),
                    true,
                )
            } else {
                (t.screen.rows_since(from), false)
            }
        })
    }

    /// Text of the cursor's row up to the cursor (the shell prompt, before anything is typed).
    pub fn cursor_line_prefix(&self, id: Uuid) -> Option<String> {
        self.get(id).ok().map(|t| {
            let t = lock(&t);
            let (row, col) = t.screen.cursor();
            let line = t
                .screen
                .preview_lines()
                .get(row as usize)
                .cloned()
                .unwrap_or_default();
            line.chars().take(col as usize).collect()
        })
    }

    /// Types a message as one block: bracketed paste when the program asked for it, then Enter
    /// (research 003 R7).
    pub fn deliver(&self, id: Uuid, text: &str) -> io::Result<()> {
        let paste = self
            .get(id)
            .is_ok_and(|t| lock(&t).screen.bracketed_paste());
        if paste {
            self.write(id, format!("\x1b[200~{text}\x1b[201~").as_bytes())?;
            std::thread::sleep(Duration::from_millis(80));
            self.write(id, b"\r")
        } else {
            let typed = text.replace("\r\n", "\n").replace('\n', "\r");
            self.write(id, format!("{typed}\r").as_bytes())
        }
    }

    /// Time since the terminal last printed something.
    pub fn quiet_for(&self, id: Uuid) -> Option<Duration> {
        self.get(id).ok().map(|t| lock(&t).last_output.elapsed())
    }

    pub fn output_seq(&self, id: Uuid) -> Option<u64> {
        self.get(id).ok().map(|t| lock(&t).output_seq)
    }

    pub fn history_rows(&self, id: Uuid, max_rows: usize) -> Option<Vec<String>> {
        self.get(id)
            .ok()
            .map(|t| lock(&t).screen.history_rows(max_rows))
    }

    pub fn state_event(&self, id: Uuid) -> Option<TerminalStateEvent> {
        let terminal = self.get(id).ok()?;
        let t = lock(&terminal);
        Some(state_event(id, &t))
    }

    fn emit_state(&self, id: Uuid) {
        if let Some(event) = self.state_event(id) {
            self.events.emit(CoreEvent::TerminalState(event));
        }
    }
}

impl Drop for TerminalManager {
    fn drop(&mut self) {
        self.stop_all();
    }
}

fn state_event(id: Uuid, t: &Terminal) -> TerminalStateEvent {
    let (run, exit_code) = match t.status {
        RunStatus::Starting => (RunState::Starting, None),
        RunStatus::Running { .. } => (RunState::Running, None),
        RunStatus::Exited { code } => (RunState::Exited, Some(code)),
    };
    TerminalStateEvent {
        node_id: id,
        run,
        exit_code,
        busy: t.busy(),
        warning: t.warning.clone(),
    }
}

fn spawn_reader(
    terminal: Weak<Mutex<Terminal>>,
    generation: u64,
    mut reader: Box<dyn Read + Send>,
) {
    std::thread::Builder::new()
        .name("sonata-pty-reader".into())
        .spawn(move || {
            let mut buf = vec![0u8; 16 * 1024];
            loop {
                let n = match reader.read(&mut buf) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => n,
                };
                let Some(terminal) = terminal.upgrade() else {
                    break;
                };
                let mut t = lock(&terminal);
                if t.generation != generation {
                    break;
                }
                t.screen.process(&buf[..n]);
                t.output_seq += 1;
                t.last_output = std::time::Instant::now();
                if t.sink.is_some() {
                    t.pending.extend_from_slice(&buf[..n]);
                    if t.pending.len() >= MAX_CHUNK {
                        t.flush();
                    }
                }
            }
        })
        .expect("spawn reader thread");
}

fn spawn_waiter(
    terminal: Weak<Mutex<Terminal>>,
    generation: u64,
    mut child: Box<dyn portable_pty::Child + Send + Sync>,
    events: Arc<dyn EventSink>,
    id: Uuid,
) {
    std::thread::Builder::new()
        .name("sonata-pty-waiter".into())
        .spawn(move || {
            let code = child.wait().map(|s| s.exit_code() as i32).unwrap_or(-1);
            // Give the reader a moment to drain the last output before reporting the exit.
            std::thread::sleep(Duration::from_millis(50));
            let Some(terminal) = terminal.upgrade() else {
                return;
            };
            let event = {
                let mut t = lock(&terminal);
                if t.generation != generation {
                    return;
                }
                t.status = RunStatus::Exited { code };
                t.writer = None;
                t.killer = None;
                t.flush();
                state_event(id, &t)
            };
            events.emit(CoreEvent::TerminalState(event));
        })
        .expect("spawn waiter thread");
}

/// One thread for all terminals: flushes pending output every [`FLUSH_INTERVAL`].
fn spawn_flusher(terminals: Weak<Mutex<HashMap<Uuid, Shared>>>) {
    std::thread::Builder::new()
        .name("sonata-pty-flusher".into())
        .spawn(move || {
            loop {
                std::thread::sleep(FLUSH_INTERVAL);
                let Some(registry) = terminals.upgrade() else {
                    break;
                };
                let all: Vec<Shared> = lock(&registry).values().cloned().collect();
                drop(registry);
                for terminal in all {
                    lock(&terminal).flush();
                }
            }
        })
        .expect("spawn flusher thread");
}

fn new_token() -> String {
    use rand::RngCore;
    let mut bytes = [0u8; 32];
    rand::rng().fill_bytes(&mut bytes);
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    a.len() == b.len() && a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

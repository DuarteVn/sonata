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
use crate::model::node::Conversation;

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
    /// Sonata's shell startup folder (`run/`, with `zsh/` and `bash/`); see `terminal::pty`.
    pub shell_helpers: Option<PathBuf>,
    /// The node's stored conversation; every start offers it once (feature 009, research R3).
    pub resume: Option<Conversation>,
    /// The terminal's shell on Windows (feature 012); `None` = the default shell.
    pub shell_choice: Option<crate::model::node::TerminalShell>,
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
    /// Conversation offered to the first `sonata run` of the current start (feature 009).
    resume_offer: Option<Conversation>,
    /// An agent hook reported from this process (feature 010, research R5).
    agent_hooked: bool,
    /// Between the agent's turn-start and turn-end reports.
    turn_active: bool,
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

    /// Busy as the canvas and `sonata list` show it: a hooked Kimi follows its turns, because
    /// Kimi owns the terminal even while it waits for input (feature 010, research R5).
    fn working(&self) -> bool {
        let kimi = self
            .spec
            .launch_command
            .as_deref()
            .and_then(|c| c.split_whitespace().next())
            == Some("kimi");
        if kimi && self.agent_hooked && matches!(self.status, RunStatus::Running { .. }) {
            self.turn_active
        } else {
            self.busy()
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
            resume_offer: None,
            agent_hooked: false,
            turn_active: false,
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
            t.resume_offer = t.spec.resume.clone();
            t.agent_hooked = false;
            t.turn_active = false;
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
            shell_helpers: spec.shell_helpers.as_deref(),
            shell_choice: spec.shell_choice.as_ref(),
        })?;
        let pid = spawned.pid;
        let launch = spec
            .launch_command
            .as_deref()
            .map(str::trim)
            .filter(|c| !c.is_empty())
            .map(|c| pty::wrap_launch_command(c, spec.wrap_agents));
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
        if let Some(command) = launch {
            spawn_launcher(Arc::downgrade(terminal), generation, command);
        }
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

    /// Conversation offered at the next start or restart.
    pub fn set_resume(&self, id: Uuid, conversation: Option<Conversation>) {
        if let Ok(terminal) = self.get(id) {
            lock(&terminal).spec.resume = conversation;
        }
    }

    /// The current start's resume offer; returns it once, then `None` until the next start.
    pub fn take_resume_offer(&self, id: Uuid) -> Option<Conversation> {
        self.get(id).ok().and_then(|t| lock(&t).resume_offer.take())
    }

    /// The Windows shell used at the next start or restart (feature 012).
    pub fn set_shell_choice(&self, id: Uuid, shell: Option<crate::model::node::TerminalShell>) {
        if let Ok(terminal) = self.get(id) {
            lock(&terminal).spec.shell_choice = shell;
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

    /// Busy for display (`sonata list`, activity); confirmations keep using [`Self::is_busy`].
    pub fn is_working(&self, id: Uuid) -> bool {
        self.get(id).is_ok_and(|t| lock(&t).working())
    }

    /// Records an agent hook report from the terminal's current process: it is hooked from now
    /// on, and `turn` (when given) starts or ends its turn (feature 010).
    pub fn report_agent_hook(&self, id: Uuid, turn: Option<bool>) {
        if let Ok(t) = self.get(id) {
            let mut t = lock(&t);
            t.agent_hooked = true;
            if let Some(active) = turn {
                t.turn_active = active;
            }
        }
    }

    pub fn agent_hooked(&self, id: Uuid) -> bool {
        self.get(id).is_ok_and(|t| lock(&t).agent_hooked)
    }

    pub fn turn_active(&self, id: Uuid) -> bool {
        self.get(id).is_ok_and(|t| lock(&t).turn_active)
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

    /// The prompt as shown before anything is typed: up to two rows above the cursor and the
    /// cursor's row up to the cursor (multi-line prompts print a line above the input).
    pub fn prompt_block(&self, id: Uuid) -> Option<Vec<String>> {
        self.get(id).ok().map(|t| {
            let t = lock(&t);
            let (row, col) = t.screen.cursor();
            let lines = t.screen.preview_lines();
            let row = row as usize;
            let mut block: Vec<String> = (row.saturating_sub(2)..row)
                .map(|r| lines.get(r).cloned().unwrap_or_default())
                .collect();
            block.push(
                lines
                    .get(row)
                    .map(|l| l.chars().take(col as usize).collect())
                    .unwrap_or_default(),
            );
            block
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
        busy: t.working(),
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
            let mut tail: Vec<u8> = Vec::new();
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
                // ConPTY asks for the cursor position at start and waits for the answer. With no
                // view attached, nobody else answers, so Sonata does (feature 012).
                if cfg!(windows) && t.sink.is_none() && asks_cursor_position(&tail, &buf[..n]) {
                    let (row, col) = t.screen.cursor();
                    if let Some(w) = t.writer.as_mut() {
                        let _ = w.write_all(format!("\x1b[{};{}R", row + 1, col + 1).as_bytes());
                        let _ = w.flush();
                    }
                }
                tail.clear();
                tail.extend_from_slice(&buf[n.saturating_sub(3)..n]);
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

/// Whether the output asks for the cursor position (`ESC [ 6 n`), also when the sequence is
/// split between the previous chunk's last bytes (`tail`) and this one.
pub fn asks_cursor_position(tail: &[u8], chunk: &[u8]) -> bool {
    const DSR: &[u8] = b"\x1b[6n";
    if chunk.windows(DSR.len()).any(|w| w == DSR) {
        return true;
    }
    let mut joined = tail.to_vec();
    joined.extend_from_slice(&chunk[..chunk.len().min(DSR.len() - 1)]);
    joined.windows(DSR.len()).any(|w| w == DSR)
}

/// Types the launch command once the shell is ready: it printed something and then stayed quiet
/// for a moment (at most [`LAUNCH_WAIT`]).
fn spawn_launcher(terminal: Weak<Mutex<Terminal>>, generation: u64, command: String) {
    std::thread::Builder::new()
        .name("sonata-launch".into())
        .spawn(move || {
            let start = std::time::Instant::now();
            loop {
                std::thread::sleep(Duration::from_millis(50));
                let Some(terminal) = terminal.upgrade() else {
                    return;
                };
                let mut t = lock(&terminal);
                if t.generation != generation {
                    return;
                }
                let ready = t.output_seq > 0 && t.last_output.elapsed() >= LAUNCH_QUIET;
                if ready || start.elapsed() >= LAUNCH_WAIT {
                    if let Some(w) = t.writer.as_mut() {
                        let _ = w.write_all(format!("{command}\r").as_bytes());
                        let _ = w.flush();
                    }
                    return;
                }
            }
        })
        .expect("spawn launcher thread");
}

const LAUNCH_QUIET: Duration = Duration::from_millis(300);
const LAUNCH_WAIT: Duration = Duration::from_secs(10);

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

#[cfg(test)]
mod tests {
    use super::asks_cursor_position;

    #[test]
    fn cursor_position_requests_are_found_across_chunks() {
        assert!(asks_cursor_position(b"", b"abc\x1b[6ndef"));
        assert!(asks_cursor_position(b"x\x1b[", b"6n"));
        assert!(asks_cursor_position(b"\x1b", b"[6n"));
        assert!(!asks_cursor_position(b"", b"\x1b[5n"));
        assert!(!asks_cursor_position(b"\x1b[6", b"m"));
        assert!(!asks_cursor_position(b"", b""));
    }
}

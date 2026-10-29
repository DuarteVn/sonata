//! Asks between terminals (research 003 R8): one queue per target, loop detection, deadlines,
//! end-of-turn detection, replies, cancellation, and a short exchange log per terminal.

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Mutex;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::{Duration, Instant};

use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

use super::extract::clean_answer;
use super::{MAX_RESULT_BYTES, cap_text};
use crate::App;
use crate::events::{AskEvent, CoreEvent};
use crate::terminal::manager::RunStatus;

pub const TICK: Duration = Duration::from_millis(200);
/// Quiet time that ends an agent's turn when it has no end-of-turn signal (FR-017).
pub const QUIET_AFTER_OUTPUT: Duration = Duration::from_secs(3);
/// A shell command is done when the prompt is back and the screen is quiet this long.
pub const SHELL_QUIET: Duration = Duration::from_millis(300);
/// Ends an ask whose target never printed anything.
pub const SILENT_LIMIT: Duration = Duration::from_secs(30);
/// Fallback for agents with an end-of-turn signal, in case the signal never arrives.
pub const HOOKED_QUIET_FALLBACK: Duration = Duration::from_secs(20);
/// A target must become ready within this time.
pub const READY_LIMIT: Duration = Duration::from_secs(60);
/// A target is ready when its screen has been quiet this long.
pub const READY_QUIET: Duration = Duration::from_millis(1500);
pub const LOG_KEEP: usize = 50;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum AskState {
    Queued,
    Waiting,
    Answered,
    Replied,
    TimedOut,
    Failed,
    Cancelled,
}

impl AskState {
    pub fn done(self) -> bool {
        !matches!(self, Self::Queued | Self::Waiting)
    }
}

/// How the end of the target's turn is detected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetKind {
    /// Claude Code or Codex started through `sonata run`: wait for its end-of-turn signal.
    Hooked,
    /// A plain shell: done when the command finished and the prompt is back.
    Shell,
    /// Anything else: quiet-terminal heuristic.
    Other,
    /// Kimi Code, whose hooks exist only after the user sets them up (feature 010, research R5):
    /// like `Other` until the target's process reports a hook, then like `Hooked`, and never
    /// cut off by quiet while a turn is active.
    Probing,
}

#[derive(Debug, Clone)]
pub struct AskResult {
    pub state: AskState,
    pub text: String,
    pub alternate_screen: bool,
    pub seconds: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LogEntry {
    pub at: DateTime<Utc>,
    pub direction: &'static str,
    pub other: Uuid,
    pub first_line: String,
    pub outcome: AskState,
    pub result_chars: usize,
}

struct Ask {
    from: Uuid,
    to: Uuid,
    message: String,
    timeout: Duration,
    kind: TargetKind,
    state: AskState,
    created: Instant,
    sent_at: Option<Instant>,
    deadline: Option<Instant>,
    anchor: u64,
    prompt: Vec<String>,
    last_seq: u64,
    last_output: Instant,
    saw_output: bool,
    tx: Sender<AskResult>,
}

#[derive(Default)]
struct State {
    asks: HashMap<Uuid, Ask>,
    /// Pending asks per target, oldest first; the front one is the active ask.
    queues: HashMap<Uuid, VecDeque<Uuid>>,
    logs: HashMap<Uuid, VecDeque<LogEntry>>,
}

#[derive(Default)]
pub struct Broker {
    state: Mutex<State>,
}

pub struct Submitted {
    pub id: Uuid,
    pub rx: Receiver<AskResult>,
    pub position: usize,
}

fn first_line(s: &str) -> String {
    let line = s
        .lines()
        .find(|l| !l.trim().is_empty())
        .unwrap_or_default()
        .trim();
    line.chars().take(80).collect()
}

impl Broker {
    fn lock(&self) -> std::sync::MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Queues an ask. Refuses it when the target is (directly or through others) waiting for the
    /// asker, since the asker cannot answer while it waits (FR-014). The error lists the chain.
    pub fn submit(
        &self,
        app: &App,
        from: Uuid,
        to: Uuid,
        message: String,
        timeout: Duration,
        kind: TargetKind,
    ) -> Result<Submitted, Vec<Uuid>> {
        let mut st = self.lock();
        if let Some(chain) = find_chain(&st, to, from) {
            return Err(chain);
        }
        let id = Uuid::new_v4();
        let (tx, rx) = channel();
        let now = Instant::now();
        st.asks.insert(
            id,
            Ask {
                from,
                to,
                message,
                timeout,
                kind,
                state: AskState::Queued,
                created: now,
                sent_at: None,
                deadline: None,
                anchor: 0,
                prompt: Vec::new(),
                last_seq: 0,
                last_output: now,
                saw_output: false,
                tx,
            },
        );
        let queue = st.queues.entry(to).or_default();
        queue.push_back(id);
        let position = queue.len() - 1;
        drop(st);
        app.emit(CoreEvent::AskState(AskEvent {
            ask_id: id,
            from,
            to,
            state: AskState::Queued,
        }));
        Ok(Submitted { id, rx, position })
    }

    /// Completes the active ask to `target` with an explicit reply (FR-008).
    pub fn reply(&self, app: &App, target: Uuid, text: String) -> bool {
        let id = {
            let st = self.lock();
            st.queues
                .get(&target)
                .and_then(|q| q.front())
                .copied()
                .filter(|id| st.asks[id].state == AskState::Waiting)
        };
        match id {
            Some(id) => {
                self.finish(app, id, AskState::Replied, Some(text));
                true
            }
            None => false,
        }
    }

    /// End-of-turn signal from a hooked agent. An empty text falls back to the screen.
    pub fn turn_complete(&self, app: &App, target: Uuid, text: Option<String>) {
        let id = {
            let st = self.lock();
            st.queues
                .get(&target)
                .and_then(|q| q.front())
                .copied()
                .filter(|id| st.asks[id].state == AskState::Waiting)
        };
        if let Some(id) = id {
            let text = text.filter(|t| !t.trim().is_empty());
            self.finish(app, id, AskState::Answered, text);
        }
    }

    /// Cancels the asks a terminal made (it closed or its command was interrupted).
    pub fn cancel_from(&self, app: &App, asker: Uuid) {
        let ids: Vec<Uuid> = self
            .lock()
            .asks
            .iter()
            .filter(|(_, a)| a.from == asker)
            .map(|(id, _)| *id)
            .collect();
        for id in ids {
            self.finish(app, id, AskState::Cancelled, Some(String::new()));
        }
    }

    pub fn cancel(&self, app: &App, id: Uuid) {
        self.finish(app, id, AskState::Cancelled, Some(String::new()));
    }

    /// Fails every pending ask (the app is quitting).
    pub fn shutdown(&self, app: &App) {
        let ids: Vec<Uuid> = self.lock().asks.keys().copied().collect();
        for id in ids {
            self.finish(app, id, AskState::Failed, Some("Sonata quit.".into()));
        }
    }

    pub fn log(&self, node: Uuid) -> Vec<LogEntry> {
        self.lock()
            .logs
            .get(&node)
            .map(|l| l.iter().cloned().collect())
            .unwrap_or_default()
    }

    /// Pending asks to a terminal (front first), for the "asked by" marker.
    pub fn askers_of(&self, target: Uuid) -> Vec<Uuid> {
        let st = self.lock();
        st.queues
            .get(&target)
            .map(|q| q.iter().map(|id| st.asks[id].from).collect())
            .unwrap_or_default()
    }

    fn partial(&self, app: &App, ask_id: Uuid) -> (String, bool) {
        let (to, anchor, message, prompt) = {
            let st = self.lock();
            let Some(a) = st.asks.get(&ask_id) else {
                return (String::new(), false);
            };
            if a.sent_at.is_none() {
                return (String::new(), false);
            }
            (a.to, a.anchor, a.message.clone(), a.prompt.clone())
        };
        match app.terminals().rows_since(to, anchor) {
            Some((rows, true)) => (rows.join("\n").trim_end().to_string(), true),
            Some((rows, false)) => (clean_answer(rows, &message, &prompt), false),
            None => (String::new(), false),
        }
    }

    fn finish(&self, app: &App, id: Uuid, state: AskState, text: Option<String>) {
        let (text, alternate) = match text {
            Some(t) => (t, false),
            None => self.partial(app, id),
        };
        let mut st = self.lock();
        let Some(ask) = st.asks.remove(&id) else {
            return;
        };
        if let Some(q) = st.queues.get_mut(&ask.to) {
            q.retain(|x| *x != id);
        }
        let seconds = ask.sent_at.unwrap_or(ask.created).elapsed().as_secs();
        let text = cap_text(text, MAX_RESULT_BYTES);
        let line = first_line(&ask.message);
        for (node, direction, other) in [(ask.from, "out", ask.to), (ask.to, "in", ask.from)] {
            let log = st.logs.entry(node).or_default();
            log.push_front(LogEntry {
                at: Utc::now(),
                direction,
                other,
                first_line: line.clone(),
                outcome: state,
                result_chars: text.chars().count(),
            });
            log.truncate(LOG_KEEP);
        }
        drop(st);
        let _ = ask.tx.send(AskResult {
            state,
            text,
            alternate_screen: alternate,
            seconds,
        });
        app.emit(CoreEvent::AskState(AskEvent {
            ask_id: id,
            from: ask.from,
            to: ask.to,
            state,
        }));
    }

    /// Called every [`TICK`]: sends queued asks to ready targets and ends finished turns.
    pub fn tick(&self, app: &App) {
        let fronts: Vec<Uuid> = {
            let st = self.lock();
            st.queues
                .values()
                .filter_map(|q| q.front().copied())
                .collect()
        };
        let now = Instant::now();
        for id in fronts {
            let snapshot = {
                let st = self.lock();
                st.asks.get(&id).map(|a| {
                    (
                        a.to,
                        a.state,
                        a.kind,
                        a.created,
                        a.sent_at,
                        a.deadline,
                        a.last_seq,
                        a.last_output,
                        a.saw_output,
                        a.message.clone(),
                    )
                })
            };
            let Some((
                to,
                state,
                kind,
                created,
                sent_at,
                deadline,
                last_seq,
                last_output,
                saw_output,
                message,
            )) = snapshot
            else {
                continue;
            };
            let terms = app.terminals();
            let status = terms.status(to);
            match state {
                AskState::Queued => {
                    if now.duration_since(created) > READY_LIMIT {
                        self.finish(
                            app,
                            id,
                            AskState::Failed,
                            Some("The terminal did not become ready in time.".into()),
                        );
                        continue;
                    }
                    if !matches!(status, Some(RunStatus::Running { .. }))
                        || !app.terminal_ready(to, READY_QUIET)
                    {
                        continue;
                    }
                    let anchor = terms.absolute_row(to).unwrap_or(0);
                    let prompt = terms.prompt_block(to).unwrap_or_default();
                    if let Err(e) = terms.deliver(to, &message) {
                        self.finish(
                            app,
                            id,
                            AskState::Failed,
                            Some(format!("Could not send the message: {e}")),
                        );
                        continue;
                    }
                    let seq = terms.output_seq(to).unwrap_or(0);
                    let from;
                    {
                        let mut st = self.lock();
                        let Some(a) = st.asks.get_mut(&id) else {
                            continue;
                        };
                        a.state = AskState::Waiting;
                        a.sent_at = Some(Instant::now());
                        a.deadline = Some(Instant::now() + a.timeout);
                        a.anchor = anchor;
                        a.prompt = prompt;
                        a.last_seq = seq;
                        a.last_output = Instant::now();
                        from = a.from;
                    }
                    app.emit(CoreEvent::AskState(AskEvent {
                        ask_id: id,
                        from,
                        to,
                        state: AskState::Waiting,
                    }));
                }
                AskState::Waiting => {
                    let sent_at = sent_at.unwrap_or(now);
                    if !matches!(status, Some(RunStatus::Running { .. })) {
                        let name = app.node_name(to).unwrap_or_else(|| "The terminal".into());
                        let (partial, _) = self.partial(app, id);
                        let text = if partial.is_empty() {
                            format!("{name} stopped.")
                        } else {
                            format!("{partial}\n\n[{name} stopped]")
                        };
                        self.finish(app, id, AskState::Failed, Some(text));
                        continue;
                    }
                    if deadline.is_some_and(|d| now >= d) {
                        self.finish(app, id, AskState::TimedOut, None);
                        continue;
                    }
                    let seq = terms.output_seq(to).unwrap_or(last_seq);
                    let (saw, last) = if seq != last_seq {
                        let mut st = self.lock();
                        if let Some(a) = st.asks.get_mut(&id) {
                            a.last_seq = seq;
                            a.last_output = now;
                            a.saw_output = true;
                        }
                        (true, now)
                    } else {
                        (saw_output, last_output)
                    };
                    let quiet = now.duration_since(last);
                    let done = match kind {
                        TargetKind::Hooked => saw && quiet >= HOOKED_QUIET_FALLBACK,
                        TargetKind::Probing if terms.agent_hooked(to) => {
                            saw && quiet >= HOOKED_QUIET_FALLBACK && !terms.turn_active(to)
                        }
                        TargetKind::Shell => saw && quiet >= SHELL_QUIET && !terms.is_busy(to),
                        TargetKind::Other | TargetKind::Probing => {
                            (saw && quiet >= QUIET_AFTER_OUTPUT)
                                || (!saw && now.duration_since(sent_at) >= SILENT_LIMIT)
                        }
                    };
                    if done {
                        self.finish(app, id, AskState::Answered, None);
                    }
                }
                _ => {}
            }
        }
    }
}

/// Path of waiting asks from `start` to `goal` (start waits on X, X waits on … goal).
fn find_chain(st: &State, start: Uuid, goal: Uuid) -> Option<Vec<Uuid>> {
    let mut edges: HashMap<Uuid, Vec<Uuid>> = HashMap::new();
    for a in st.asks.values() {
        edges.entry(a.from).or_default().push(a.to);
    }
    let mut seen = HashSet::new();
    let mut stack = vec![vec![start]];
    while let Some(path) = stack.pop() {
        let last = *path.last().expect("non-empty");
        if last == goal && path.len() > 1 {
            return Some(path);
        }
        if !seen.insert(last) {
            continue;
        }
        for next in edges.get(&last).into_iter().flatten() {
            let mut p = path.clone();
            p.push(*next);
            stack.push(p);
        }
    }
    None
}

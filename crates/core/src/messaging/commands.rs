//! Handlers for every `sonata` command (contracts/cli.md). The caller is the terminal whose
//! identity was checked by the server; every target must be linked to it (FR-004).

use std::path::PathBuf;
use std::time::Duration;

use serde_json::json;
use uuid::Uuid;

use super::broker::{AskResult, AskState, TargetKind};
use super::{
    EXIT_DENIED, EXIT_ERROR, EXIT_NOT_FOUND, EXIT_TIMEOUT, EXIT_USAGE, MAX_MESSAGE_BYTES, Reply,
    Request,
};
use crate::App;
use crate::lanes::confirm::Answer;
use crate::lanes::ops::Action;
use crate::model::node::{BrowserContent, Node, NodeContent};
use crate::terminal::manager::RunStatus;

pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(10 * 60);
const MIN_TIMEOUT: Duration = Duration::from_secs(5);
const MAX_TIMEOUT: Duration = Duration::from_secs(60 * 60);
pub const PROGRESS_EVERY: Duration = Duration::from_secs(15);
const LANE_TIMEOUT: Duration = Duration::from_secs(5 * 60);
const LANE_TIMEOUT_RANGE: std::ops::RangeInclusive<Duration> =
    Duration::from_secs(60)..=Duration::from_secs(30 * 60);

pub struct Ctx<'a> {
    pub app: &'a App,
    pub me: Uuid,
    pub ws: Uuid,
    pub cwd: String,
}

pub fn handle(app: &App, req: &Request, progress: &mut dyn FnMut(&str)) -> Reply {
    let Ok(ws) = app.workspace_of_node(req.terminal_id) else {
        return Reply::err(EXIT_NOT_FOUND, "This terminal is no longer on the canvas.");
    };
    let ctx = Ctx {
        app,
        me: req.terminal_id,
        ws,
        cwd: req.cwd.clone(),
    };
    let args: Vec<&str> = req.args.iter().map(String::as_str).collect();
    let stdin = req.stdin.clone();
    match args.as_slice() {
        ["list", ..] => list(&ctx),
        ["ask", rest @ ..] => ask(&ctx, rest, stdin, progress),
        ["check", name] => check(&ctx, name),
        ["reply", rest @ ..] => reply(&ctx, rest, stdin),
        ["note", "read", note] => note_read(&ctx, note),
        ["note", "write", rest @ ..] => note_write(&ctx, rest, stdin),
        ["note", "new", rest @ ..] => {
            orchestrator(&ctx).unwrap_or_else(|| note_new(&ctx, rest, stdin))
        }
        ["browser", "new", rest @ ..] => {
            orchestrator(&ctx).unwrap_or_else(|| browser_new(&ctx, rest))
        }
        ["browser", rest @ ..] => browser(&ctx, rest),
        ["lane", "status", rest @ ..] => lane_status(&ctx, rest),
        ["lane", "land", rest @ ..] => lane_request(&ctx, Action::Land, rest, progress),
        ["lane", "remove", rest @ ..] => lane_request(&ctx, Action::Remove, rest, progress),
        ["notify", rest @ ..] if !rest.is_empty() || stdin.is_some() => {
            notify(&ctx, &stdin.unwrap_or_else(|| rest.join(" ")))
        }
        ["recruit", rest @ ..] => orchestrator(&ctx).unwrap_or_else(|| recruit(&ctx, rest)),
        ["dismiss", name] => orchestrator(&ctx).unwrap_or_else(|| dismiss(&ctx, name, progress)),
        ["link", a, b] => orchestrator(&ctx).unwrap_or_else(|| link(&ctx, a, b, true)),
        ["unlink", a, b] => orchestrator(&ctx).unwrap_or_else(|| link(&ctx, a, b, false)),
        ["_session", agent, hook] => session(&ctx, agent, hook),
        ["_turn", "complete"] => {
            app.broker().turn_complete(app, ctx.me, stdin);
            Reply::ok("")
        }
        ["_turn", "start"] => Reply::ok(""),
        _ => Reply::err(
            EXIT_USAGE,
            format!(
                "Unknown command: sonata {}. Run `sonata help`.",
                req.args.join(" ")
            ),
        ),
    }
}

// ---- Lookups ----

fn me_node(ctx: &Ctx<'_>) -> Option<Node> {
    ctx.app
        .workspaces()
        .workspace(ctx.ws)?
        .node(ctx.me)
        .cloned()
}

fn linked(ctx: &Ctx<'_>) -> Vec<Node> {
    let m = ctx.app.workspaces();
    let Some(w) = m.workspace(ctx.ws) else {
        return Vec::new();
    };
    w.links
        .iter()
        .filter_map(|l| l.other(ctx.me))
        .filter_map(|id| w.node(id).cloned())
        .collect()
}

pub fn note_title(text: &str) -> String {
    let line = text
        .lines()
        .find(|l| l.trim_start().starts_with('#'))
        .map(|l| l.trim_start_matches('#').trim())
        .or_else(|| text.lines().map(str::trim).find(|l| !l.is_empty()))
        .unwrap_or("(empty note)");
    line.chars().take(60).collect()
}

pub fn short_id(id: Uuid) -> String {
    format!("n-{}", &id.simple().to_string()[..4])
}

fn linked_names(ctx: &Ctx<'_>) -> String {
    let names: Vec<String> = linked(ctx).iter().map(|n| display_name(ctx, n)).collect();
    if names.is_empty() {
        "nothing (link items on the canvas first)".into()
    } else {
        names.join(", ")
    }
}

fn display_name(ctx: &Ctx<'_>, n: &Node) -> String {
    match &n.content {
        NodeContent::Terminal(t) => t.name.clone(),
        NodeContent::Browser(b) => browser_title(b),
        NodeContent::Note(_) => {
            note_title(&ctx.app.note_read(n.id).map(|v| v.text).unwrap_or_default())
        }
    }
}

/// A browser is named by its page title, else its host.
pub fn browser_title(b: &BrowserContent) -> String {
    let title = b.title.as_deref().map(str::trim).filter(|t| !t.is_empty());
    let fallback = || {
        let rest = b.url.split_once("://").map(|(_, r)| r).unwrap_or(&b.url);
        rest.split('/').next().unwrap_or("browser").to_string()
    };
    title
        .map(|t| t.chars().take(60).collect())
        .unwrap_or_else(fallback)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Terminal,
    Note,
    Browser,
}

fn kind_of(n: &Node) -> Kind {
    match n.content {
        NodeContent::Terminal(_) => Kind::Terminal,
        NodeContent::Note(_) => Kind::Note,
        NodeContent::Browser(_) => Kind::Browser,
    }
}

/// Short id printed by `sonata list` for notes (`n-xxxx`) and browsers (`b-xxxx`).
fn item_id(n: &Node) -> Option<String> {
    match kind_of(n) {
        Kind::Note => Some(short_id(n.id)),
        Kind::Browser => Some(crate::browser::short_id(n.id)),
        Kind::Terminal => None,
    }
}

fn resolve(ctx: &Ctx<'_>, name: &str, kind: Kind) -> Result<Node, Reply> {
    let wanted = name.trim().to_lowercase();
    let matches: Vec<Node> = linked(ctx)
        .into_iter()
        .filter(|n| kind_of(n) == kind)
        .filter(|n| {
            display_name(ctx, n).to_lowercase() == wanted
                || item_id(n).is_some_and(|id| id == wanted)
        })
        .collect();
    let label = match kind {
        Kind::Terminal => "terminal",
        Kind::Note => "note",
        Kind::Browser => "browser",
    };
    match matches.len() {
        1 => Ok(matches.into_iter().next().expect("one")),
        0 => Err(Reply::err(
            EXIT_DENIED,
            format!(
                "No linked {label} named \"{name}\". You are linked to: {}.",
                linked_names(ctx)
            ),
        )),
        _ => Err(Reply::err(
            EXIT_USAGE,
            format!(
                "\"{name}\" matches several linked items; use the short id from `sonata list` or rename them on the canvas."
            ),
        )),
    }
}

fn state_label(app: &App, id: Uuid) -> &'static str {
    match app.terminals().status(id) {
        Some(RunStatus::Running { .. }) if app.terminals().is_busy(id) => "working",
        Some(RunStatus::Running { .. }) => "idle",
        Some(RunStatus::Exited { .. }) => "exited",
        Some(RunStatus::Starting) => "starting",
        None => "not started",
    }
}

pub fn agent_label(command: Option<&str>) -> String {
    command
        .and_then(|c| c.split_whitespace().next())
        .unwrap_or("shell")
        .to_string()
}

pub fn target_kind(node: &Node) -> TargetKind {
    let cmd = node.terminal().and_then(|t| t.launch_command.as_deref());
    match cmd.and_then(|c| c.split_whitespace().next()) {
        None => TargetKind::Shell,
        Some(a) if crate::terminal::pty::WRAPPED_AGENTS.contains(&a) => TargetKind::Hooked,
        Some(_) => TargetKind::Other,
    }
}

// ---- Everyone ----

fn list(ctx: &Ctx<'_>) -> Reply {
    let Some(me) = me_node(ctx) else {
        return Reply::err(EXIT_NOT_FOUND, "This terminal is gone.");
    };
    let t = me
        .terminal()
        .cloned()
        .unwrap_or_else(|| crate::model::node::TerminalContent::new("?".into()));
    let ws_name = ctx
        .app
        .workspaces()
        .workspace(ctx.ws)
        .map(|w| w.name.clone())
        .unwrap_or_default();
    let mut text = format!(
        "You are \"{}\" in workspace \"{ws_name}\"{}\n",
        t.name,
        if t.orchestrator {
            " · Orchestrator"
        } else {
            ""
        }
    );
    if let Some(role) = &t.role {
        text.push_str(&format!(
            "Role: {}\n",
            role.text.lines().next().unwrap_or_default()
        ));
    }
    let items = linked(ctx);
    let mut data = Vec::new();
    if items.is_empty() {
        text.push_str(
            "Linked: nothing yet. Ask the user to link you to other items on the canvas.",
        );
    } else {
        text.push_str("Linked:\n");
        for n in &items {
            match n.terminal() {
                Some(lt) => {
                    let state = state_label(ctx.app, n.id);
                    let agent = agent_label(lt.launch_command.as_deref());
                    text.push_str(&format!(
                        "  terminal  {:<16} {:<10} {agent}\n",
                        lt.name, state
                    ));
                    data.push(json!({"kind": "terminal", "name": lt.name, "state": state, "agent": agent, "orchestrator": lt.orchestrator}));
                }
                None => {
                    let title = display_name(ctx, n);
                    let id = item_id(n).unwrap_or_default();
                    if let Some(b) = n.browser() {
                        text.push_str(&format!("  browser   {:<16} ({id})  {}\n", title, b.url));
                        data.push(
                            json!({"kind": "browser", "name": title, "id": id, "url": b.url}),
                        );
                    } else {
                        text.push_str(&format!("  note      {:<16} ({id})\n", title));
                        data.push(json!({"kind": "note", "name": title, "id": id}));
                    }
                }
            }
        }
    }
    Reply::ok(text.trim_end()).with_data(json!({"name": t.name, "workspace": ws_name, "orchestrator": t.orchestrator, "role": t.role.map(|r| r.text), "linked": data}))
}

fn parse_duration(s: &str) -> Option<Duration> {
    let (num, unit) = s.split_at(s.find(|c: char| !c.is_ascii_digit()).unwrap_or(s.len()));
    let n: u64 = num.parse().ok()?;
    let secs = match unit {
        "" | "s" => n,
        "m" => n * 60,
        "h" => n * 3600,
        _ => return None,
    };
    Some(Duration::from_secs(secs))
}

/// Key names for `--keys` (research 003 R7).
pub fn keys_to_bytes(spec: &str) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    let mut rest = spec.trim();
    while !rest.is_empty() {
        if let Some(stripped) = rest.strip_prefix('"') {
            let end = stripped.find('"').ok_or("Unclosed quote in --keys.")?;
            out.extend_from_slice(&stripped.as_bytes()[..end]);
            rest = stripped[end + 1..].trim_start();
            continue;
        }
        let (word, tail) = rest.split_at(rest.find(char::is_whitespace).unwrap_or(rest.len()));
        rest = tail.trim_start();
        let bytes: &[u8] = match word.to_lowercase().as_str() {
            "enter" | "return" => b"\r",
            "tab" => b"\t",
            "esc" | "escape" => b"\x1b",
            "up" => b"\x1b[A",
            "down" => b"\x1b[B",
            "right" => b"\x1b[C",
            "left" => b"\x1b[D",
            "space" => b" ",
            "backspace" => b"\x7f",
            "ctrl-c" => b"\x03",
            "ctrl-d" => b"\x04",
            w if w.len() == 1 => word.as_bytes(),
            _ => {
                return Err(format!(
                    "Unknown key \"{word}\". Use enter, tab, esc, up, down, left, right, space, backspace, ctrl-c, ctrl-d, a single character, or \"quoted text\"."
                ));
            }
        };
        out.extend_from_slice(bytes);
    }
    Ok(out)
}

/// Makes sure a target terminal runs: loads its workspace or restarts it (FR-015).
fn ensure_running(ctx: &Ctx<'_>, target: Uuid) {
    match ctx.app.terminals().status(target) {
        None => ctx.app.load_workspace_terminals(ctx.ws),
        Some(RunStatus::Exited { .. }) => {
            let _ = ctx.app.restart_terminal(target, true);
        }
        _ => {}
    }
}

fn ask(
    ctx: &Ctx<'_>,
    rest: &[&str],
    stdin: Option<String>,
    progress: &mut dyn FnMut(&str),
) -> Reply {
    let mut all = false;
    let mut keys: Option<String> = None;
    let mut timeout = DEFAULT_TIMEOUT;
    let mut positional = Vec::new();
    let mut it = rest.iter();
    while let Some(a) = it.next() {
        match *a {
            "--all" => all = true,
            "--keys" => keys = it.next().map(|s| s.to_string()),
            "--timeout" => match it.next().and_then(|v| parse_duration(v)) {
                Some(d) if (MIN_TIMEOUT..=MAX_TIMEOUT).contains(&d) => timeout = d,
                _ => {
                    return Reply::err(EXIT_USAGE, "--timeout takes 5s to 60m, e.g. 90s, 10m, 1h.");
                }
            },
            other => positional.push(other.to_string()),
        }
    }
    let targets: Vec<String> = if all {
        Vec::new()
    } else if positional.is_empty() {
        return Reply::err(EXIT_USAGE, "Usage: sonata ask <name> \"<message>\"");
    } else {
        positional
            .remove(0)
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    };

    if let Some(spec) = keys {
        let [name] = targets.as_slice() else {
            return Reply::err(EXIT_USAGE, "--keys needs exactly one terminal.");
        };
        let node = match resolve(ctx, name, Kind::Terminal) {
            Ok(n) => n,
            Err(r) => return r,
        };
        let bytes = match keys_to_bytes(&spec) {
            Ok(b) => b,
            Err(e) => return Reply::err(EXIT_USAGE, e),
        };
        return match ctx.app.terminals().write(node.id, &bytes) {
            Ok(()) => Reply::ok(format!("Sent keys to {name}.")),
            Err(e) => Reply::err(EXIT_ERROR, format!("Could not send keys: {e}")),
        };
    }

    let message = stdin.unwrap_or_else(|| positional.join(" "));
    if message.trim().is_empty() {
        return Reply::err(EXIT_USAGE, "The message is empty.");
    }
    if message.len() > MAX_MESSAGE_BYTES {
        return Reply::err(
            EXIT_USAGE,
            "Messages can be at most 100 KB. Put long content in a linked note instead.",
        );
    }
    let nodes: Vec<Node> = if all {
        let ts: Vec<Node> = linked(ctx)
            .into_iter()
            .filter(|n| kind_of(n) == Kind::Terminal)
            .collect();
        if ts.is_empty() {
            return Reply::err(EXIT_DENIED, "You are not linked to any terminal.");
        }
        ts
    } else {
        let mut v = Vec::new();
        for t in &targets {
            match resolve(ctx, t, Kind::Terminal) {
                Ok(n) => v.push(n),
                Err(r) => return r,
            }
        }
        v
    };

    let mut pending = Vec::new();
    for n in &nodes {
        ensure_running(ctx, n.id);
        match ctx.app.broker().submit(
            ctx.app,
            ctx.me,
            n.id,
            message.clone(),
            timeout,
            target_kind(n),
        ) {
            Ok(s) => {
                if s.position > 0 {
                    progress(&format!(
                        "{} is busy; your message is number {} in its queue.",
                        display_name(ctx, n),
                        s.position
                    ));
                }
                pending.push((display_name(ctx, n), s))
            }
            Err(chain) => {
                for (_, s) in &pending {
                    ctx.app.broker().cancel(ctx.app, s.id);
                }
                let names: Vec<String> = chain
                    .iter()
                    .map(|id| ctx.app.node_name(*id).unwrap_or_else(|| id.to_string()))
                    .collect();
                return Reply::err(
                    EXIT_DENIED,
                    format!(
                        "Refused: this would wait in a loop ({} → you). The other side is waiting for you, so answer it first.",
                        names.join(" → ")
                    ),
                );
            }
        }
    }

    // Wait for every answer, reporting progress, and cancel if the caller goes away.
    let started = std::time::Instant::now();
    let mut results: Vec<(String, AskResult)> = Vec::new();
    for (name, s) in pending {
        loop {
            match s.rx.recv_timeout(PROGRESS_EVERY) {
                Ok(r) => {
                    results.push((name, r));
                    break;
                }
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => progress(&format!(
                    "{name} is working ({}s)…",
                    started.elapsed().as_secs()
                )),
                Err(_) => {
                    results.push((
                        name,
                        AskResult {
                            state: AskState::Failed,
                            text: "The ask was lost.".into(),
                            alternate_screen: false,
                            seconds: 0,
                        },
                    ));
                    break;
                }
            }
        }
    }
    let describe = |r: &AskResult| {
        match r.state {
        AskState::Answered | AskState::Replied => String::new(),
        AskState::TimedOut => "\n[timed out: still working; use `sonata check` to look, the message was not sent again]".to_string(),
        AskState::Cancelled => "\n[cancelled]".to_string(),
        _ => String::new(),
    }
    };
    let data: Vec<_> = results.iter().map(|(n, r)| json!({"name": n, "state": r.state, "seconds": r.seconds, "text": r.text, "fullScreen": r.alternate_screen})).collect();
    if results.len() == 1 {
        let (_, r) = &results[0];
        let mut text = r.text.clone();
        if r.alternate_screen {
            text.push_str("\n[full-screen program: this is its current screen]");
        }
        text.push_str(&describe(r));
        let code = match r.state {
            AskState::Answered | AskState::Replied => 0,
            AskState::TimedOut => EXIT_TIMEOUT,
            _ => EXIT_ERROR,
        };
        return Reply {
            ok: code == 0,
            exit_code: code,
            text: text.trim_start().to_string(),
            data: json!(data),
        };
    }
    let mut text = String::new();
    for (n, r) in &results {
        text.push_str(&format!(
            "## {n} ({:?}, {}s)\n{}{}\n\n",
            r.state,
            r.seconds,
            r.text,
            describe(r)
        ));
    }
    let all_ok = results
        .iter()
        .all(|(_, r)| matches!(r.state, AskState::Answered | AskState::Replied));
    Reply {
        ok: all_ok,
        exit_code: if all_ok { 0 } else { EXIT_TIMEOUT },
        text: text.trim_end().to_string(),
        data: json!(data),
    }
}

fn check(ctx: &Ctx<'_>, name: &str) -> Reply {
    let node = match resolve(ctx, name, Kind::Terminal) {
        Ok(n) => n,
        Err(r) => return r,
    };
    match ctx.app.terminals().screen_text(node.id) {
        Some(screen) => Reply::ok(screen.trim_end()),
        None => Reply::err(EXIT_NOT_FOUND, format!("{name} is not running.")),
    }
}

fn reply(ctx: &Ctx<'_>, rest: &[&str], stdin: Option<String>) -> Reply {
    let text = stdin.unwrap_or_else(|| rest.join(" "));
    if text.len() > super::MAX_RESULT_BYTES {
        return Reply::err(EXIT_USAGE, "Replies can be at most 1 MB.");
    }
    if ctx.app.broker().reply(ctx.app, ctx.me, text) {
        Reply::ok("Delivered.")
    } else {
        Reply::err(EXIT_NOT_FOUND, "No one is waiting for an answer from you.")
    }
}

fn note_read(ctx: &Ctx<'_>, name: &str) -> Reply {
    let node = match resolve(ctx, name, Kind::Note) {
        Ok(n) => n,
        Err(r) => return r,
    };
    match ctx.app.note_read(node.id) {
        Ok(v) => Reply::ok(v.text),
        Err(e) => Reply::err(EXIT_ERROR, e.message),
    }
}

fn note_write(ctx: &Ctx<'_>, rest: &[&str], stdin: Option<String>) -> Reply {
    let append = rest.contains(&"--append");
    let words: Vec<&str> = rest.iter().copied().filter(|a| *a != "--append").collect();
    let Some((name, body)) = words.split_first() else {
        return Reply::err(
            EXIT_USAGE,
            "Usage: sonata note write <note> \"<text>\" [--append]",
        );
    };
    let text = stdin.unwrap_or_else(|| body.join(" "));
    if text.len() > MAX_MESSAGE_BYTES {
        return Reply::err(EXIT_USAGE, "Note text can be at most 100 KB per write.");
    }
    let node = match resolve(ctx, name, Kind::Note) {
        Ok(n) => n,
        Err(r) => return r,
    };
    match ctx.app.note_write_from_agent(node.id, &text, append) {
        Ok(()) => Reply::ok(if append { "Appended." } else { "Written." }),
        Err(e) => Reply::err(EXIT_ERROR, e.message),
    }
}

fn notify(ctx: &Ctx<'_>, message: &str) -> Reply {
    ctx.app.emit(crate::events::CoreEvent::Notify(
        crate::events::NotifyEvent {
            node_id: ctx.me,
            title: ctx.app.node_name(ctx.me).unwrap_or_else(|| "Sonata".into()),
            message: message.chars().take(500).collect(),
        },
    ));
    Reply::ok("Notification sent.")
}

// ---- Orchestrator ----

fn orchestrator(ctx: &Ctx<'_>) -> Option<Reply> {
    let is = me_node(ctx)
        .and_then(|n| n.terminal().map(|t| t.orchestrator))
        .unwrap_or(false);
    (!is).then(|| Reply::err(EXIT_DENIED, "Only the Orchestrator can do this. Ask the user to turn on Orchestrator for this terminal."))
}

fn recruit(ctx: &Ctx<'_>, rest: &[&str]) -> Reply {
    let mut what = None;
    let mut name = None;
    let mut role = None;
    let mut role_text = None;
    let mut folder = None;
    let mut lane = false;
    let mut it = rest.iter();
    while let Some(a) = it.next() {
        match *a {
            "--lane" => lane = true,
            "--name" => name = it.next().map(|s| s.to_string()),
            "--role" => role = it.next().map(|s| s.to_string()),
            "--role-text" => role_text = it.next().map(|s| s.to_string()),
            "--folder" => folder = it.next().map(|s| s.to_string()),
            other if other.starts_with('-') => {
                return Reply::err(
                    EXIT_USAGE,
                    format!("Unknown option \"{other}\". Run `sonata help recruit`."),
                );
            }
            other if what.is_none() => what = Some(other.to_string()),
            other => {
                return Reply::err(
                    EXIT_USAGE,
                    format!("Unexpected \"{other}\". Quote commands with spaces."),
                );
            }
        }
    }
    let Some(what) = what else {
        return Reply::err(
            EXIT_USAGE,
            "Usage: sonata recruit <claude|codex|opencode|gemini|shell|\"<command>\"> [--name N] [--role R | --role-text T] [--folder F | --lane]",
        );
    };
    if lane && folder.is_some() {
        return Reply::err(
            EXIT_USAGE,
            "--lane and --folder cannot be combined: a lane always starts from the workspace folder.",
        );
    }
    let folder = folder.map(|f| {
        let p = PathBuf::from(&f);
        if p.is_absolute() {
            p
        } else {
            PathBuf::from(&ctx.cwd).join(p)
        }
    });
    match ctx.app.recruit(
        ctx.me,
        &what,
        name.as_deref(),
        role.as_deref(),
        role_text.as_deref(),
        match (folder, lane) {
            (_, true) => crate::app::RecruitPlace::Lane,
            (Some(f), false) => crate::app::RecruitPlace::Folder(f),
            (None, false) => crate::app::RecruitPlace::WorkspaceFolder,
        },
    ) {
        Ok((final_name, lane)) => {
            let place = lane
                .as_ref()
                .map(|l| format!(" in lane {} (base {})", l.branch, l.base_branch))
                .unwrap_or_default();
            Reply::ok(format!("Recruited \"{final_name}\"{place}. It is linked to you; ask it with `sonata ask \"{final_name}\" \"…\"`.")).with_data(json!({"name": final_name, "lane": lane.map(|l| l.branch)}))
        }
        Err(e) => Reply::err(
            if e.code == crate::ErrorCode::NodeNotFound {
                EXIT_NOT_FOUND
            } else {
                EXIT_ERROR
            },
            e.message,
        ),
    }
}

fn dismiss(ctx: &Ctx<'_>, name: &str, progress: &mut dyn FnMut(&str)) -> Reply {
    let node = match resolve(ctx, name, Kind::Terminal) {
        Ok(n) => n,
        Err(r) => return r,
    };
    if node.terminal().and_then(|t| t.recruited_by) != Some(ctx.me) {
        return Reply::err(
            EXIT_DENIED,
            format!(
                "You can only dismiss terminals you recruited; \"{name}\" was not recruited by you."
            ),
        );
    }
    if ctx.app.terminal_has_lane_checkout(node.id) {
        let answer = match ask_human(ctx, node.id, Action::Remove, false, LANE_TIMEOUT, progress) {
            Ok(a) => a,
            Err(r) => return r,
        };
        if !answer.accepted {
            return Reply::err(
                EXIT_DENIED,
                format!(
                    "The user declined removing the lane of \"{name}\". Nothing was dismissed."
                ),
            );
        }
        return match ctx.app.dismiss_with_lane(node.id, removal(answer)) {
            Ok(()) => Reply::ok(format!("Dismissed \"{name}\" and removed its lane.")),
            Err(e) => Reply::err(EXIT_ERROR, e.message),
        };
    }
    match ctx.app.dismiss(node.id) {
        Ok(()) => Reply::ok(format!("Dismissed \"{name}\".")),
        Err(e) => Reply::err(EXIT_ERROR, e.message),
    }
}

// ---- Lanes (feature 008) ----

fn removal(a: Answer) -> crate::app::LaneRemoval {
    crate::app::LaneRemoval {
        discard_changes: a.discard_changes,
        delete_branch: a.delete_branch,
    }
}

/// The terminal a `lane` command acts on: the caller, or a linked terminal by name.
fn lane_target(ctx: &Ctx<'_>, name: Option<&str>) -> Result<Uuid, Reply> {
    match name {
        None => Ok(ctx.me),
        Some(n) => resolve(ctx, n, Kind::Terminal).map(|node| node.id),
    }
}

fn lane_status(ctx: &Ctx<'_>, rest: &[&str]) -> Reply {
    let target = match lane_target(ctx, rest.first().copied()) {
        Ok(t) => t,
        Err(r) => return r,
    };
    match ctx.app.lane_status(target) {
        Ok(s) => {
            let mut text = format!("Lane {} (base {})", s.branch, s.base_branch);
            match (s.ahead, s.behind, s.changed) {
                (Some(a), Some(b), Some(c)) => text.push_str(&format!(
                    ": {a} ahead, {b} behind, {c} changed file{}.",
                    if c == 1 { "" } else { "s" }
                )),
                _ => text.push('.'),
            }
            if s.state != crate::lanes::status::LaneState::Ok {
                text.push_str(&format!(" State: {:?}.", s.state));
            }
            Reply::ok(text).with_data(serde_json::to_value(&s).unwrap_or_default())
        }
        Err(e) => Reply::err(EXIT_NOT_FOUND, e.message),
    }
}

/// Shows the confirmation in the app and waits for the human.
fn ask_human(
    ctx: &Ctx<'_>,
    target: Uuid,
    action: Action,
    delete_branch: bool,
    timeout: Duration,
    progress: &mut dyn FnMut(&str),
) -> Result<Answer, Reply> {
    let verb = match action {
        Action::Land => "land",
        Action::Remove => "lane removal",
    };
    progress(&format!(
        "Waiting for the user to confirm the {verb} in Sonata…"
    ));
    let mut tick = || {
        progress(&format!(
            "Still waiting for the user to confirm the {verb}…"
        ))
    };
    match ctx.app.request_lane_confirmation(
        ctx.me,
        target,
        action,
        delete_branch,
        timeout,
        &mut tick,
    ) {
        Ok(Some(a)) => Ok(a),
        Ok(None) => Err(Reply::err(
            EXIT_DENIED,
            format!(
                "No answer from the user within {}m. Nothing changed.",
                timeout.as_secs() / 60
            ),
        )),
        Err(e) => Err(Reply::err(EXIT_ERROR, e.message)),
    }
}

fn lane_request(
    ctx: &Ctx<'_>,
    action: Action,
    rest: &[&str],
    progress: &mut dyn FnMut(&str),
) -> Reply {
    let mut name = None;
    let mut delete_branch = false;
    let mut timeout = LANE_TIMEOUT;
    let mut it = rest.iter();
    while let Some(a) = it.next() {
        match *a {
            "--delete-branch" if action == Action::Remove => delete_branch = true,
            "--timeout" => match it.next().and_then(|v| parse_duration(v)) {
                Some(d) if LANE_TIMEOUT_RANGE.contains(&d) => timeout = d,
                _ => return Reply::err(EXIT_USAGE, "--timeout takes 1m to 30m."),
            },
            other if other.starts_with('-') => {
                return Reply::err(
                    EXIT_USAGE,
                    format!("Unknown option \"{other}\". Run `sonata help lane`."),
                );
            }
            other if name.is_none() => name = Some(other),
            other => return Reply::err(EXIT_USAGE, format!("Unexpected \"{other}\".")),
        }
    }
    if name.is_some()
        && let Some(denied) = orchestrator(ctx)
    {
        return denied;
    }
    let target = match lane_target(ctx, name) {
        Ok(t) => t,
        Err(r) => return r,
    };
    let lane = match ctx.app.lane_status(target) {
        Ok(s) if s.state != crate::lanes::status::LaneState::Landed => s,
        _ => return Reply::err(EXIT_NOT_FOUND, "This terminal has no active lane."),
    };
    let answer = match ask_human(ctx, target, action, delete_branch, timeout, progress) {
        Ok(a) => a,
        Err(r) => return r,
    };
    let verb = if action == Action::Land {
        "land"
    } else {
        "removal"
    };
    if !answer.accepted {
        return Reply::err(
            EXIT_DENIED,
            format!("The user declined the {verb}. Nothing changed."),
        );
    }
    let own = target == ctx.me;
    match action {
        Action::Land => match ctx.app.lane_land_confirmed(target, own) {
            Ok(n) => Reply::ok(format!(
                "Landed {} into {} ({n} commit{}). The lane was removed; the branch is kept.",
                lane.branch,
                lane.base_branch,
                if n == 1 { "" } else { "s" }
            )),
            Err(e) => Reply::err(EXIT_ERROR, e.message),
        },
        Action::Remove => match ctx.app.lane_remove_confirmed(target, removal(answer), own) {
            Ok(()) => Reply::ok(format!(
                "Removed lane {}. {}",
                lane.branch,
                if answer.delete_branch {
                    "The branch was deleted."
                } else {
                    "The branch is kept."
                }
            )),
            Err(e) => Reply::err(EXIT_ERROR, e.message),
        },
    }
}

fn link(ctx: &Ctx<'_>, a: &str, b: &str, create: bool) -> Reply {
    let find = |name: &str| -> Result<Uuid, Reply> {
        let wanted = name.trim().to_lowercase();
        let nodes: Vec<Node> = ctx
            .app
            .workspaces()
            .workspace(ctx.ws)
            .map(|w| w.nodes.clone())
            .unwrap_or_default();
        let hits: Vec<Uuid> = nodes
            .iter()
            .filter(|n| {
                display_name(ctx, n).to_lowercase() == wanted
                    || item_id(n).is_some_and(|id| id == wanted)
            })
            .map(|n| n.id)
            .collect();
        match hits.as_slice() {
            [id] => Ok(*id),
            [] => Err(Reply::err(
                EXIT_NOT_FOUND,
                format!("Nothing named \"{name}\" in this workspace."),
            )),
            _ => Err(Reply::err(
                EXIT_USAGE,
                format!("\"{name}\" matches several items."),
            )),
        }
    };
    let (x, y) = match (find(a), find(b)) {
        (Ok(x), Ok(y)) => (x, y),
        (Err(r), _) | (_, Err(r)) => return r,
    };
    let result = if create {
        ctx.app
            .create_link(ctx.ws, x, y)
            .map(|_| format!("Linked \"{a}\" and \"{b}\"."))
    } else {
        let link = ctx
            .app
            .links_of(ctx.ws, x)
            .into_iter()
            .find(|l| l.joins(x, y));
        match link {
            Some(l) => ctx
                .app
                .delete_link(ctx.ws, l.id)
                .map(|_| format!("Unlinked \"{a}\" and \"{b}\".")),
            None => {
                return Reply::err(
                    EXIT_NOT_FOUND,
                    format!("\"{a}\" and \"{b}\" are not linked."),
                );
            }
        }
    };
    match result {
        Ok(text) => {
            ctx.app.notify_nodes_changed(ctx.ws);
            Reply::ok(text)
        }
        Err(e) => Reply::err(EXIT_ERROR, e.message),
    }
}

// ---- Agent sessions ----

fn session(ctx: &Ctx<'_>, agent: &str, hook: &str) -> Reply {
    match ctx.app.prepare_agent_session(ctx.me, hook) {
        Ok(files) => Reply::ok("").with_data(json!({
            "agent": agent,
            "instructionsFile": files.instructions,
            "settingsFile": files.settings,
            "codexNotify": super::instructions::codex_notify(hook),
        })),
        Err(e) => Reply::err(EXIT_ERROR, e.message),
    }
}

// ---- Browser nodes (feature 004) ----

fn browser(ctx: &Ctx<'_>, rest: &[&str]) -> Reply {
    use crate::browser::{CLICK_TIMEOUT, LOAD_TIMEOUT, MAX_LOAD_TIMEOUT, PageInfo};
    let usage = "Usage: sonata browser <open|read|click|fill|press|screenshot|back|forward|reload|eval> <browser> …";
    let (sub, name, args) = match rest {
        [sub, name, args @ ..] => (*sub, *name, args),
        _ => return Reply::err(EXIT_USAGE, usage),
    };
    let mut timeout = None;
    let mut out = None;
    let mut positional: Vec<&str> = Vec::new();
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match *a {
            "--timeout" => match it.next().and_then(|v| parse_duration(v)) {
                Some(d) if d >= Duration::from_secs(1) && d <= MAX_LOAD_TIMEOUT => {
                    timeout = Some(d)
                }
                _ => return Reply::err(EXIT_USAGE, "--timeout takes 1s to 5m."),
            },
            "--out" => out = it.next().copied(),
            other => positional.push(other),
        }
    }
    let node = match resolve(ctx, name, Kind::Browser) {
        Ok(n) => n,
        Err(r) => return r,
    };
    if sub == "eval" && orchestrator(ctx).is_some() {
        return Reply::err(
            EXIT_DENIED,
            "Only the Orchestrator can run JavaScript in a browser.",
        );
    }
    let Some(host) = ctx.app.browser_host() else {
        return Reply::err(
            EXIT_ERROR,
            "Browsers are not available right now (the Sonata window is not ready).",
        );
    };
    let load = timeout.unwrap_or(LOAD_TIMEOUT);
    let page = |r: Result<PageInfo, String>| match r {
        Ok(p) => Reply::ok(format!("{}\n{}", p.title, p.url))
            .with_data(json!({"url": p.url, "title": p.title})),
        Err(e) => Reply::err(EXIT_ERROR, e),
    };
    let done = |r: Result<(), String>, msg: &str| match r {
        Ok(()) => Reply::ok(msg),
        Err(e) => Reply::err(EXIT_ERROR, e),
    };
    let marker = ctx.app.mark_browser_control(ctx.me, node.id);
    let reply = match (sub, positional.as_slice()) {
        ("open", [url]) => match crate::browser::normalize_url(url) {
            Ok(u) => page(host.open(node.id, &u, load)),
            Err(e) => Reply::err(EXIT_USAGE, e.message),
        },
        ("read", []) => match host.read(node.id) {
            Ok(text) => Reply::ok(super::cap_text(text, super::MAX_MESSAGE_BYTES)),
            Err(e) => Reply::err(EXIT_ERROR, e),
        },
        ("click", [target]) => page(host.click(node.id, target, timeout.unwrap_or(CLICK_TIMEOUT))),
        ("fill", [target, text @ ..]) if !text.is_empty() => {
            done(host.fill(node.id, target, &text.join(" ")), "Filled.")
        }
        ("press", [key]) => done(host.press(node.id, key), "Pressed."),
        ("back", []) => page(host.back(node.id, load)),
        ("forward", []) => page(host.forward(node.id, load)),
        ("reload", []) => page(host.reload(node.id, load)),
        ("screenshot", []) => {
            let path = match out {
                Some(o) => {
                    let p = PathBuf::from(o);
                    if p.is_absolute() {
                        p
                    } else {
                        PathBuf::from(&ctx.cwd).join(p)
                    }
                }
                None => ctx.app.screenshot_path(),
            };
            match host.screenshot(node.id, &path) {
                Ok(()) => Reply::ok(path.display().to_string()).with_data(json!({"path": path})),
                Err(e) => Reply::err(EXIT_ERROR, e),
            }
        }
        ("eval", js) if !js.is_empty() => match host.eval(node.id, &js.join(" ")) {
            Ok(v) => Reply::ok(super::cap_text(v, super::MAX_RESULT_BYTES)),
            Err(e) => Reply::err(EXIT_ERROR, e),
        },
        _ => Reply::err(EXIT_USAGE, usage),
    };
    ctx.app.end_browser_control(marker);
    reply
}

fn note_new(ctx: &Ctx<'_>, rest: &[&str], stdin: Option<String>) -> Reply {
    let Some((title, body)) = rest.split_first() else {
        return Reply::err(
            EXIT_USAGE,
            "Usage: sonata note new \"<title>\" [\"<text>\"] (or --stdin for the text)",
        );
    };
    let text = stdin.unwrap_or_else(|| body.join(" "));
    match ctx.app.orchestrator_new_note(ctx.me, title, &text) {
        Ok(n) => Reply::ok(format!(
            "Created note \"{title}\" ({}), linked to you.",
            short_id(n.id)
        ))
        .with_data(json!({"id": short_id(n.id)})),
        Err(e) => Reply::err(EXIT_ERROR, e.message),
    }
}

fn browser_new(ctx: &Ctx<'_>, rest: &[&str]) -> Reply {
    match ctx
        .app
        .orchestrator_new_browser(ctx.me, rest.first().copied())
    {
        Ok(n) => {
            let id = crate::browser::short_id(n.id);
            let url = n.browser().map(|b| b.url.clone()).unwrap_or_default();
            Reply::ok(format!(
                "Created browser {id} ({url}), linked to you. Use `sonata browser read {id}`."
            ))
            .with_data(json!({"id": id}))
        }
        Err(e) => Reply::err(
            if e.code == crate::ErrorCode::InvalidAddress {
                EXIT_USAGE
            } else {
                EXIT_ERROR
            },
            e.message,
        ),
    }
}

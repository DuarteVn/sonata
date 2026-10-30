//! Codex provider: plan limits from `codex app-server`, or from the newest session file
//! (research R2). Sonata never reads Codex's login; Codex uses it to answer.

use std::path::{Path, PathBuf};
use std::time::Duration;

use chrono::{DateTime, TimeZone, Utc};
use serde_json::Value;

use super::runner::{RunSpec, run};
use super::window::kind_for_minutes;
use super::{Reading, ReadingStatus, Window, WindowKind};

pub const TIMEOUT: Duration = Duration::from_secs(20);
const STDOUT_CAP: usize = 1024 * 1024;
/// Session files can be huge; only their end is read.
const TAIL_BYTES: u64 = 256 * 1024;
const LOGIN_HINT: &str = "Run `codex login` in a terminal.";

/// The JSON-RPC lines sent to the app server: handshake, then the rate limit read.
pub fn request_lines(version: &str) -> String {
    let init = serde_json::json!({
        "id": 1,
        "method": "initialize",
        "params": {
            "clientInfo": {"name": "sonata", "title": "Sonata", "version": version},
            "capabilities": null
        }
    });
    let read = serde_json::json!({
        "id": 2,
        "method": "account/rateLimits/read",
        "params": {"excludeResetCreditDetails": true}
    });
    format!("{init}\n{{\"method\":\"initialized\"}}\n{read}\n")
}

#[derive(Debug, Clone, PartialEq)]
pub struct Parsed {
    pub status: ReadingStatus,
    pub plan: Option<String>,
    pub windows: Vec<Window>,
    pub message: Option<String>,
}

fn plan_name(v: &Value) -> Option<String> {
    let raw = v.as_str().filter(|s| !s.is_empty() && *s != "unknown")?;
    Some(
        raw.split('_')
            .map(|w| {
                let mut c = w.chars();
                c.next()
                    .map(|f| f.to_uppercase().chain(c).collect::<String>())
                    .unwrap_or_default()
            })
            .collect::<Vec<_>>()
            .join(" "),
    )
}

fn epoch(v: Option<&Value>) -> Option<DateTime<Utc>> {
    v.and_then(Value::as_i64)
        .and_then(|s| Utc.timestamp_opt(s, 0).single())
}

/// One window in either naming: app server (camelCase) or session file (snake_case).
fn window(v: &Value, fallback_kind: WindowKind, base: Option<DateTime<Utc>>) -> Option<Window> {
    let used = v
        .get("usedPercent")
        .or_else(|| v.get("used_percent"))?
        .as_f64()?;
    let minutes = v
        .get("windowDurationMins")
        .or_else(|| v.get("window_minutes"))
        .and_then(Value::as_i64);
    let (kind, label) = match minutes {
        Some(m) => kind_for_minutes(m),
        None if fallback_kind == WindowKind::Short => (WindowKind::Short, "Short window".into()),
        None => (WindowKind::Weekly, "Weekly".into()),
    };
    let resets_at = epoch(v.get("resetsAt").or_else(|| v.get("resets_at"))).or_else(|| {
        // Older session files give seconds left instead of a time.
        let secs = v.get("resets_in_seconds").and_then(Value::as_i64)?;
        Some(base? + chrono::Duration::seconds(secs))
    });
    Some(Window {
        kind,
        label,
        used_percent: used,
        resets_at,
    })
}

fn snapshot(v: &Value, base: Option<DateTime<Utc>>) -> Parsed {
    let windows: Vec<Window> = [
        ("primary", WindowKind::Short),
        ("secondary", WindowKind::Weekly),
    ]
    .iter()
    .filter_map(|(key, kind)| window(v.get(*key)?, *kind, base))
    .collect();
    let plan = v
        .get("planType")
        .or_else(|| v.get("plan_type"))
        .and_then(plan_name);
    if windows.is_empty() {
        return Parsed {
            status: ReadingStatus::NoPlanLimits,
            plan,
            windows,
            message: Some("This account reports no plan limits.".into()),
        };
    }
    Parsed {
        status: ReadingStatus::Ok,
        plan,
        windows,
        message: None,
    }
}

fn response_line(stdout: &str) -> Option<Value> {
    stdout
        .lines()
        .filter_map(|l| serde_json::from_str::<Value>(l).ok())
        .find(|v| v.get("id").and_then(Value::as_i64) == Some(2))
}

/// Parses the app server's answer to `account/rateLimits/read`. `None` when there is no usable
/// answer (for example an older Codex without the method), so the caller falls back.
pub fn parse_response(stdout: &str) -> Option<Parsed> {
    let v = response_line(stdout)?;
    if let Some(err) = v.get("error") {
        let message = err
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_lowercase();
        if ["auth", "login", "logged in", "sign in"]
            .iter()
            .any(|w| message.contains(w))
        {
            return Some(Parsed {
                status: ReadingStatus::NotLoggedIn,
                plan: None,
                windows: Vec::new(),
                message: Some(LOGIN_HINT.into()),
            });
        }
        return None;
    }
    let limits = v.pointer("/result/rateLimits")?;
    Some(snapshot(limits, None))
}

/// The last `token_count` event with rate limits in the tail of a session file.
pub fn parse_rollout_tail(text: &str) -> Option<Parsed> {
    text.lines().rev().find_map(|line| {
        let v: Value = serde_json::from_str(line).ok()?;
        let payload = v.get("payload")?;
        if payload.get("type")?.as_str()? != "token_count" {
            return None;
        }
        let limits = payload.get("rate_limits").filter(|l| l.is_object())?;
        let base = v
            .get("timestamp")
            .and_then(Value::as_str)
            .and_then(|t| DateTime::parse_from_rfc3339(t).ok())
            .map(|t| t.with_timezone(&Utc));
        Some(snapshot(limits, base))
    })
}

fn codex_home(env: &[(String, String)]) -> Option<PathBuf> {
    env.iter()
        .find(|(k, _)| k == "CODEX_HOME")
        .map(|(_, v)| PathBuf::from(v))
        .or_else(|| crate::platform::home_dir().map(|h| h.join(".codex")))
}

/// Session files that may hold a snapshot; only the newest few are tried.
const FILES_TRIED: usize = 5;

/// `rollout-*.jsonl` files under `sessions/` (date folders, at most four levels deep),
/// newest first.
fn session_files(sessions: &Path) -> Vec<(PathBuf, std::time::SystemTime)> {
    fn walk(dir: &Path, depth: u32, out: &mut Vec<(PathBuf, std::time::SystemTime)>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let Ok(meta) = entry.metadata() else { continue };
            if meta.is_dir() && depth < 4 {
                walk(&path, depth + 1, out);
            } else if meta.is_file() {
                let name = entry.file_name();
                let name = name.to_string_lossy();
                if name.starts_with("rollout-")
                    && name.ends_with(".jsonl")
                    && let Ok(modified) = meta.modified()
                {
                    out.push((path, modified));
                }
            }
        }
    }
    let mut out = Vec::new();
    walk(sessions, 0, &mut out);
    out.sort_by_key(|f| std::cmp::Reverse(f.1));
    out
}

fn read_tail(path: &Path) -> std::io::Result<String> {
    use std::io::{Read, Seek, SeekFrom};
    let mut file = std::fs::File::open(path)?;
    let len = file.metadata()?.len();
    file.seek(SeekFrom::Start(len.saturating_sub(TAIL_BYTES)))?;
    let mut buf = Vec::new();
    file.take(TAIL_BYTES).read_to_end(&mut buf)?;
    Ok(String::from_utf8_lossy(&buf).into_owned())
}

fn reading(parsed: Parsed, read_at: DateTime<Utc>) -> Reading {
    let mut r = Reading::new("codex", parsed.status, read_at);
    r.plan = parsed.plan;
    r.windows = parsed.windows;
    r.message = parsed.message;
    r
}

fn from_session_files(env: &[(String, String)], now: DateTime<Utc>) -> Reading {
    let files = codex_home(env)
        .map(|home| session_files(&home.join("sessions")))
        .unwrap_or_default();
    files
        .into_iter()
        .take(FILES_TRIED)
        .find_map(|(path, modified)| {
            let parsed = parse_rollout_tail(&read_tail(&path).ok()?)?;
            Some(reading(parsed, modified.into()))
        })
        .unwrap_or_else(|| Reading::unavailable("codex", now, "Use Codex once to record limits."))
}

fn answered(stdout: &[u8]) -> bool {
    response_line(&String::from_utf8_lossy(stdout)).is_some()
}

/// Reads Codex usage from its app server, or from its newest session file.
pub fn read(path: &Path, env: &[(String, String)], now: DateTime<Utc>) -> Reading {
    let mut spec = RunSpec::new(
        vec![path.display().to_string(), "app-server".into()],
        TIMEOUT,
    );
    spec.env = env.to_vec();
    spec.stdout_cap = STDOUT_CAP;
    spec.stdin = Some(request_lines(env!("CARGO_PKG_VERSION")).into_bytes());
    spec.stop_when = Some(answered);
    match run(&spec) {
        Ok(out) => match parse_response(&String::from_utf8_lossy(&out.stdout)) {
            Some(parsed) => reading(parsed, now),
            None => from_session_files(env, now),
        },
        Err(e) => {
            tracing::info!("codex app-server: {e}");
            from_session_files(env, now)
        }
    }
}

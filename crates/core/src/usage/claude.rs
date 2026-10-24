//! Claude Code provider: login status and plan limits read through the `claude` CLI (research R1).
//! Sonata never reads Claude Code's login; the CLI uses it to answer.

use std::path::Path;
use std::time::Duration;

use chrono::{DateTime, TimeZone, Utc};
use serde_json::Value;

use super::runner::{RunSpec, redact, run};
use super::{Reading, ReadingStatus, Window, WindowKind};

pub const TIMEOUT: Duration = Duration::from_secs(20);
const LOGIN_HINT: &str = "Run `claude auth login` in a terminal.";

/// Probe arguments (research R1): one tiny print-mode request whose `stream-json` output
/// carries a `rate_limit_event`. Shown verbatim in the consent dialog.
pub const PROBE_ARGS: [&str; 18] = [
    "-p",
    ".",
    "--model",
    "haiku",
    "--system-prompt",
    "Reply with one dot.",
    "--tools",
    "",
    "--setting-sources",
    "",
    "--strict-mcp-config",
    "--disable-slash-commands",
    "--no-session-persistence",
    "--max-turns",
    "1",
    "--output-format",
    "stream-json",
    "--verbose",
];

/// What `claude auth status --json` says about the login.
#[derive(Debug, Clone, PartialEq)]
pub struct AuthInfo {
    /// `Ok` (subscription login), `NotLoggedIn`, or `NoPlanLimits`.
    pub status: ReadingStatus,
    pub needs_probe: bool,
    pub plan: Option<String>,
    pub account: Option<String>,
    pub email: Option<String>,
    pub message: Option<String>,
}

fn text(v: &Value, key: &str) -> Option<String> {
    v.get(key)
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

fn title_case(word: &str) -> String {
    let mut chars = word.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

/// `None` when the output is not the expected JSON object.
pub fn parse_auth_status(stdout: &str) -> Option<AuthInfo> {
    let v: Value = serde_json::from_str(stdout.trim()).ok()?;
    let logged_in = v.get("loggedIn")?.as_bool()?;
    let mut info = AuthInfo {
        status: ReadingStatus::Ok,
        needs_probe: false,
        plan: text(&v, "subscriptionType").map(|p| title_case(&p)),
        account: Some(text(&v, "orgName").unwrap_or_else(|| "Personal".into())),
        email: text(&v, "email"),
        message: None,
    };
    if !logged_in {
        info.status = ReadingStatus::NotLoggedIn;
        info.message = Some(LOGIN_HINT.into());
        info.account = None;
        return Some(info);
    }
    let subscription = text(&v, "authMethod").as_deref() == Some("claude.ai")
        && text(&v, "apiProvider")
            .as_deref()
            .is_none_or(|p| p == "firstParty");
    if subscription {
        info.needs_probe = true;
    } else {
        info.status = ReadingStatus::NoPlanLimits;
        info.message = Some("This login pays per use, so it has no plan limits.".into());
    }
    Some(info)
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProbeResult {
    pub status: ReadingStatus,
    pub windows: Vec<Window>,
    pub message: Option<String>,
}

fn epoch(v: &Value) -> Option<DateTime<Utc>> {
    v.as_i64().and_then(|s| Utc.timestamp_opt(s, 0).single())
}

/// `five_hour` → short, `seven_day` → weekly, `seven_day_<model>` → other. Unknown keys: `None`.
fn window_from(key: &str, v: &Value) -> Option<Window> {
    let utilization = v.get("utilization")?.as_f64()?;
    let (kind, label) = match key {
        "five_hour" => (WindowKind::Short, "5-hour".to_string()),
        "seven_day" => (WindowKind::Weekly, "Weekly".to_string()),
        _ => {
            let model = key.strip_prefix("seven_day_")?;
            if model.is_empty() || !model.chars().all(|c| c.is_ascii_alphanumeric()) {
                return None;
            }
            (WindowKind::Other, format!("Weekly ({})", title_case(model)))
        }
    };
    Some(Window {
        kind,
        label,
        used_percent: (utilization * 1000.0).round() / 10.0,
        resets_at: v.get("resetsAt").and_then(epoch),
    })
}

/// Reads the last `rate_limit_event` of a `stream-json` run.
pub fn parse_probe(stdout: &str, exit_ok: bool) -> ProbeResult {
    let mut last_event = None;
    let mut result_line = None;
    for line in stdout.lines() {
        let Ok(v) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        match v.get("type").and_then(Value::as_str) {
            Some("rate_limit_event") => last_event = Some(v),
            Some("result") => result_line = Some(v),
            _ => {}
        }
    }
    if let Some(event) = last_event {
        let mut windows: Vec<Window> = Vec::new();
        if let Some(map) = event
            .pointer("/rate_limit_info/unifiedWindows")
            .and_then(Value::as_object)
        {
            for (key, v) in map {
                windows.extend(window_from(key, v));
            }
        }
        windows.sort_by_key(|w| (w.kind, w.label.clone()));
        if !windows.is_empty() {
            return ProbeResult {
                status: ReadingStatus::Ok,
                windows,
                message: None,
            };
        }
    }
    let failed = !exit_ok
        || result_line
            .as_ref()
            .and_then(|r| r.get("is_error"))
            .and_then(Value::as_bool)
            .unwrap_or(false);
    if failed {
        let detail = result_line
            .as_ref()
            .and_then(|r| r.get("result"))
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_lowercase();
        if ["login", "api key", "auth", "credential"]
            .iter()
            .any(|w| detail.contains(w))
        {
            return ProbeResult {
                status: ReadingStatus::NotLoggedIn,
                windows: Vec::new(),
                message: Some(LOGIN_HINT.into()),
            };
        }
        return ProbeResult {
            status: ReadingStatus::Unavailable,
            windows: Vec::new(),
            message: Some("Claude Code did not report usage.".into()),
        };
    }
    ProbeResult {
        status: ReadingStatus::NoPlanLimits,
        windows: Vec::new(),
        message: Some("This account reports no plan limits.".into()),
    }
}

fn spec(path: &Path, args: &[&str], env: &[(String, String)]) -> RunSpec {
    let argv = std::iter::once(path.display().to_string())
        .chain(args.iter().map(|a| a.to_string()))
        .collect();
    let mut spec = RunSpec::new(argv, TIMEOUT);
    spec.env = env.to_vec();
    spec
}

/// Reads Claude Code usage: login status first, then the probe when the login has plan limits.
pub fn read(path: &Path, env: &[(String, String)], now: DateTime<Utc>) -> Reading {
    let auth = match run(&spec(path, &["auth", "status", "--json"], env)) {
        Ok(out) => parse_auth_status(&String::from_utf8_lossy(&out.stdout)),
        Err(e) => return Reading::unavailable("claude", now, format!("Login check {e}.")),
    };
    let Some(auth) = auth else {
        return Reading::unavailable("claude", now, "Claude Code did not report its login.");
    };
    let mut reading = Reading::new("claude", auth.status, now);
    reading.plan = auth.plan;
    reading.account = auth.account;
    reading.account_email = auth.email;
    reading.message = auth.message;
    if !auth.needs_probe {
        return reading;
    }
    match run(&spec(path, &PROBE_ARGS, env)) {
        Ok(out) => {
            let probe = parse_probe(&String::from_utf8_lossy(&out.stdout), out.code == Some(0));
            if probe.status == ReadingStatus::Unavailable && !out.stderr.is_empty() {
                tracing::info!("claude probe stderr: {}", redact(out.stderr.trim()));
            }
            reading.status = probe.status;
            reading.windows = probe.windows;
            reading.message = probe.message;
        }
        Err(e) => {
            reading.status = ReadingStatus::Unavailable;
            reading.message = Some(format!("Usage check {e}."));
        }
    }
    reading
}

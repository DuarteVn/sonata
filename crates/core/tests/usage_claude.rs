// Unix shells, modes, or sockets; Windows counterparts live in *_windows.rs (feature 012).
#![cfg(unix)]
mod common;

use chrono::{TimeZone, Utc};
use common::fake_bin::{fake_bin, usage_fixture};
use sonata_core::usage::claude::{PROBE_ARGS, parse_auth_status, parse_probe, read};
use sonata_core::usage::{ReadingStatus, WindowKind};

fn fixture(name: &str) -> String {
    std::fs::read_to_string(usage_fixture(name)).unwrap()
}

#[test]
fn team_login_gives_plan_account_and_email() {
    let auth = parse_auth_status(&fixture("claude-auth-team.json")).unwrap();
    assert_eq!(auth.status, ReadingStatus::Ok);
    assert!(auth.needs_probe);
    assert_eq!(auth.plan.as_deref(), Some("Team"));
    assert_eq!(auth.account.as_deref(), Some("Example Org"));
    assert_eq!(auth.email.as_deref(), Some("user@example.com"));
}

#[test]
fn logged_out_says_how_to_log_in() {
    let auth = parse_auth_status(&fixture("claude-auth-logged-out.json")).unwrap();
    assert_eq!(auth.status, ReadingStatus::NotLoggedIn);
    assert!(!auth.needs_probe);
    assert_eq!(
        auth.message.as_deref(),
        Some("Run `claude auth login` in a terminal.")
    );
}

#[test]
fn api_key_accounts_have_no_plan_limits() {
    let auth = parse_auth_status(&fixture("claude-auth-api-key.json")).unwrap();
    assert_eq!(auth.status, ReadingStatus::NoPlanLimits);
    assert!(!auth.needs_probe);
    let bedrock = r#"{"loggedIn":true,"authMethod":"claude.ai","apiProvider":"bedrock"}"#;
    assert_eq!(
        parse_auth_status(bedrock).unwrap().status,
        ReadingStatus::NoPlanLimits
    );
    assert!(parse_auth_status("not json").is_none());
}

#[test]
fn probe_output_gives_both_windows() {
    let probe = parse_probe(&fixture("claude-stream-ok.jsonl"), true);
    assert_eq!(probe.status, ReadingStatus::Ok);
    assert_eq!(probe.windows.len(), 2);
    let short = &probe.windows[0];
    assert_eq!(short.kind, WindowKind::Short);
    assert_eq!(short.label, "5-hour");
    assert_eq!(short.used_percent, 25.0);
    assert_eq!(
        short.resets_at,
        Some(Utc.timestamp_opt(1_792_767_000, 0).unwrap())
    );
    let weekly = &probe.windows[1];
    assert_eq!(weekly.kind, WindowKind::Weekly);
    assert_eq!(weekly.label, "Weekly");
    assert_eq!(weekly.used_percent, 4.0);
}

#[test]
fn last_event_wins_and_model_windows_are_labelled() {
    let probe = parse_probe(&fixture("claude-stream-extra-window.jsonl"), true);
    let found: Vec<(WindowKind, &str, f64)> = probe
        .windows
        .iter()
        .map(|w| (w.kind, w.label.as_str(), w.used_percent))
        .collect();
    assert_eq!(
        found,
        [
            (WindowKind::Short, "5-hour", 50.1),
            (WindowKind::Weekly, "Weekly", 30.0),
            (WindowKind::Other, "Weekly (Sonnet)", 12.0),
        ]
    );
}

#[test]
fn no_event_means_no_plan_limits_and_garbage_is_unavailable() {
    assert_eq!(
        parse_probe(&fixture("claude-stream-no-limits.jsonl"), true).status,
        ReadingStatus::NoPlanLimits
    );
    let bad = parse_probe("panic: something broke", false);
    assert_eq!(bad.status, ReadingStatus::Unavailable);
    assert!(bad.message.is_some());
    let auth_error = r#"{"type":"result","subtype":"success","is_error":true,"result":"Invalid API key · Please run /login"}"#;
    assert_eq!(
        parse_probe(auth_error, false).status,
        ReadingStatus::NotLoggedIn
    );
}

fn fake_claude(dir: &std::path::Path, auth: &str, stream: &str) -> std::path::PathBuf {
    let body = format!(
        r#"if [ "$1" = auth ]; then cat '{auth}'; exit 0; fi
printf '%s\n' "$@" > '{dir}/args'
pwd > '{dir}/cwd'
ls -A | wc -l > '{dir}/count'
cat '{stream}'"#,
        auth = usage_fixture(auth).display(),
        stream = usage_fixture(stream).display(),
        dir = dir.display(),
    );
    fake_bin(dir, "claude", &body)
}

#[test]
fn reader_runs_auth_status_then_the_exact_probe() {
    let dir = tempfile::tempdir().unwrap();
    let bin = fake_claude(
        dir.path(),
        "claude-auth-team.json",
        "claude-stream-ok.jsonl",
    );
    let now = Utc.timestamp_opt(1_792_760_000, 0).unwrap();
    let r = read(&bin, &[], now);
    assert_eq!(r.status, ReadingStatus::Ok, "{:?}", r.message);
    assert_eq!(r.provider_id, "claude");
    assert_eq!(r.plan.as_deref(), Some("Team"));
    assert_eq!(r.account.as_deref(), Some("Example Org"));
    assert_eq!(r.account_email.as_deref(), Some("user@example.com"));
    assert_eq!(r.windows.len(), 2);
    assert_eq!(r.read_at, now);
    let args = std::fs::read_to_string(dir.path().join("args")).unwrap();
    let expected: String = PROBE_ARGS.iter().map(|a| format!("{a}\n")).collect();
    assert_eq!(args, expected);
    let cwd = std::fs::read_to_string(dir.path().join("cwd")).unwrap();
    assert_ne!(cwd.trim(), dir.path().display().to_string());
    let count = std::fs::read_to_string(dir.path().join("count")).unwrap();
    assert_eq!(count.trim(), "0", "probe runs in an empty folder");
}

#[test]
fn reader_skips_the_probe_when_logged_out() {
    let dir = tempfile::tempdir().unwrap();
    let bin = fake_claude(
        dir.path(),
        "claude-auth-logged-out.json",
        "claude-stream-ok.jsonl",
    );
    let r = read(&bin, &[], Utc::now());
    assert_eq!(r.status, ReadingStatus::NotLoggedIn);
    assert!(!dir.path().join("args").exists());
}

/// Runs the real Claude Code once (one tiny request). Opt in with
/// `SONATA_LIVE_CLAUDE=/path/to/claude cargo test -p sonata-core --test usage_claude -- --ignored`.
#[test]
#[ignore]
fn live_claude_reports_plan_limits() {
    let Some(path) = std::env::var_os("SONATA_LIVE_CLAUDE") else {
        return;
    };
    let r = read(std::path::Path::new(&path), &[], Utc::now());
    println!(
        "status={:?} plan={:?} windows={:?} message={:?}",
        r.status, r.plan, r.windows, r.message
    );
    assert_eq!(r.status, ReadingStatus::Ok, "{:?}", r.message);
}

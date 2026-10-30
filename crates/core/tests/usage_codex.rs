// Unix shells, modes, or sockets; Windows counterparts live in *_windows.rs (feature 012).
#![cfg(unix)]
mod common;

use chrono::{TimeZone, Utc};
use common::fake_bin::{fake_bin, usage_fixture};
use sonata_core::usage::codex::{parse_response, parse_rollout_tail, read, request_lines};
use sonata_core::usage::{ReadingStatus, WindowKind};

fn fixture(name: &str) -> String {
    std::fs::read_to_string(usage_fixture(name)).unwrap()
}

#[test]
fn app_server_response_gives_windows_and_plan() {
    let parsed = parse_response(&fixture("codex-ratelimits-response.json")).unwrap();
    assert_eq!(parsed.status, ReadingStatus::Ok);
    assert_eq!(parsed.plan.as_deref(), Some("Plus"));
    let w: Vec<(WindowKind, &str, f64)> = parsed
        .windows
        .iter()
        .map(|w| (w.kind, w.label.as_str(), w.used_percent))
        .collect();
    assert_eq!(
        w,
        [
            (WindowKind::Short, "5-hour", 42.0),
            (WindowKind::Weekly, "Weekly", 17.5)
        ]
    );
    assert_eq!(
        parsed.windows[0].resets_at,
        Some(Utc.timestamp_opt(1_792_780_000, 0).unwrap())
    );
}

#[test]
fn auth_error_is_not_logged_in() {
    let parsed = parse_response(&fixture("codex-ratelimits-auth-error.json")).unwrap();
    assert_eq!(parsed.status, ReadingStatus::NotLoggedIn);
    assert!(
        parse_response(r#"{"id":2,"error":{"code":-32601,"message":"method not found"}}"#)
            .is_none()
    );
}

#[test]
fn other_window_lengths_are_labelled_from_minutes() {
    let body = r#"{"id":2,"result":{"rateLimits":{"primary":{"usedPercent":1,"windowDurationMins":120,"resetsAt":null},"secondary":{"usedPercent":2,"windowDurationMins":43200,"resetsAt":null},"planType":null}}}"#;
    let parsed = parse_response(body).unwrap();
    let labels: Vec<(WindowKind, &str)> = parsed
        .windows
        .iter()
        .map(|w| (w.kind, w.label.as_str()))
        .collect();
    assert_eq!(
        labels,
        [(WindowKind::Short, "2-hour"), (WindowKind::Other, "30-day")]
    );
}

#[test]
fn rollout_tail_uses_the_last_snapshot() {
    let parsed = parse_rollout_tail(&fixture("codex-rollout.jsonl")).unwrap();
    assert_eq!(parsed.plan.as_deref(), Some("Pro"));
    assert_eq!(parsed.windows[0].used_percent, 33.0);
    assert_eq!(parsed.windows[1].used_percent, 8.0);
    assert!(parse_rollout_tail("{}\nnot json\n").is_none());
}

fn fake_codex(dir: &std::path::Path, response: &str) -> std::path::PathBuf {
    let body = format!(
        r#"[ "$1" = app-server ] || exit 2
while IFS= read -r line; do
  printf '%s\n' "$line" >> '{dir}/received'
  case "$line" in
    *'"initialize"'*) echo '{{"id":1,"result":{{"userAgent":"codex/1.0"}}}}' ;;
    *'rateLimits/read'*) cat '{response}' ;;
  esac
done"#,
        dir = dir.display(),
        response = usage_fixture(response).display()
    );
    fake_bin(dir, "codex", &body)
}

#[test]
fn reader_talks_to_the_app_server() {
    let dir = tempfile::tempdir().unwrap();
    let bin = fake_codex(dir.path(), "codex-ratelimits-response.json");
    let now = Utc.timestamp_opt(1_792_770_000, 0).unwrap();
    let r = read(&bin, &[], now);
    assert_eq!(r.status, ReadingStatus::Ok, "{:?}", r.message);
    assert_eq!(r.provider_id, "codex");
    assert_eq!(r.read_at, now);
    assert_eq!(r.windows.len(), 2);
    let received = std::fs::read_to_string(dir.path().join("received")).unwrap();
    let expected: String = request_lines("0.2.0")
        .lines()
        .map(|l| format!("{l}\n"))
        .collect();
    assert_eq!(received, expected);
}

#[test]
fn reader_falls_back_to_the_newest_session_file() {
    let dir = tempfile::tempdir().unwrap();
    let bin = fake_bin(dir.path(), "codex", "exit 1");
    let home = dir.path().join("codex-home");
    let day = home.join("sessions/2026/10/23");
    std::fs::create_dir_all(&day).unwrap();
    std::fs::create_dir_all(home.join("sessions/2026/10/22")).unwrap();
    std::fs::write(home.join("sessions/2026/10/22/rollout-old.jsonl"), "{}\n").unwrap();
    std::fs::copy(
        usage_fixture("codex-rollout.jsonl"),
        day.join("rollout-new.jsonl"),
    )
    .unwrap();
    let env = vec![("CODEX_HOME".to_string(), home.display().to_string())];
    let r = read(&bin, &env, Utc::now());
    assert_eq!(r.status, ReadingStatus::Ok, "{:?}", r.message);
    assert_eq!(r.windows[0].used_percent, 33.0);
    let mtime: chrono::DateTime<Utc> = std::fs::metadata(day.join("rollout-new.jsonl"))
        .unwrap()
        .modified()
        .unwrap()
        .into();
    assert_eq!(r.read_at.timestamp(), mtime.timestamp());
}

#[test]
fn no_session_file_says_how_to_record_limits() {
    let dir = tempfile::tempdir().unwrap();
    let bin = fake_bin(dir.path(), "codex", "exit 1");
    let env = vec![(
        "CODEX_HOME".to_string(),
        dir.path().join("none").display().to_string(),
    )];
    let r = read(&bin, &env, Utc::now());
    assert_eq!(r.status, ReadingStatus::Unavailable);
    assert_eq!(
        r.message.as_deref(),
        Some("Use Codex once to record limits.")
    );
}

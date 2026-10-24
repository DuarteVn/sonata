mod common;

use chrono::{TimeZone, Utc};
use common::fake_bin::fake_bin;
use sonata_core::usage::custom::{parse_output, run_custom};
use sonata_core::usage::{ReadingStatus, WindowKind};

#[test]
fn normalized_output_with_both_time_forms() {
    let out = r#"{"status":"ok","plan":"Pro","account":"work","windows":[
        {"kind":"short","label":"Session","usedPercent":42.5,"resetsAt":"2026-10-23T18:00:00Z"},
        {"kind":"weekly","usedPercent":12,"resetsAt":1793343600},
        {"kind":"other","usedPercent":3}]}"#;
    let r = parse_output("mine", out, Utc::now());
    assert_eq!(r.status, ReadingStatus::Ok, "{:?}", r.message);
    assert_eq!(r.provider_id, "mine");
    assert_eq!(r.plan.as_deref(), Some("Pro"));
    assert_eq!(r.account.as_deref(), Some("work"));
    let w = &r.windows;
    assert_eq!(
        (w[0].kind, w[0].label.as_str(), w[0].used_percent),
        (WindowKind::Short, "Session", 42.5)
    );
    assert_eq!(
        w[0].resets_at,
        Some(Utc.with_ymd_and_hms(2026, 10, 23, 18, 0, 0).unwrap())
    );
    assert_eq!(w[1].label, "Weekly");
    assert_eq!(
        w[1].resets_at,
        Some(Utc.timestamp_opt(1_793_343_600, 0).unwrap())
    );
    assert_eq!(w[2].label, "Limit");
    assert_eq!(w[2].resets_at, None);
    let short = parse_output(
        "mine",
        r#"{"windows":[{"kind":"short","usedPercent":1}]}"#,
        Utc::now(),
    );
    assert_eq!(short.windows[0].label, "Short window");
}

#[test]
fn other_statuses_carry_a_message() {
    let r = parse_output(
        "m",
        r#"{"status":"notLoggedIn","message":"Run my-agent login."}"#,
        Utc::now(),
    );
    assert_eq!(r.status, ReadingStatus::NotLoggedIn);
    assert_eq!(r.message.as_deref(), Some("Run my-agent login."));
    assert_eq!(
        parse_output("m", r#"{"status":"noPlanLimits"}"#, Utc::now()).status,
        ReadingStatus::NoPlanLimits
    );
    assert_eq!(
        parse_output(
            "m",
            r#"{"status":"unavailable","message":"offline"}"#,
            Utc::now()
        )
        .status,
        ReadingStatus::Unavailable
    );
}

#[test]
fn invalid_output_is_unavailable_with_the_reason() {
    let cases = [
        ("not json", "invalid usage JSON"),
        (r#"{"windows":[]}"#, "1 to 4 windows"),
        (
            r#"{"windows":[{"kind":"short","usedPercent":1},{"kind":"short","usedPercent":1},{"kind":"short","usedPercent":1},{"kind":"short","usedPercent":1},{"kind":"short","usedPercent":1}]}"#,
            "1 to 4 windows",
        ),
        (
            r#"{"plan":"0123456789012345678901234567890123456789x","windows":[{"kind":"short","usedPercent":1}]}"#,
            "plan",
        ),
        (
            r#"{"windows":[{"kind":"short","label":"0123456789012345678901234","usedPercent":1}]}"#,
            "label",
        ),
        (
            r#"{"windows":[{"kind":"short","usedPercent":-3}]}"#,
            "usedPercent",
        ),
        (
            r#"{"windows":[{"kind":"daily","usedPercent":1}]}"#,
            "invalid usage JSON",
        ),
        (
            r#"{"status":"unavailable","message":"__LONG__"}"#,
            "message",
        ),
    ];
    for (out, reason) in cases {
        let out = out.replace("__LONG__", &"m".repeat(201));
        let r = parse_output("m", &out, Utc::now());
        assert_eq!(r.status, ReadingStatus::Unavailable, "{out}");
        let msg = r.message.unwrap();
        assert!(msg.contains(reason), "{out} → {msg}");
    }
}

#[test]
fn exit_status_and_timeout_are_reported() {
    let dir = tempfile::tempdir().unwrap();
    let fail = fake_bin(dir.path(), "fail", "exit 2");
    let r = run_custom(
        "m",
        &[fail.display().to_string()],
        &[],
        None,
        std::time::Duration::from_secs(5),
        Utc::now(),
    );
    assert_eq!(r.message.as_deref(), Some("exited with status 2"));
    let slow = fake_bin(dir.path(), "slow", "sleep 30");
    let r = run_custom(
        "m",
        &[slow.display().to_string()],
        &[],
        None,
        std::time::Duration::from_millis(300),
        Utc::now(),
    );
    assert_eq!(r.status, ReadingStatus::Unavailable);
    assert_eq!(r.message.as_deref(), Some("timed out after 0.3 s"));
}

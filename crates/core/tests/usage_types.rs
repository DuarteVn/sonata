use chrono::{TimeZone, Utc};
use sonata_core::usage::{Reading, ReadingStatus, Window, WindowKind, current_windows};

fn window(used: f64, resets: Option<i64>) -> Window {
    Window {
        kind: WindowKind::Short,
        label: "5-hour".into(),
        used_percent: used,
        resets_at: resets.map(|s| Utc.timestamp_opt(s, 0).unwrap()),
    }
}

#[test]
fn reading_serializes_in_camel_case_with_rfc3339_times() {
    let mut r = Reading::new(
        "claude",
        ReadingStatus::Ok,
        Utc.timestamp_opt(1_800_000_000, 0).unwrap(),
    );
    r.plan = Some("Team".into());
    r.account_email = Some("user@example.com".into());
    r.windows = vec![window(25.0, Some(1_800_003_600))];
    let v = serde_json::to_value(&r).unwrap();
    assert_eq!(v["providerId"], "claude");
    assert_eq!(v["status"], "ok");
    assert_eq!(v["accountEmail"], "user@example.com");
    assert_eq!(v["readAt"], "2027-01-15T08:00:00Z");
    assert_eq!(v["windows"][0]["usedPercent"], 25.0);
    assert_eq!(v["windows"][0]["kind"], "short");
    assert_eq!(v["windows"][0]["resetsAt"], "2027-01-15T09:00:00Z");
    let back: Reading = serde_json::from_value(v).unwrap();
    assert_eq!(back, r);
}

#[test]
fn empty_optional_fields_are_omitted() {
    let r = Reading::new("codex", ReadingStatus::NotLoggedIn, Utc::now());
    let v = serde_json::to_value(&r).unwrap();
    assert!(v.get("plan").is_none());
    assert!(v.get("accountEmail").is_none());
    assert!(v.get("message").is_none());
}

#[test]
fn invalid_and_past_windows_are_dropped() {
    let now = Utc.timestamp_opt(1_800_000_000, 0).unwrap();
    let kept = current_windows(
        vec![
            window(10.0, Some(1_800_000_060)),
            window(f64::NAN, Some(1_800_000_060)),
            window(-1.0, None),
            window(50.0, Some(1_799_999_000)),
            window(120.0, None),
        ],
        now,
    );
    let used: Vec<f64> = kept.iter().map(|w| w.used_percent).collect();
    assert_eq!(used, vec![10.0, 120.0]);
}

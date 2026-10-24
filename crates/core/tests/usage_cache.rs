mod common;

use chrono::{TimeZone, Utc};
use common::Sandbox;
use sonata_core::usage::cache::{self, UsageCache};
use sonata_core::usage::window::{WarningEntry, WarningLevel};
use sonata_core::usage::{Reading, ReadingStatus, Window, WindowKind};

fn reading(id: &str, status: ReadingStatus) -> Reading {
    let mut r = Reading::new(id, status, Utc.timestamp_opt(1_800_000_000, 0).unwrap());
    r.plan = Some("Team".into());
    r.account = Some("Example Org".into());
    r.account_email = Some("user@example.com".into());
    if status == ReadingStatus::Ok {
        r.windows = vec![Window {
            kind: WindowKind::Short,
            label: "5-hour".into(),
            used_percent: 42.0,
            resets_at: Some(Utc.timestamp_opt(1_800_003_600, 0).unwrap()),
        }];
    }
    r
}

#[test]
fn round_trips_in_an_envelope_without_email() {
    let sb = Sandbox::new();
    let mut c = UsageCache::default();
    c.readings
        .insert("claude".into(), reading("claude", ReadingStatus::Ok));
    c.warnings.push(WarningEntry {
        provider_id: "claude".into(),
        window_kind: WindowKind::Short,
        resets_at: Some(Utc.timestamp_opt(1_800_003_600, 0).unwrap()),
        sent: WarningLevel::Warn80,
    });
    cache::save(&sb.root, &c).unwrap();
    let text = std::fs::read_to_string(sb.root.usage_cache_file()).unwrap();
    let v: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(v["type"], "usageCache");
    assert_eq!(v["schemaVersion"], 1);
    assert!(!text.contains('@'), "{text}");
    assert!(!text.contains("accountEmail"), "{text}");
    let loaded = cache::load(&sb.root);
    assert!(loaded.warning.is_none());
    let mut expected = reading("claude", ReadingStatus::Ok);
    expected.account_email = None;
    assert_eq!(loaded.value.readings["claude"], expected);
    assert_eq!(loaded.value.warnings, c.warnings);
}

#[test]
fn only_lasting_statuses_are_cached() {
    let sb = Sandbox::new();
    let mut c = UsageCache::default();
    for (id, status) in [
        ("a", ReadingStatus::Ok),
        ("b", ReadingStatus::NoPlanLimits),
        ("c", ReadingStatus::NotLoggedIn),
        ("d", ReadingStatus::Unavailable),
        ("e", ReadingStatus::NotInstalled),
        ("f", ReadingStatus::Off),
    ] {
        c.readings.insert(id.into(), reading(id, status));
    }
    cache::save(&sb.root, &c).unwrap();
    let loaded = cache::load(&sb.root).value;
    let ids: Vec<&String> = loaded.readings.keys().collect();
    assert_eq!(ids, ["a", "b", "c"]);
}

#[test]
fn missing_file_is_an_empty_cache() {
    let sb = Sandbox::new();
    let loaded = cache::load(&sb.root);
    assert!(loaded.warning.is_none());
    assert_eq!(loaded.value, UsageCache::default());
}

#[test]
fn corrupt_file_is_reported_and_kept_aside() {
    let sb = Sandbox::new();
    std::fs::write(sb.root.usage_cache_file(), "{not json").unwrap();
    let loaded = cache::load(&sb.root);
    assert!(loaded.warning.is_some());
    assert_eq!(loaded.value, UsageCache::default());
    let kept: Vec<_> = std::fs::read_dir(sb.root.path())
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.starts_with("usage-cache.json.invalid-"))
        .collect();
    assert_eq!(kept.len(), 1, "bad file kept aside, not overwritten");
}

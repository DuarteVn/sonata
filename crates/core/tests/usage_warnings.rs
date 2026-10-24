mod common;

use chrono::{DateTime, TimeZone, Utc};
use common::Sandbox;
use sonata_core::usage::cache::{self, UsageCache};
use sonata_core::usage::window::{WarningEntry, WarningLevel, observe};
use sonata_core::usage::{Window, WindowKind};

fn t(s: i64) -> DateTime<Utc> {
    Utc.timestamp_opt(1_800_000_000 + s, 0).unwrap()
}

fn w(percent: f64, reset: i64) -> Window {
    Window {
        kind: WindowKind::Short,
        label: "5-hour".into(),
        used_percent: percent,
        resets_at: Some(t(reset)),
    }
}

fn levels(entries: &mut Vec<WarningEntry>, percent: f64, reset: i64) -> Vec<u8> {
    observe(entries, "claude", &[w(percent, reset)], t(0))
        .into_iter()
        .map(|(_, l)| l.percent())
        .collect()
}

#[test]
fn each_level_fires_once_per_period() {
    let mut e = Vec::new();
    assert!(levels(&mut e, 79.0, 3600).is_empty());
    assert_eq!(levels(&mut e, 85.0, 3600), [80]);
    assert!(levels(&mut e, 90.0, 3600).is_empty());
    assert_eq!(levels(&mut e, 100.0, 3600), [100]);
    assert!(levels(&mut e, 95.0, 3600).is_empty(), "no re-arm on a drop");
    assert!(levels(&mut e, 100.0, 3600).is_empty());
}

#[test]
fn jumping_straight_to_100_sends_only_the_full_warning() {
    let mut e = Vec::new();
    assert_eq!(levels(&mut e, 100.0, 3600), [100]);
    assert!(levels(&mut e, 85.0, 3600).is_empty());
}

#[test]
fn a_new_period_starts_again() {
    let mut e = Vec::new();
    assert_eq!(levels(&mut e, 85.0, 3600), [80]);
    assert_eq!(
        levels(&mut e, 85.0, 7200),
        [80],
        "new resetsAt = new period"
    );
}

#[test]
fn windows_are_tracked_separately() {
    let mut e = Vec::new();
    let weekly = Window {
        kind: WindowKind::Weekly,
        label: "Weekly".into(),
        used_percent: 81.0,
        resets_at: Some(t(86_400)),
    };
    let fired = observe(&mut e, "claude", &[w(85.0, 3600), weekly], t(0));
    assert_eq!(fired.len(), 2);
    assert_eq!(fired[1].0.label, "Weekly");
}

#[test]
fn past_periods_are_pruned() {
    let mut e = Vec::new();
    levels(&mut e, 85.0, 3600);
    assert_eq!(e.len(), 1);
    observe(&mut e, "codex", &[], t(4000));
    assert!(e.is_empty());
}

#[test]
fn sent_levels_survive_a_restart() {
    let sb = Sandbox::new();
    let mut c = UsageCache::default();
    levels(&mut c.warnings, 85.0, 3600);
    cache::save(&sb.root, &c).unwrap();
    let mut again = cache::load(&sb.root).value;
    assert_eq!(again.warnings[0].sent, WarningLevel::Warn80);
    assert!(levels(&mut again.warnings, 85.0, 3600).is_empty());
}

#[test]
fn reset_times_that_drift_a_little_are_the_same_period() {
    let mut e = Vec::new();
    assert_eq!(levels(&mut e, 85.0, 3600), [80]);
    assert!(levels(&mut e, 86.0, 3630).is_empty());
    assert_eq!(levels(&mut e, 85.0, 3600 + 3600), [80]);
}

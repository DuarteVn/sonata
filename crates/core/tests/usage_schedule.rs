use std::time::{Duration, Instant};

use sonata_core::usage::schedule::{RefreshReason, Schedule};

const MIN: Duration = Duration::from_secs(60);

#[test]
fn automatic_reads_follow_the_provider_interval() {
    let t0 = Instant::now();
    let mut s = Schedule::default();
    assert!(s.due("claude", RefreshReason::Auto, t0, None, false));
    s.mark("claude", t0);
    s.mark("codex", t0);
    assert!(!s.due("claude", RefreshReason::Auto, t0 + 4 * MIN, None, false));
    assert!(s.due("claude", RefreshReason::Auto, t0 + 5 * MIN, None, false));
    assert!(!s.due("codex", RefreshReason::Auto, t0 + MIN / 2, None, false));
    assert!(s.due("codex", RefreshReason::Auto, t0 + MIN, None, false));
}

#[test]
fn panel_open_reads_only_when_the_cache_is_older_than_a_minute() {
    let t0 = Instant::now();
    let s = Schedule::default();
    assert!(s.due("claude", RefreshReason::PanelOpen, t0, None, false));
    assert!(!s.due("claude", RefreshReason::PanelOpen, t0, Some(MIN / 2), false));
    assert!(s.due("claude", RefreshReason::PanelOpen, t0, Some(2 * MIN), false));
}

#[test]
fn nothing_but_manual_runs_within_a_minute_of_the_last_attempt() {
    let t0 = Instant::now();
    let mut s = Schedule::default();
    s.mark("claude", t0);
    let soon = t0 + MIN / 2;
    assert!(!s.due(
        "claude",
        RefreshReason::PanelOpen,
        soon,
        Some(10 * MIN),
        false
    ));
    assert!(!s.due("claude", RefreshReason::Auto, soon, None, true));
    assert!(s.due(
        "claude",
        RefreshReason::Manual,
        soon,
        Some(Duration::ZERO),
        false
    ));
}

#[test]
fn a_passed_reset_makes_the_provider_due() {
    let t0 = Instant::now();
    let mut s = Schedule::default();
    s.mark("claude", t0);
    assert!(!s.due("claude", RefreshReason::Auto, t0 + 2 * MIN, None, false));
    assert!(s.due("claude", RefreshReason::Auto, t0 + 2 * MIN, None, true));
}

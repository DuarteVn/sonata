use std::time::{Duration, Instant};

use sonata_core::update::{CHECK_INTERVAL, UpdateSchedule};

#[test]
fn a_check_is_due_at_launch() {
    assert!(UpdateSchedule::new(true).due(Instant::now()));
}

#[test]
fn the_next_check_waits_a_day() {
    let start = Instant::now();
    let mut schedule = UpdateSchedule::new(true);
    schedule.mark_checked(start);
    assert_eq!(CHECK_INTERVAL, Duration::from_secs(24 * 60 * 60));
    assert!(!schedule.due(start + Duration::from_secs(60)));
    assert!(!schedule.due(start + CHECK_INTERVAL - Duration::from_secs(1)));
    assert!(schedule.due(start + CHECK_INTERVAL));
}

#[test]
fn nothing_is_due_while_turned_off() {
    let mut schedule = UpdateSchedule::new(false);
    assert!(!schedule.due(Instant::now()));
    schedule.set_enabled(true);
    assert!(schedule.due(Instant::now()));
    schedule.set_enabled(false);
    assert!(!schedule.due(Instant::now() + CHECK_INTERVAL * 3));
}

#[test]
fn a_manual_check_does_not_move_the_schedule() {
    // Manual checks never call mark_checked; only automatic ones do.
    let start = Instant::now();
    let mut schedule = UpdateSchedule::new(true);
    schedule.mark_checked(start);
    assert_eq!(schedule.last_check(), Some(start));
}

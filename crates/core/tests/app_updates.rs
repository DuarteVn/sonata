mod common;

use std::time::Instant;

use common::{Sandbox, test_app};
use sonata_core::events::CoreEvent;
use sonata_core::persistence::store::Store;
use sonata_core::update::{Release, UpdateState};

fn release() -> Release {
    Release {
        version: "0.2.1".into(),
        notes: "notes".into(),
        date: None,
    }
}

#[test]
fn update_state_changes_are_emitted() {
    let sb = Sandbox::new();
    let (app, events) = test_app(&sb);
    events.take();
    app.update(|s| s.found(release()));
    assert!(matches!(app.update_state(), UpdateState::Available { .. }));
    let emitted: Vec<_> = events
        .take()
        .into_iter()
        .filter(|e| matches!(e, CoreEvent::UpdateState(_)))
        .collect();
    assert_eq!(emitted.len(), 1);
    assert_eq!(emitted[0].name(), "update://state");
}

#[test]
fn turning_auto_check_off_saves_and_stops_the_schedule() {
    let sb = Sandbox::new();
    let (app, _) = test_app(&sb);
    assert!(app.update_check_due(Instant::now()));
    app.set_auto_update_check(false).unwrap();
    assert!(!app.update_check_due(Instant::now()));
    assert!(!app.auto_update_check());
    let saved = Store::new(sb.root.clone()).load_preferences().value;
    assert!(!saved.auto_update_check);
}

#[test]
fn an_automatic_check_is_not_due_again_the_same_day() {
    let sb = Sandbox::new();
    let (app, _) = test_app(&sb);
    let now = Instant::now();
    app.mark_update_checked(now);
    assert!(!app.update_check_due(now));
}

use sonata_core::update::{Release, UpdateState};

fn release(version: &str) -> Release {
    Release {
        version: version.into(),
        notes: format!("notes {version}"),
        date: Some("2026-10-30T12:00:00Z".into()),
    }
}

fn available(version: &str) -> UpdateState {
    let mut state = UpdateState::Idle;
    state.found(release(version));
    state
}

#[test]
fn a_found_release_becomes_available() {
    assert_eq!(
        available("0.2.0"),
        UpdateState::Available {
            version: "0.2.0".into(),
            notes: "notes 0.2.0".into(),
            date: Some("2026-10-30T12:00:00Z".into()),
        }
    );
}

#[test]
fn later_hides_until_the_next_check_finds_it_again() {
    let mut state = available("0.2.0");
    state.dismiss();
    assert_eq!(state, UpdateState::Dismissed { version: "0.2.0".into() });
    assert!(!state.pending());
    state.found(release("0.2.0"));
    assert!(matches!(state, UpdateState::Available { .. }));
}

#[test]
fn a_newer_release_replaces_an_older_one() {
    let mut state = available("0.2.0");
    state.found(release("0.2.1"));
    assert_eq!(state.version(), Some("0.2.1"));
}

#[test]
fn download_then_ready_to_install() {
    let mut state = available("0.2.0");
    assert!(state.start_download());
    state.progress(10, Some(100));
    assert_eq!(
        state,
        UpdateState::Downloading { version: "0.2.0".into(), received: 10, total: Some(100) }
    );
    state.downloaded();
    assert_eq!(state, UpdateState::ReadyToInstall { version: "0.2.0".into() });
    state.installing();
    assert_eq!(state, UpdateState::Installing { version: "0.2.0".into() });
}

#[test]
fn a_failed_download_keeps_the_version_and_can_retry() {
    let mut state = available("0.2.0");
    state.start_download();
    state.fail("bad signature".into());
    assert_eq!(
        state,
        UpdateState::Failed { version: "0.2.0".into(), reason: "bad signature".into() }
    );
    assert!(state.start_download());
}

#[test]
fn checks_do_not_interrupt_a_download_or_a_pending_install() {
    let mut state = available("0.2.0");
    state.start_download();
    state.found(release("0.2.1"));
    assert!(matches!(state, UpdateState::Downloading { .. }));
    state.downloaded();
    state.found(release("0.2.1"));
    assert!(matches!(state, UpdateState::ReadyToInstall { .. }));
}

#[test]
fn download_only_starts_from_available_or_failed() {
    let mut idle = UpdateState::Idle;
    assert!(!idle.start_download());
    assert_eq!(idle, UpdateState::Idle);
    let mut dismissed = available("0.2.0");
    dismissed.dismiss();
    assert!(!dismissed.start_download());
}

#[test]
fn a_wrong_location_blocks_the_update() {
    let mut state = available("0.2.0");
    state.block("Move Sonata to Applications to update".into());
    assert_eq!(
        state,
        UpdateState::Blocked {
            version: "0.2.0".into(),
            reason: "Move Sonata to Applications to update".into()
        }
    );
}

#[test]
fn pending_states() {
    let mut state = UpdateState::Idle;
    assert!(!state.pending());
    state.found(release("0.2.0"));
    assert!(state.pending());
    state.start_download();
    assert!(state.pending());
    state.fail("x".into());
    assert!(state.pending());
    state.block("y".into());
    assert!(state.pending());
}

#[test]
fn serializes_like_the_ipc_contract() {
    let mut state = available("0.2.0");
    state.start_download();
    let json = serde_json::to_value(&state).unwrap();
    assert_eq!(
        json,
        serde_json::json!({"kind": "downloading", "version": "0.2.0", "received": 0, "total": null})
    );
    let json = serde_json::to_value(UpdateState::ReadyToInstall { version: "1.0.0".into() }).unwrap();
    assert_eq!(json, serde_json::json!({"kind": "readyToInstall", "version": "1.0.0"}));
}

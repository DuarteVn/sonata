//! Device runtimes: boot, states, attach, delete, quit (FR-007, FR-008, edge cases).

mod common;

use std::sync::Arc;
use std::sync::atomic::Ordering;

use common::fake_device::{FakeLauncher, FakeWorld};
use common::{Sandbox, WAIT, test_app, wait_until};
use sonata_core::device::DeviceState;
use sonata_core::events::CoreEvent;
use sonata_core::model::node::{Frame, Platform};
use sonata_core::workspace::manager::CreateWorkspace;

const PHONE: Frame = Frame {
    x: 0.0,
    y: 0.0,
    width: 380.0,
    height: 820.0,
};

fn setup() -> (
    Sandbox,
    Arc<sonata_core::App>,
    Arc<sonata_core::events::RecordingSink>,
    Arc<FakeWorld>,
    uuid::Uuid,
) {
    let sb = Sandbox::new();
    let (app, sink) = test_app(&sb);
    let world = Arc::new(FakeWorld::default());
    world.boot_polls.store(2, Ordering::SeqCst);
    app.set_device_launcher(Arc::new(FakeLauncher(world.clone())));
    let ws = app
        .create_workspace(CreateWorkspace {
            name: "W".into(),
            working_directory: sb.project("p"),
        })
        .unwrap()
        .id;
    app.open_workspace(ws).unwrap();
    (sb, app, sink, world, ws)
}

fn device(app: &sonata_core::App, ws: uuid::Uuid, id: &str) -> uuid::Uuid {
    let n = app.create_device(ws, PHONE).unwrap().id;
    app.select_device(n, Platform::Android, id, Some(id), Some("Android 14"))
        .unwrap();
    n
}

#[test]
fn boot_goes_through_booting_to_ready_and_emits_state() {
    let (_sb, app, sink, world, ws) = setup();
    let d = device(&app, ws, "Pixel");
    assert_eq!(
        app.device_state(d).state,
        DeviceState::Off,
        "never boots on its own"
    );
    app.device_boot(d).unwrap();
    assert_eq!(app.device_state(d).state, DeviceState::Booting);
    assert!(wait_until(WAIT, || app.device_state(d).state == DeviceState::Ready));
    assert_eq!(world.calls(), vec!["start Pixel"]);
    let states: Vec<String> = sink
        .take()
        .into_iter()
        .filter_map(|e| match e {
            CoreEvent::DeviceState(s) if s.node_id == d => Some(format!("{:?}", s.state)),
            _ => None,
        })
        .collect();
    assert_eq!(states, vec!["Booting", "Ready"]);
    // Booting again while running does nothing.
    app.device_boot(d).unwrap();
    assert_eq!(world.calls(), vec!["start Pixel"]);
    app.quit();
}

#[test]
fn a_crash_shows_stopped_and_commands_fail() {
    let (_sb, app, _, world, ws) = setup();
    let d = device(&app, ws, "Pixel");
    app.device_boot(d).unwrap();
    assert!(wait_until(WAIT, || app.device_state(d).state == DeviceState::Ready));
    assert!(app.device_driver(d).is_ok());
    world.crash.store(true, Ordering::SeqCst);
    assert!(wait_until(WAIT, || app.device_state(d).state == DeviceState::Stopped));
    let err = app.device_driver(d).err().unwrap();
    assert_eq!(err.code, sonata_core::ErrorCode::DeviceNotRunning);
    assert_eq!(err.message, "device not running");
    app.quit();
}

#[test]
fn start_failure_is_an_error_state_with_the_message() {
    let (_sb, app, _, world, ws) = setup();
    *world.fail_start.lock().unwrap() = Some("HVF is not available".into());
    let d = device(&app, ws, "Pixel");
    assert!(app.device_boot(d).is_err());
    assert_eq!(
        app.device_state(d).state,
        DeviceState::Error("HVF is not available".into())
    );
    app.quit();
}

#[test]
fn delete_and_quit_stop_only_devices_sonata_started() {
    let (_sb, app, _, world, ws) = setup();
    world.outside.lock().unwrap().push("Outside".into());
    let mine = device(&app, ws, "Mine");
    let other = device(&app, ws, "Outside");
    let kept = device(&app, ws, "Kept");
    for d in [mine, other, kept] {
        app.device_boot(d).unwrap();
    }
    assert!(wait_until(WAIT, || [mine, other, kept]
        .iter()
        .all(|d| app.device_state(*d).state == DeviceState::Ready)));
    let running: Vec<String> = app.running_devices().into_iter().map(|r| r.name).collect();
    assert_eq!(running.len(), 3, "{running:?}");

    app.delete_node(ws, mine, false).unwrap();
    app.delete_node(ws, other, false).unwrap();
    assert!(world.calls().contains(&"stop Mine".to_string()));
    assert!(!world.calls().iter().any(|c| c == "stop Outside"));
    app.quit();
    assert!(world.calls().contains(&"stop Kept".to_string()));
}

#[test]
fn stop_returns_the_device_to_off_and_it_can_boot_again() {
    let (_sb, app, _, world, ws) = setup();
    let d = device(&app, ws, "Pixel");
    app.device_boot(d).unwrap();
    assert!(wait_until(WAIT, || app.device_state(d).state == DeviceState::Ready));
    app.device_stop(d).unwrap();
    assert_eq!(app.device_state(d).state, DeviceState::Off);
    app.device_boot(d).unwrap();
    assert_eq!(
        world.calls(),
        vec!["start Pixel", "stop Pixel", "start Pixel"]
    );
    app.quit();
}

#[test]
fn a_node_without_a_chosen_device_cannot_boot() {
    let (_sb, app, _, _, ws) = setup();
    let n = app.create_device(ws, PHONE).unwrap().id;
    let err = app.device_boot(n).unwrap_err();
    assert!(err.message.contains("Choose a device"), "{}", err.message);
    app.quit();
}

#[test]
fn quit_check_lists_devices_sonata_started() {
    let (_sb, app, _, world, ws) = setup();
    world.outside.lock().unwrap().push("Outside".into());
    let mine = device(&app, ws, "Pixel 8");
    let other = device(&app, ws, "Outside");
    assert!(app.quit_check().is_empty(), "nothing running yet");
    app.device_boot(mine).unwrap();
    app.device_boot(other).unwrap();
    let busy = app.quit_check();
    assert_eq!(busy.len(), 1);
    assert_eq!(busy[0].count, 0);
    assert_eq!(busy[0].devices, vec!["Pixel 8".to_string()]);
    app.quit();
}

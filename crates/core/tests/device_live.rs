//! Live screen frames and node input (US1, research R2, R3).

mod common;

use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};

use common::fake_device::{FakeLauncher, FakeWorld};
use common::{Sandbox, WAIT, test_app, wait_until};
use sonata_core::device::DeviceState;
use sonata_core::device::live::{FRAME_HEADER, LiveInput, decode_header};
use sonata_core::model::node::{Frame, Platform};
use sonata_core::workspace::manager::CreateWorkspace;

fn ready_device() -> (Sandbox, Arc<sonata_core::App>, Arc<FakeWorld>, uuid::Uuid) {
    let sb = Sandbox::new();
    let (app, _) = test_app(&sb);
    let world = Arc::new(FakeWorld::default());
    world.boot_polls.store(0, Ordering::SeqCst);
    app.set_device_launcher(Arc::new(FakeLauncher(world.clone())));
    let ws = app
        .create_workspace(CreateWorkspace {
            name: "W".into(),
            working_directory: sb.project("p"),
        })
        .unwrap()
        .id;
    app.open_workspace(ws).unwrap();
    let d = app
        .create_device(
            ws,
            Frame {
                x: 0.0,
                y: 0.0,
                width: 380.0,
                height: 820.0,
            },
        )
        .unwrap()
        .id;
    app.select_device(d, Platform::Android, "Pixel", None, None)
        .unwrap();
    app.device_boot(d).unwrap();
    assert!(wait_until(WAIT, || app.device_state(d).state == DeviceState::Ready));
    (sb, app, world, d)
}

#[test]
fn only_the_latest_frame_is_sent_and_only_after_an_ack() {
    let (_sb, app, world, d) = ready_device();
    let sent: Arc<Mutex<Vec<Vec<u8>>>> = Arc::default();
    let out = sent.clone();
    app.device_stream(
        d,
        36,
        80,
        Box::new(move |bytes| out.lock().unwrap().push(bytes)),
    )
    .unwrap();
    assert!(world.calls().contains(&"stream 36x80".to_string()));

    world.emit(36, 80, 1);
    world.emit(36, 80, 2);
    world.emit(36, 80, 3);
    let seqs = |v: &Vec<Vec<u8>>| {
        v.iter()
            .map(|b| decode_header(b).unwrap().2)
            .collect::<Vec<_>>()
    };
    assert_eq!(seqs(&sent.lock().unwrap()), vec![1], "waits for the UI");
    app.device_frame_ack(d);
    assert_eq!(
        seqs(&sent.lock().unwrap()),
        vec![1, 3],
        "skips stale frame 2"
    );
    app.device_frame_ack(d);
    world.emit(36, 80, 4);
    let all = sent.lock().unwrap().clone();
    assert_eq!(seqs(&all), vec![1, 3, 4]);
    let last = all.last().unwrap();
    assert_eq!(decode_header(last), Some((36, 80, 4)));
    assert_eq!(last.len(), FRAME_HEADER + 36 * 80 * 4);
    app.quit();
}

#[test]
fn ending_the_stream_writes_a_private_card_image() {
    let (_sb, app, world, d) = ready_device();
    app.device_stream(d, 4, 8, Box::new(|_| {})).unwrap();
    world.emit(4, 8, 7);
    let card = app.device_unstream(d).unwrap().expect("card image");
    assert!(card.ends_with(format!("{d}.png")));
    assert_eq!(common::mode(&card), 0o600);
    let bytes = std::fs::read(&card).unwrap();
    assert_eq!(&bytes[..8], b"\x89PNG\r\n\x1a\n");
    // Nothing streamed: no card.
    assert!(
        app.device_unstream(d).unwrap().is_some(),
        "keeps the last card"
    );
    app.quit();
}

#[test]
fn node_input_reaches_the_device() {
    let (_sb, app, world, d) = ready_device();
    world.take_calls();
    for input in [
        LiveInput::Touch {
            x: 10,
            y: 20,
            down: true,
        },
        LiveInput::Touch {
            x: 10,
            y: 90,
            down: false,
        },
        LiveInput::Wheel {
            x: 5,
            y: 6,
            dx: 0,
            dy: -120,
        },
        LiveInput::Text("hi".into()),
        LiveInput::Key("Backspace".into()),
        LiveInput::Key("back".into()),
        LiveInput::Key("home".into()),
        LiveInput::Key("recents".into()),
    ] {
        app.device_input(d, input).unwrap();
    }
    assert_eq!(
        world.take_calls(),
        vec![
            "touch 10 20 down",
            "touch 10 90 up",
            "wheel 5 6 0 -120",
            "text hi",
            "key Named(\"Backspace\")",
            "key Back",
            "key Home",
            "key Recents",
        ]
    );
    app.quit();
}

#[test]
fn streaming_a_device_that_is_off_fails_clearly() {
    let (_sb, app, _world, d) = ready_device();
    app.device_stop(d).unwrap();
    let err = app.device_stream(d, 4, 8, Box::new(|_| {})).unwrap_err();
    assert_eq!(err.message, "device not running");
    app.quit();
}

#[test]
fn screenshots_are_private_files_at_the_chosen_path_or_in_shots() {
    let (sb, app, _world, d) = ready_device();
    let default = app.device_screenshot(d, None).unwrap();
    assert!(default.starts_with(sb.root.path().join("run/shots")));
    assert_eq!(common::mode(&default), 0o600);
    let out = sb.tmp.path().join("pixel.png");
    assert_eq!(app.device_screenshot(d, Some(&out)).unwrap(), out);
    assert_eq!(std::fs::read(&out).unwrap(), b"\x89PNG fake");
    assert_eq!(common::mode(&out), 0o600);
    app.device_stop(d).unwrap();
    assert_eq!(
        app.device_screenshot(d, None).unwrap_err().message,
        "device not running"
    );
    app.quit();
}

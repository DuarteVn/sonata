//! `sonata device …` from linked terminals, against a fake device (US3, contracts/cli.md).

mod common;

use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::Ordering;

use common::fake_device::{FakeLauncher, FakeWorld};
use common::{Sandbox, messaging_app, shells, sonata_as};
use sonata_core::model::node::{Frame, Platform};

fn fixture(name: &str) -> String {
    std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/device")
            .join(name),
    )
    .unwrap()
}

struct Setup {
    sb: Sandbox,
    app: Arc<sonata_core::App>,
    world: Arc<FakeWorld>,
    a: uuid::Uuid,
    b: uuid::Uuid,
    device: uuid::Uuid,
}

fn setup() -> Setup {
    let sb = Sandbox::new();
    let (app, _) = messaging_app(&sb);
    let world = Arc::new(FakeWorld::default());
    world.boot_polls.store(2, Ordering::SeqCst);
    *world.dump.lock().unwrap() = fixture("login.xml");
    *world.apps.lock().unwrap() = vec!["com.android.settings".into(), "com.example.shop".into()];
    app.set_device_launcher(Arc::new(FakeLauncher(world.clone())));
    let (ws, ids) = shells(&app, &sb, &["A", "B"]);
    let device = app
        .create_device(
            ws,
            Frame {
                x: 0.0,
                y: 900.0,
                width: 380.0,
                height: 820.0,
            },
        )
        .unwrap()
        .id;
    app.select_device(
        device,
        Platform::Android,
        "Pixel_8_API_34",
        Some("Pixel"),
        Some("Android 14"),
    )
    .unwrap();
    app.create_link(ws, ids[0], device).unwrap();
    Setup {
        sb,
        app,
        world,
        a: ids[0],
        b: ids[1],
        device,
    }
}

fn ok(s: &Setup, args: &[&str]) -> String {
    let out = sonata_as(&s.app, s.a, args);
    assert_eq!(out.code, 0, "{args:?}: {}", out.stderr);
    out.stdout
}

fn err(s: &Setup, args: &[&str]) -> (i32, String) {
    let out = sonata_as(&s.app, s.a, args);
    assert_ne!(out.code, 0, "{args:?} succeeded: {}", out.stdout);
    (out.code, out.stderr)
}

#[test]
fn list_shows_linked_devices_with_platform_and_state() {
    let s = setup();
    let list = ok(&s, &["list"]);
    assert!(
        list.contains("device")
            && list.contains("Pixel")
            && list.contains("android")
            && list.contains("off"),
        "{list}"
    );
    let devices = ok(&s, &["device", "list"]);
    assert!(
        devices.contains("Pixel") && devices.contains("off"),
        "{devices}"
    );
    s.app.quit();
}

#[test]
fn boot_waits_for_ready_then_commands_work() {
    let s = setup();
    let (_, msg) = err(&s, &["device", "read", "Pixel"]);
    assert!(msg.contains("device not running"), "{msg}");
    let boot = ok(&s, &["device", "boot", "pixel", "--timeout", "30s"]);
    assert!(boot.contains("ready"), "{boot}");
    let short = sonata_core::device::short_id(s.device);
    assert!(ok(&s, &["device", "list"]).contains("ready"));

    // read: numbered elements.
    let read = ok(&s, &["device", "read", &short]);
    assert!(
        read.contains("App: com.example.shop") && read.contains("[2] input    \"Email\""),
        "{read}"
    );

    s.world.take_calls();
    ok(&s, &["device", "tap", "Pixel", "5"]);
    ok(&s, &["device", "tap", "Pixel", "120", "480"]);
    ok(&s, &["device", "type", "Pixel", "hello world"]);
    ok(&s, &["device", "type", "Pixel", "a@b.c", "--into", "2"]);
    ok(&s, &["device", "key", "Pixel", "back"]);
    ok(&s, &["device", "key", "Pixel", "recents"]);
    let calls = s.world.take_calls();
    let expected = [
        "touch 540 1020 down",
        "touch 540 1020 up",
        "touch 120 480 down",
        "touch 120 480 up",
        "text hello world",
        "touch 540 560 down",
        "touch 540 560 up",
        "text a@b.c",
        "key Back",
        "key Recents",
    ];
    let got: Vec<&str> = calls
        .iter()
        .map(String::as_str)
        .filter(|c| *c != "dump")
        .collect();
    assert_eq!(got, expected);

    // swipe by direction (70% → 30% of the height, centered) and by points.
    ok(&s, &["device", "swipe", "Pixel", "up"]);
    let swipe = s.world.take_calls();
    assert_eq!(
        swipe.first().map(String::as_str),
        Some("touch 540 1680 down")
    );
    assert_eq!(swipe.last().map(String::as_str), Some("touch 540 720 up"));
    ok(
        &s,
        &[
            "device",
            "swipe",
            "Pixel",
            "100",
            "200",
            "300",
            "400",
            "--duration",
            "100ms",
        ],
    );
    let swipe = s.world.take_calls();
    assert_eq!(
        swipe.first().map(String::as_str),
        Some("touch 100 200 down")
    );
    assert_eq!(swipe.last().map(String::as_str), Some("touch 300 400 up"));

    // apps and launch.
    let apps = ok(&s, &["device", "apps", "Pixel"]);
    assert_eq!(apps.trim(), "com.android.settings\ncom.example.shop");
    ok(&s, &["device", "launch", "Pixel", "com.example.shop"]);
    let (code, msg) = err(&s, &["device", "launch", "Pixel", "com.missing"]);
    assert_eq!(code, 1);
    assert!(msg.contains("not installed or not launchable"));

    // screenshot: private file, path printed; also with --out.
    let shot = ok(&s, &["device", "screenshot", "Pixel"]);
    assert_eq!(common::mode(Path::new(shot.trim())), 0o600);
    let out = s.sb.tmp.path().join("shot.png");
    ok(
        &s,
        &[
            "device",
            "screenshot",
            "Pixel",
            "--out",
            out.to_str().unwrap(),
        ],
    );
    assert!(out.is_file());

    ok(&s, &["device", "stop", "Pixel"]);
    assert!(ok(&s, &["device", "list"]).contains("off"));
    s.app.quit();
}

#[test]
fn element_numbers_go_stale_when_the_screen_changes() {
    let s = setup();
    ok(&s, &["device", "boot", "Pixel"]);
    let (_, msg) = err(&s, &["device", "tap", "Pixel", "3"]);
    assert!(msg.contains("run read"), "no read yet: {msg}");
    ok(&s, &["device", "read", "Pixel"]);
    let (_, msg) = err(&s, &["device", "tap", "Pixel", "9"]);
    assert!(msg.contains("element 9 not found; run read again"), "{msg}");
    s.world.screen.fetch_add(1, Ordering::SeqCst);
    let (_, msg) = err(&s, &["device", "tap", "Pixel", "3"]);
    assert!(msg.contains("the screen changed; run read again"), "{msg}");
    ok(&s, &["device", "read", "Pixel"]);
    ok(&s, &["device", "tap", "Pixel", "3"]);
    s.app.quit();
}

#[test]
fn install_checks_the_file_and_prints_the_app_id() {
    let s = setup();
    ok(&s, &["device", "boot", "Pixel"]);
    let apk = s.sb.project("p").join("app-debug.apk");
    std::fs::write(&apk, b"PK").unwrap();
    let out = ok(&s, &["device", "install", "Pixel", apk.to_str().unwrap()]);
    assert!(out.contains("com.example.app"), "{out}");
    let txt = s.sb.project("p").join("notes.txt");
    std::fs::write(&txt, b"x").unwrap();
    for bad in [txt.to_str().unwrap(), "/nope/missing.apk", "../escape.apk"] {
        let (code, msg) = err(&s, &["device", "install", "Pixel", bad]);
        assert_eq!(code, 2, "{bad}: {msg}");
    }
    s.app.quit();
}

#[test]
fn unlinked_terminals_and_bad_usage_are_refused() {
    let s = setup();
    for sub in [
        "boot",
        "stop",
        "read",
        "tap",
        "swipe",
        "type",
        "key",
        "launch",
        "apps",
        "install",
        "screenshot",
    ] {
        let r = sonata_as(&s.app, s.b, &["device", sub, "Pixel", "1"]);
        assert_eq!(r.code, 3, "{sub}: {}", r.stderr);
    }
    assert_eq!(err(&s, &["device", "key", "Pixel", "volume"]).0, 2);
    assert_eq!(err(&s, &["device", "swipe", "Pixel", "sideways"]).0, 2);
    assert_eq!(err(&s, &["device", "tap", "Pixel"]).0, 2);
    assert_eq!(err(&s, &["device"]).0, 2);
    s.app.quit();
}

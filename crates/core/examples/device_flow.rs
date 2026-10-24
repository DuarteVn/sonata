//! Runs Sonata's device path against a real AVD (feature 007, tasks T024/T032):
//! attach or boot with the system launcher, then read, tap, type, keys, apps, launch, stop.
//!
//! Usage: `cargo run -p sonata-core --example device_flow -- <avd name>`

use std::time::{Duration, Instant};

use sonata_core::device::control::{self, Target as Tap};
use sonata_core::device::emulator::system_launcher;
use sonata_core::device::{DeviceLauncher, Poll, Target};
use sonata_core::model::node::Platform;

fn main() {
    let avd = std::env::args().nth(1).expect("avd name");
    let launcher = system_launcher(|| None);
    let t0 = Instant::now();
    let mut session = launcher
        .start(&Target {
            platform: Platform::Android,
            device_id: avd.clone(),
            name: avd,
        })
        .expect("start");
    println!("started by Sonata: {}", session.started_by_sonata());
    let driver = loop {
        match session.poll() {
            Poll::Ready(d) => break d,
            Poll::Booting => std::thread::sleep(Duration::from_millis(500)),
            Poll::Exited(m) => panic!("exited: {m:?}"),
        }
    };
    println!(
        "ready after {:?}, reduced: {}",
        t0.elapsed(),
        driver.reduced()
    );
    println!("screen {:?}", driver.screen_size());
    let d = driver.as_ref();

    d.key(&sonata_core::device::Key::Home).unwrap();
    control::settle(d, Instant::now());
    let apps = d.apps().unwrap();
    println!(
        "{} launchable apps, settings: {}",
        apps.len(),
        apps.iter().any(|a| a == "com.android.settings")
    );
    d.launch("com.android.settings").unwrap();
    let still = control::settle(d, Instant::now());
    println!("settings launched, still: {still}");
    let t = Instant::now();
    let (text, map) = control::read(d).expect("read");
    println!(
        "read in {:?}:\n{}",
        t.elapsed(),
        text.lines().take(14).collect::<Vec<_>>().join("\n")
    );
    let search = map
        .elements
        .iter()
        .find(|e| e.label.to_lowercase().contains("search"))
        .map(|e| e.number);
    if let Some(n) = search {
        let p = control::point(d, Some(&map), Tap::Element(n)).unwrap();
        control::tap(d, p).unwrap();
        control::settle(d, Instant::now());
        d.text("display").unwrap();
        control::settle(d, Instant::now());
        let (after, _) = control::read(d).unwrap();
        println!(
            "after typing:\n{}",
            after.lines().take(10).collect::<Vec<_>>().join("\n")
        );
        let stale = control::point(d, Some(&map), Tap::Element(n));
        println!("old numbers after screen change: {stale:?}");
    }
    for k in ["back", "back", "recents", "home"] {
        control::key(d, k).unwrap();
        control::settle(d, Instant::now());
        println!("key {k} ok");
    }
    let png = d.screenshot_png().unwrap();
    println!("screenshot {} bytes", png.len());
    if std::env::var("STOP").is_ok() {
        session.stop();
        println!("stop requested");
    }
}

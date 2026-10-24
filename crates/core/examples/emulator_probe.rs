//! Measures a running emulator through Sonata's gRPC client (feature 007, tasks T024/T025).
//!
//! Usage: `cargo run -p sonata-core --example emulator_probe -- <avd name> [width height]`
//! Reads the port and token from the emulator's discovery file; prints frame rate and the time
//! from a touch to the first changed frame.

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use sonata_core::device::avd::parse_ini;
use sonata_core::device::grpc::EmulatorClient;
use sonata_core::device::still::Signature;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let avd = args.first().expect("avd name");
    let width: u32 = args.get(1).and_then(|v| v.parse().ok()).unwrap_or(720);
    let height: u32 = args.get(2).and_then(|v| v.parse().ok()).unwrap_or(1600);
    let dir = std::path::PathBuf::from(std::env::var("HOME").unwrap())
        .join("Library/Caches/TemporaryItems/avd/running");
    let ini = std::fs::read_dir(&dir)
        .expect("discovery folder")
        .flatten()
        .map(|e| parse_ini(&std::fs::read_to_string(e.path()).unwrap_or_default()))
        .find(|i| i.get("avd.name") == Some(avd))
        .expect("running emulator");
    let port: u16 = ini["grpc.port"].parse().unwrap();
    let c = EmulatorClient::connect(port, &ini["grpc.token"]);

    let t = Instant::now();
    let png = c.screenshot_png().expect("screenshot");
    println!("screenshot png: {} bytes in {:?}", png.len(), t.elapsed());
    c.key("GoHome").expect("GoHome");
    println!("key GoHome ok");

    let frames = Arc::new(Mutex::new(Vec::<(Instant, Signature, usize)>::new()));
    let sink = frames.clone();
    let _stream = c.stream(width, height, move |f| {
        let sig = Signature::from_rgba(f.width, f.height, &f.rgba);
        sink.lock()
            .unwrap()
            .push((Instant::now(), sig, f.rgba.len()));
    });
    std::thread::sleep(Duration::from_millis(800));

    // Frame rate while the screen moves: swipe up and down for 3 seconds.
    frames.lock().unwrap().clear();
    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(3) {
        for (a, b) in [(1600, 700), (700, 1600)] {
            c.touch(540, a, true).unwrap();
            for i in 1..=10 {
                c.touch(540, a + (b - a) * i / 10, true).unwrap();
                std::thread::sleep(Duration::from_millis(16));
            }
            c.touch(540, b, false).unwrap();
        }
    }
    let n = frames.lock().unwrap().len();
    let bytes = frames.lock().unwrap().first().map(|f| f.2).unwrap_or(0);
    println!(
        "stream {width}x{height}: {n} frames in 3s = {:.1} fps, {bytes} bytes/frame",
        n as f64 / 3.0
    );

    // Click latency: tap the app drawer area and time the first clearly different frame.
    c.key("GoHome").unwrap();
    std::thread::sleep(Duration::from_millis(1500));
    let mut lat = Vec::new();
    for round in 0..5 {
        let before = frames.lock().unwrap().last().map(|f| f.1.clone());
        let t0 = Instant::now();
        if round % 2 == 0 {
            c.touch(540, 1200, true).unwrap();
            c.touch(540, 700, true).unwrap();
            c.touch(540, 700, false).unwrap();
        } else {
            c.key("GoHome").unwrap();
        }
        let deadline = t0 + Duration::from_secs(2);
        let got = loop {
            let changed = frames
                .lock()
                .unwrap()
                .iter()
                .rev()
                .find(|f| f.0 > t0)
                .and_then(|f| {
                    before
                        .as_ref()
                        .is_none_or(|b| b.differs(&f.1))
                        .then_some(f.0)
                });
            if let Some(at) = changed {
                break Some(at - t0);
            }
            if Instant::now() > deadline {
                break None;
            }
            std::thread::sleep(Duration::from_millis(2));
        };
        println!("input → changed frame: {got:?}");
        if let Some(d) = got {
            lat.push(d);
        }
        std::thread::sleep(Duration::from_millis(1200));
    }
    if !lat.is_empty() {
        lat.sort();
        println!(
            "median {:?}, max {:?}",
            lat[lat.len() / 2],
            lat.last().unwrap()
        );
    }
}

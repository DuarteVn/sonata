//! iOS simulators through simctl: list, boot, stop, screenshot, apps, launch, install; other
//! commands are refused (US4, research R11).

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use sonata_core::device::ios::{IosLauncher, parse_listapps, parse_simulators};
use sonata_core::device::process::{Child, Output, Processes};
use sonata_core::device::{DeviceLauncher, Key, Poll, Target};
use sonata_core::model::node::Platform;

fn fixture(name: &str) -> String {
    std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/device")
            .join(name),
    )
    .unwrap()
}

#[test]
fn lists_available_ios_simulators_with_version() {
    let sims = parse_simulators(&fixture("simctl_list.json")).unwrap();
    let got: Vec<(&str, &str, &str, bool)> = sims
        .iter()
        .map(|s| {
            (
                s.udid.as_str(),
                s.name.as_str(),
                s.os_version.as_str(),
                s.booted,
            )
        })
        .collect();
    assert_eq!(
        got,
        vec![
            ("BBBB-2222", "iPad Air", "iOS 18.2", true),
            ("AAAA-1111", "iPhone 16", "iOS 18.2", false),
        ]
    );
    assert_eq!(
        parse_listapps(&fixture("simctl_listapps.txt")),
        vec!["com.apple.Preferences", "com.example.shop"]
    );
}

struct Procs {
    calls: Mutex<Vec<String>>,
    list: Mutex<String>,
}

impl Processes for Procs {
    fn run(&self, program: &Path, args: &[&str], _: Duration) -> Result<Output, String> {
        let line = format!(
            "{} {}",
            program.file_name().unwrap().to_string_lossy(),
            args.join(" ")
        );
        self.calls.lock().unwrap().push(line);
        let stdout = match args {
            ["list", "devices", "available", "--json"] => self.list.lock().unwrap().clone(),
            ["listapps", _] => fixture("simctl_listapps.txt"),
            ["io", _, "screenshot", "--type=png", file] => {
                std::fs::write(file, b"\x89PNG ios").unwrap();
                String::new()
            }
            ["-extract", "CFBundleIdentifier", "raw", "-o", "-", _] => "com.example.ios\n".into(),
            _ => String::new(),
        };
        Ok(Output {
            success: true,
            stdout: stdout.into_bytes(),
            stderr: String::new(),
        })
    }
    fn spawn(&self, _: &Path, _: &[String]) -> Result<Box<dyn Child>, String> {
        Err("no".into())
    }
    fn port_free(&self, _: u16) -> bool {
        true
    }
    fn pid_alive(&self, _: u32) -> bool {
        true
    }
}

fn launcher() -> (Arc<Procs>, IosLauncher) {
    let procs = Arc::new(Procs {
        calls: Mutex::default(),
        list: Mutex::new(fixture("simctl_list.json")),
    });
    let dev = PathBuf::from("/Xcode/Contents/Developer");
    let l = IosLauncher::new(Box::new(move || Ok(dev.clone())), procs.clone());
    (procs, l)
}

fn target(udid: &str) -> Target {
    Target {
        platform: Platform::Ios,
        device_id: udid.into(),
        name: udid.into(),
    }
}

#[test]
fn boots_polls_and_stops_a_simulator() {
    let (procs, l) = launcher();
    let mut session = l.start(&target("AAAA-1111")).unwrap();
    assert!(session.started_by_sonata());
    assert!(
        matches!(session.poll(), Poll::Booting),
        "still Shutdown in the list"
    );
    *procs.list.lock().unwrap() = fixture("simctl_list.json").replace("\"Shutdown\"", "\"Booted\"");
    let Poll::Ready(driver) = session.poll() else {
        panic!("not ready")
    };
    session.stop();
    let calls = procs.calls.lock().unwrap().clone();
    assert!(
        calls.contains(&"simctl boot AAAA-1111".to_string()),
        "{calls:?}"
    );
    assert!(
        calls.contains(&"simctl shutdown AAAA-1111".to_string()),
        "{calls:?}"
    );
    assert!(driver.reduced(), "view only");
}

#[test]
fn an_already_booted_simulator_is_attached_not_stopped() {
    let (procs, l) = launcher();
    let mut session = l.start(&target("BBBB-2222")).unwrap();
    assert!(!session.started_by_sonata());
    assert!(matches!(session.poll(), Poll::Ready(_)));
    session.stop();
    assert!(
        !procs
            .calls
            .lock()
            .unwrap()
            .iter()
            .any(|c| c.contains("boot") || c.contains("shutdown"))
    );
}

#[test]
fn driver_supports_screenshots_apps_launch_install_and_refuses_input() {
    let (procs, l) = launcher();
    let mut session = l.start(&target("BBBB-2222")).unwrap();
    let Poll::Ready(d) = session.poll() else {
        panic!("not ready")
    };
    assert_eq!(d.screenshot_png().unwrap(), b"\x89PNG ios");
    assert_eq!(
        d.apps().unwrap(),
        vec!["com.apple.Preferences", "com.example.shop"]
    );
    d.launch("com.example.shop").unwrap();
    let app = tempfile::tempdir().unwrap();
    assert_eq!(d.install(app.path()).unwrap(), "com.example.ios");
    let before = procs.calls.lock().unwrap().len();
    let refused = [
        d.touch(1, 1, true),
        d.wheel(1, 1, 0, 120),
        d.text("x"),
        d.key(&Key::Back),
        d.ui_dump().map(|_| ()),
    ];
    for r in refused {
        assert_eq!(r.unwrap_err(), "not supported on iOS simulator");
    }
    assert_eq!(procs.calls.lock().unwrap().len(), before, "no effect");
    let calls = procs.calls.lock().unwrap().clone();
    assert!(calls.contains(&"simctl launch BBBB-2222 com.example.shop".to_string()));
    assert!(
        calls
            .iter()
            .any(|c| c.starts_with("simctl install BBBB-2222 "))
    );
}

//! Starting, attaching to, and stopping Android emulators with fake processes (US1, R1, R7).

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use sonata_core::device::emulator::{AndroidLauncher, Sdk};
use sonata_core::device::process::{Child, Output, Processes};
use sonata_core::device::{DeviceLauncher, Poll, Target};
use sonata_core::model::node::Platform;

#[derive(Default)]
struct World {
    calls: Mutex<Vec<String>>,
    /// `adb devices` output.
    devices: Mutex<String>,
    busy_ports: Mutex<Vec<u16>>,
    booted: AtomicBool,
    exited: Mutex<Option<String>>,
    alive_pids: Mutex<Vec<u32>>,
    spawned_args: Mutex<Vec<String>>,
}

struct FakeProcs(Arc<World>);

impl Processes for FakeProcs {
    fn run(&self, program: &Path, args: &[&str], _timeout: Duration) -> Result<Output, String> {
        let line = format!(
            "{} {}",
            program.file_name().unwrap().to_string_lossy(),
            args.join(" ")
        );
        self.0.calls.lock().unwrap().push(line.clone());
        let stdout = if args == ["devices"] {
            self.0.devices.lock().unwrap().clone()
        } else if line.ends_with("shell getprop sys.boot_completed") {
            if self.0.booted.load(Ordering::SeqCst) {
                "1\n"
            } else {
                "\n"
            }
            .to_string()
        } else {
            String::new()
        };
        Ok(Output {
            success: true,
            stdout: stdout.into_bytes(),
            stderr: String::new(),
        })
    }

    fn spawn(&self, program: &Path, args: &[String]) -> Result<Box<dyn Child>, String> {
        self.0.calls.lock().unwrap().push(format!(
            "spawn {}",
            program.file_name().unwrap().to_string_lossy()
        ));
        *self.0.spawned_args.lock().unwrap() = args.to_vec();
        Ok(Box::new(FakeChild(self.0.clone())))
    }

    fn port_free(&self, port: u16) -> bool {
        !self.0.busy_ports.lock().unwrap().contains(&port)
    }

    fn pid_alive(&self, pid: u32) -> bool {
        self.0.alive_pids.lock().unwrap().contains(&pid)
    }
}

struct FakeChild(Arc<World>);

impl Child for FakeChild {
    fn pid(&self) -> u32 {
        4242
    }
    fn exited(&mut self) -> Option<Option<String>> {
        self.0.exited.lock().unwrap().clone().map(Some)
    }
    fn kill(&mut self) {
        self.0.calls.lock().unwrap().push("kill 4242".into());
    }
}

struct Setup {
    _tmp: tempfile::TempDir,
    world: Arc<World>,
    launcher: AndroidLauncher,
    running_dir: PathBuf,
}

fn setup() -> Setup {
    let tmp = tempfile::tempdir().unwrap();
    let running_dir = tmp.path().join("running");
    std::fs::create_dir_all(&running_dir).unwrap();
    let world = Arc::new(World::default());
    *world.devices.lock().unwrap() =
        "List of devices attached\nemulator-5554\tdevice\nR58M123ABC\tdevice\n\n".into();
    let sdk = Sdk {
        emulator: PathBuf::from("/sdk/emulator/emulator"),
        adb: PathBuf::from("/sdk/platform-tools/adb"),
    };
    let launcher = AndroidLauncher::new(
        Box::new(move || Ok(sdk.clone())),
        Arc::new(FakeProcs(world.clone())),
        vec![running_dir.clone()],
    );
    Setup {
        _tmp: tmp,
        world,
        launcher,
        running_dir,
    }
}

fn target(id: &str) -> Target {
    Target {
        platform: Platform::Android,
        device_id: id.into(),
        name: id.into(),
    }
}

fn discovery(dir: &Path, pid: u32, entries: &[(&str, &str)]) {
    let text: String = entries.iter().map(|(k, v)| format!("{k}={v}\n")).collect();
    std::fs::write(dir.join(format!("pid_{pid}.ini")), text).unwrap();
}

#[test]
fn starts_a_hidden_emulator_on_free_ports_with_token_auth() {
    let s = setup();
    s.world.busy_ports.lock().unwrap().extend([5556, 8554]);
    let session = s.launcher.start(&target("Pixel_8")).unwrap();
    assert!(session.started_by_sonata());
    let args = s.world.spawned_args.lock().unwrap().clone();
    // 5554 is used by a running emulator, 5556 is busy, so 5558; gRPC 8554 is busy, so 8555.
    assert_eq!(
        args,
        [
            "-avd",
            "Pixel_8",
            "-port",
            "5558",
            "-grpc",
            "8555",
            "-grpc-use-token",
            "-qt-hide-window",
            "-no-boot-anim"
        ]
    );
    assert!(
        !args.iter().any(|a| a.contains("token=")),
        "token never on argv"
    );
}

#[test]
fn becomes_ready_when_boot_completes_and_reads_the_token_from_discovery() {
    let s = setup();
    let mut session = s.launcher.start(&target("Pixel_8")).unwrap();
    assert!(matches!(session.poll(), Poll::Booting));
    discovery(
        &s.running_dir,
        4242,
        &[
            ("avd.name", "Pixel_8"),
            ("port.serial", "5556"),
            ("grpc.port", "8554"),
            ("grpc.token", "tok"),
        ],
    );
    s.world.booted.store(true, Ordering::SeqCst);
    match session.poll() {
        Poll::Ready(driver) => assert!(!driver.reduced()),
        _ => panic!("not ready"),
    }
    let calls = s.world.calls.lock().unwrap().clone();
    assert!(
        calls.contains(&"adb -s emulator-5556 shell getprop sys.boot_completed".to_string()),
        "{calls:?}"
    );
}

#[test]
fn process_exit_is_reported_with_its_last_output() {
    let s = setup();
    let mut session = s.launcher.start(&target("Pixel_8")).unwrap();
    *s.world.exited.lock().unwrap() =
        Some("ERROR | x86_64 emulation currently requires hardware acceleration!".into());
    match session.poll() {
        Poll::Exited(Some(msg)) => assert!(msg.contains("hardware acceleration")),
        _ => panic!("expected exit"),
    }
}

#[test]
fn stop_asks_the_emulator_to_exit_gracefully() {
    let s = setup();
    let mut session = s.launcher.start(&target("Pixel_8")).unwrap();
    session.stop();
    let calls = s.world.calls.lock().unwrap().clone();
    assert!(
        calls.contains(&"adb -s emulator-5556 emu kill".to_string()),
        "{calls:?}"
    );
}

#[test]
fn attaches_to_an_emulator_already_running_and_never_stops_it() {
    let s = setup();
    s.world.alive_pids.lock().unwrap().push(777);
    discovery(
        &s.running_dir,
        777,
        &[
            ("avd.name", "Outside"),
            ("port.serial", "5554"),
            ("grpc.port", "8554"),
            ("grpc.token", "t"),
        ],
    );
    // A dead instance's file is ignored.
    discovery(
        &s.running_dir,
        778,
        &[("avd.name", "Gone"), ("port.serial", "5560")],
    );
    let mut session = s.launcher.start(&target("Outside")).unwrap();
    assert!(!session.started_by_sonata());
    assert!(
        s.world.spawned_args.lock().unwrap().is_empty(),
        "no second copy"
    );
    s.world.booted.store(true, Ordering::SeqCst);
    assert!(matches!(session.poll(), Poll::Ready(_)));
    session.stop();
    assert!(
        !s.world
            .calls
            .lock()
            .unwrap()
            .iter()
            .any(|c| c.contains("emu kill"))
    );

    // When the instance goes away the session ends.
    s.world.alive_pids.lock().unwrap().clear();
    assert!(matches!(session.poll(), Poll::Exited(None)));

    let mut gone = s.launcher.start(&target("Gone")).unwrap();
    assert!(gone.started_by_sonata(), "dead instance: start a new one");
    gone.stop();
}

#[test]
fn attached_without_a_token_runs_in_reduced_mode() {
    let s = setup();
    s.world.alive_pids.lock().unwrap().push(900);
    discovery(
        &s.running_dir,
        900,
        &[
            ("avd.name", "Jwt"),
            ("port.serial", "5562"),
            ("grpc.port", "8556"),
        ],
    );
    let mut session = s.launcher.start(&target("Jwt")).unwrap();
    s.world.booted.store(true, Ordering::SeqCst);
    match session.poll() {
        Poll::Ready(driver) => assert!(driver.reduced()),
        _ => panic!("not ready"),
    }
}

#[test]
fn a_missing_sdk_fails_the_start() {
    let world = Arc::new(World::default());
    let launcher = AndroidLauncher::new(
        Box::new(|| Err("Android SDK not found".to_string())),
        Arc::new(FakeProcs(world)),
        vec![],
    );
    assert_eq!(
        launcher.start(&target("Pixel")).err().unwrap(),
        "Android SDK not found"
    );
}

/// AVDs made by `avdmanager` have `hw.keyboard=no`; then the emulator drops gRPC key events, so
/// text and keys must go through adb (found on a real AVD).
#[test]
fn without_a_hardware_keyboard_text_and_keys_use_adb() {
    for (keyboard, via_adb) in [("no", true), ("yes", false)] {
        let s = setup();
        let avd_dir = s.running_dir.join(format!("Pixel_{keyboard}.avd"));
        std::fs::create_dir_all(&avd_dir).unwrap();
        std::fs::write(
            avd_dir.join("config.ini"),
            format!("hw.keyboard={keyboard}\n"),
        )
        .unwrap();
        let mut session = s.launcher.start(&target("Pixel_8")).unwrap();
        discovery(
            &s.running_dir,
            4242,
            &[
                ("avd.name", "Pixel_8"),
                ("port.serial", "5556"),
                ("grpc.port", "1"),
                ("grpc.token", "tok"),
                ("avd.dir", avd_dir.to_str().unwrap()),
            ],
        );
        s.world.booted.store(true, Ordering::SeqCst);
        let Poll::Ready(driver) = session.poll() else {
            panic!("not ready")
        };
        let _ = driver.text("hi");
        let _ = driver.key(&sonata_core::device::Key::Back);
        let calls = s.world.calls.lock().unwrap().clone();
        let adb_text = calls.iter().any(|c| c.ends_with("shell input text hi"));
        let adb_key = calls
            .iter()
            .any(|c| c.ends_with("shell input keyevent KEYCODE_BACK"));
        assert_eq!(
            (adb_text, adb_key),
            (via_adb, via_adb),
            "hw.keyboard={keyboard}: {calls:?}"
        );
    }
}

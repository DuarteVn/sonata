//! adb output parsing and the Android driver's adb paths, with fake processes (US3, R7, R8).

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use sonata_core::device::adb::{
    parse_aapt_package, parse_launchable, parse_packages, parse_wm_size, strip_dump_status,
};
use sonata_core::device::android::AndroidDriver;
use sonata_core::device::process::{Child, Output, Processes};
use sonata_core::device::{DeviceDriver, Key};

fn fixture(name: &str) -> String {
    std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/device")
            .join(name),
    )
    .unwrap()
}

#[test]
fn parsers() {
    assert_eq!(
        parse_launchable(&fixture("query_activities.txt")),
        vec![
            "com.android.chrome",
            "com.android.settings",
            "com.example.shop"
        ]
    );
    assert_eq!(
        parse_wm_size("Physical size: 1080x2400\n"),
        Some((1080, 2400))
    );
    assert_eq!(
        parse_wm_size("Physical size: 1080x2400\nOverride size: 720x1600\n"),
        Some((720, 1600))
    );
    assert_eq!(parse_wm_size("error"), None);
    assert_eq!(
        parse_packages("package:com.b\npackage:com.a\n\n"),
        vec!["com.a", "com.b"]
    );
    assert_eq!(
        parse_aapt_package("com.example.app\n").as_deref(),
        Some("com.example.app")
    );
    assert_eq!(parse_aapt_package("W/ziparchive: bad\n"), None);
    assert!(
        strip_dump_status("<hierarchy></hierarchy>UI hierchary dumped to: /dev/tty")
            .ends_with("</hierarchy>")
    );
}

/// Answers adb calls from a script keyed by the arguments after `-s <serial>`.
struct Script {
    calls: Mutex<Vec<String>>,
    answers: Mutex<Vec<(String, String)>>,
}

impl Script {
    fn new(answers: &[(&str, &str)]) -> Arc<Self> {
        Arc::new(Self {
            calls: Mutex::default(),
            answers: Mutex::new(
                answers
                    .iter()
                    .map(|(k, v)| (k.to_string(), v.to_string()))
                    .collect(),
            ),
        })
    }
    fn calls(&self) -> Vec<String> {
        self.calls.lock().unwrap().clone()
    }
}

struct Procs(Arc<Script>);

impl Processes for Procs {
    fn run(&self, program: &Path, args: &[&str], _: Duration) -> Result<Output, String> {
        let rest = if args.first() == Some(&"-s") {
            &args[2..]
        } else {
            args
        };
        let line = format!(
            "{} {}",
            program.file_name().unwrap().to_string_lossy(),
            rest.join(" ")
        );
        self.0.calls.lock().unwrap().push(line.clone());
        let mut answers = self.0.answers.lock().unwrap();
        // First matching answer is used once, so a command can answer differently later.
        let i = answers.iter().position(|(k, _)| line.ends_with(k.as_str()));
        let stdout = i.map(|i| answers.remove(i).1).unwrap_or_default();
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

fn driver(script: &Arc<Script>, adb: PathBuf) -> AndroidDriver {
    AndroidDriver::new(
        Arc::new(Procs(script.clone())),
        adb,
        "emulator-5554".into(),
        None,
        false,
    )
}

#[test]
fn apps_and_launch() {
    // `monkey` exits with 251 on emulators even when it worked, so launch resolves the
    // launcher activity and starts it (found on a real AVD).
    let s = Script::new(&[
        (
            "query-activities --brief -a android.intent.action.MAIN -c android.intent.category.LAUNCHER",
            &fixture("query_activities.txt"),
        ),
        (
            "resolve-activity --brief -c android.intent.category.LAUNCHER com.example.shop",
            "priority=0 preferredOrder=0 match=0x108000 specificIndex=-1 isDefault=true\ncom.example.shop/.MainActivity\n",
        ),
        (
            "resolve-activity --brief -c android.intent.category.LAUNCHER com.missing",
            "No activity found\n",
        ),
    ]);
    let d = driver(&s, "/sdk/platform-tools/adb".into());
    assert_eq!(d.apps().unwrap().len(), 3);
    assert!(d.launch("com.example.shop").is_ok());
    assert_eq!(
        d.launch("com.missing").unwrap_err(),
        "not installed or not launchable"
    );
    assert_eq!(
        d.launch("x; rm -rf /").unwrap_err(),
        "not installed or not launchable"
    );
    assert!(
        s.calls()
            .contains(&"adb shell am start -n com.example.shop/.MainActivity".to_string()),
        "{:?}",
        s.calls()
    );
}

#[test]
fn install_reports_the_app_id_from_aapt2_then_the_package_list() {
    let tmp = tempfile::tempdir().unwrap();
    let sdk = tmp.path();
    let adb = sdk.join("platform-tools/adb");
    // No build-tools: the id comes from the package list difference.
    let s = Script::new(&[
        ("pm list packages", "package:com.a\n"),
        (
            "install -r /x/app.apk",
            "Performing Streamed Install\nSuccess\n",
        ),
        ("pm list packages", "package:com.a\npackage:com.new\n"),
    ]);
    assert_eq!(
        driver(&s, adb.clone())
            .install(Path::new("/x/app.apk"))
            .unwrap(),
        "com.new"
    );

    // Reinstalling an app already there.
    let s = Script::new(&[
        ("pm list packages", "package:com.a\n"),
        ("install -r /x/app.apk", "Success\n"),
        ("pm list packages", "package:com.a\n"),
    ]);
    assert_eq!(
        driver(&s, adb.clone())
            .install(Path::new("/x/app.apk"))
            .unwrap(),
        "installed (update of an existing app)"
    );

    // With aapt2 in the newest build-tools.
    for v in ["33.0.0", "35.0.1"] {
        let dir = sdk.join("build-tools").join(v);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("aapt2"), "").unwrap();
    }
    let s = Script::new(&[
        ("install -r /x/app.apk", "Success\n"),
        ("aapt2 dump packagename /x/app.apk", "com.from.aapt\n"),
    ]);
    assert_eq!(
        driver(&s, adb.clone())
            .install(Path::new("/x/app.apk"))
            .unwrap(),
        "com.from.aapt"
    );
    assert!(s.calls().iter().any(|c| c.starts_with("aapt2")));

    let s = Script::new(&[(
        "install -r /x/app.apk",
        "Failure [INSTALL_FAILED_OLDER_SDK]\n",
    )]);
    assert_eq!(
        driver(&s, adb)
            .install(Path::new("/x/app.apk"))
            .unwrap_err(),
        "Failure [INSTALL_FAILED_OLDER_SDK]"
    );
}

#[test]
fn reduced_mode_uses_adb_input() {
    let s = Script::new(&[("wm size", "Physical size: 1080x2400\n")]);
    let d = driver(&s, "/sdk/platform-tools/adb".into());
    assert!(d.reduced());
    d.touch(100, 200, true).unwrap();
    d.touch(102, 201, false).unwrap();
    d.touch(100, 200, true).unwrap();
    d.touch(100, 900, false).unwrap();
    d.text("hi there").unwrap();
    d.key(&Key::Back).unwrap();
    d.key(&Key::Recents).unwrap();
    assert!(d.text("olá").is_err(), "non-ASCII is refused clearly");
    let calls = s.calls();
    assert!(
        calls.contains(&"adb shell input tap 102 201".to_string()),
        "{calls:?}"
    );
    assert!(
        calls
            .iter()
            .any(|c| c.starts_with("adb shell input swipe 100 200 100 900")),
        "{calls:?}"
    );
    assert!(calls.contains(&"adb shell input text hi%sthere".to_string()));
    assert!(calls.contains(&"adb shell input keyevent KEYCODE_BACK".to_string()));
    assert!(calls.contains(&"adb shell input keyevent KEYCODE_APP_SWITCH".to_string()));
}

#[test]
fn a_dump_that_never_settles_is_a_clear_error() {
    let s = Script::new(&[(
        "uiautomator dump /dev/tty",
        "ERROR: could not get idle state.\n",
    )]);
    let d = driver(&s, "/sdk/platform-tools/adb".into());
    assert_eq!(
        d.ui_dump().unwrap_err(),
        "screen never settled; try again or use coordinates"
    );
}

#[test]
fn a_dump_with_no_root_yet_is_retried() {
    // Right after an app starts, uiautomator can answer "null root node" once (real AVD).
    let s = Script::new(&[
        (
            "uiautomator dump /dev/tty",
            "ERROR: null root node returned by UiTestAutomationBridge.\n",
        ),
        (
            "uiautomator dump /dev/tty",
            "<hierarchy rotation=\"0\"></hierarchy>UI hierchary dumped to: /dev/tty\n",
        ),
    ]);
    let d = driver(&s, "/sdk/platform-tools/adb".into());
    assert_eq!(
        d.ui_dump().unwrap(),
        "<hierarchy rotation=\"0\"></hierarchy>"
    );
}

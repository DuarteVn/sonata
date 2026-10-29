//! Linux defaults for the device node (feature 011, research R12).
#![cfg(target_os = "linux")]

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use sonata_core::device::emulator::discovery_dirs;
use sonata_core::device::tools::{Env, detect};
use sonata_core::platform::{IOS_SIMULATORS, android_sdk_defaults, brew_prefixes};

fn env(home: &Path, vars: &[(&str, &str)]) -> Env {
    Env {
        home: home.to_path_buf(),
        vars: vars
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect::<HashMap<_, _>>(),
        brew_prefixes: Vec::new(),
    }
}

#[test]
fn android_studio_and_package_folders_are_the_sdk_defaults() {
    let home = Path::new("/home/me");
    assert_eq!(
        android_sdk_defaults(home),
        [
            PathBuf::from("/home/me/Android/Sdk"),
            "/opt/android-sdk".into(),
            "/usr/lib/android-sdk".into(),
        ]
    );
    assert!(brew_prefixes().is_empty());
}

#[test]
fn emulator_discovery_uses_the_runtime_folder() {
    let home = Path::new("/home/me");
    let dirs = discovery_dirs(&env(
        home,
        &[("XDG_RUNTIME_DIR", "/run/user/1000"), ("USER", "me")],
    ));
    assert_eq!(dirs[0], PathBuf::from("/run/user/1000/avd/running"));
    assert_eq!(dirs[1], PathBuf::from("/tmp/android-me/avd/running"));
    assert!(dirs.contains(&home.join(".android/avd/running")));
    assert!(!dirs.iter().any(|d| d.to_string_lossy().contains("Library")));
}

#[test]
fn ios_is_not_offered() {
    assert!(!IOS_SIMULATORS);
    let tmp = tempfile::tempdir().unwrap();
    let report = detect(&env(tmp.path(), &[]), None);
    assert!(report.ios.is_none());
    let json = serde_json::to_value(&report).unwrap();
    assert!(json["ios"].is_null());
}

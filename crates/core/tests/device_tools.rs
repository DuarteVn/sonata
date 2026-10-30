//! Finding the Android SDK, virtual devices, and Xcode without changing anything (US2, R6).
// Unix shells, modes, or sockets; Windows counterparts live in *_windows.rs (feature 012).
#![cfg(unix)]

mod common;

use std::collections::HashMap;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use sonata_core::device::avd::{android_version, list_avds, parse_ini};
use sonata_core::device::tools::{Env, detect};
use sonata_core::platform::android_sdk_defaults;

/// The platform's first default SDK folder (`~/Library/Android/sdk` on macOS, `~/Android/Sdk`
/// on Linux).
fn default_sdk(home: &Path) -> PathBuf {
    android_sdk_defaults(home).remove(0)
}

fn exe(path: &Path) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, "#!/bin/sh\n").unwrap();
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
}

fn sdk(root: &Path) -> PathBuf {
    exe(&root.join("emulator/emulator"));
    exe(&root.join("platform-tools/adb"));
    root.to_path_buf()
}

fn avd(home: &Path, name: &str, display: &str, api: u32) {
    let dir = home.join(format!("{name}.avd"));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        home.join(format!("{name}.ini")),
        format!(
            "avd.ini.encoding=UTF-8\npath={}\npath.rel=avd/{name}.avd\ntarget=android-{api}\n",
            dir.display()
        ),
    )
    .unwrap();
    std::fs::write(
        dir.join("config.ini"),
        format!(
            "AvdId={name}\navd.ini.displayname={display}\nabi.type=arm64-v8a\nhw.lcd.height=2400\nhw.lcd.width=1080\nimage.sysdir.1=system-images/android-{api}/google_apis/arm64-v8a/\n"
        ),
    )
    .unwrap();
}

fn env(home: &Path, vars: &[(&str, &Path)]) -> Env {
    Env {
        home: home.to_path_buf(),
        // Homebrew prefixes inside the test folder, never the real ones.
        brew_prefixes: vec![home.join("brew")],
        vars: vars
            .iter()
            .map(|(k, v)| (k.to_string(), v.display().to_string()))
            .collect::<HashMap<_, _>>(),
    }
}

#[test]
fn nothing_installed_reports_every_place_looked() {
    // Detection only reads files: it runs no program, so it cannot trigger an installer.
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    let r = detect(&env(&home, &[]), None);
    assert!(r.android.sdk.is_none());
    let mut looked = android_sdk_defaults(&home);
    looked.push(home.join("brew/share/android-commandlinetools"));
    assert_eq!(r.android.looked, looked);
    assert!(
        r.android
            .help_url
            .starts_with("https://developer.android.com/")
    );
    // iOS is offered only on macOS (feature 011).
    assert_eq!(r.ios.is_some(), cfg!(target_os = "macos"));
    if let Some(ios) = &r.ios {
        assert!(ios.developer_dir.is_none() || Path::new("/Applications/Xcode.app").exists());
        assert!(ios.help_url.starts_with("https://developer.apple.com/"));
    }
}

#[test]
fn sdk_candidates_in_order_and_only_complete_ones_count() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    let chosen = tmp.path().join("chosen");
    std::fs::create_dir_all(chosen.join("emulator")).unwrap(); // no adb: incomplete
    let from_home_var = sdk(&tmp.path().join("android_home"));
    let from_root_var = sdk(&tmp.path().join("sdk_root"));
    let studio = sdk(&default_sdk(&home));
    let xcode = tmp.path().join("Xcode.app/Contents/Developer");
    exe(&xcode.join("usr/bin/simctl"));

    let r = detect(
        &env(
            &home,
            &[
                ("ANDROID_HOME", &from_home_var),
                ("ANDROID_SDK_ROOT", &from_root_var),
                ("DEVELOPER_DIR", &xcode),
            ],
        ),
        Some(&chosen),
    );
    assert_eq!(r.android.sdk.as_deref(), Some(from_home_var.as_path()));
    assert_eq!(
        r.android.looked,
        vec![chosen.clone(), from_home_var.clone()]
    );
    if let Some(ios) = &r.ios {
        assert_eq!(ios.developer_dir.as_deref(), Some(xcode.as_path()));
    }

    let r = detect(&env(&home, &[("ANDROID_SDK_ROOT", &from_root_var)]), None);
    assert_eq!(r.android.sdk.as_deref(), Some(from_root_var.as_path()));
    let r = detect(&env(&home, &[]), None);
    assert_eq!(r.android.sdk.as_deref(), Some(studio.as_path()));
    let r = detect(&env(&home, &[]), Some(&from_root_var));
    assert_eq!(
        r.android.sdk.as_deref(),
        Some(from_root_var.as_path()),
        "preference first"
    );
}

#[test]
fn the_homebrew_command_line_tools_folder_counts_without_android_studio() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    let brew = sdk(&home.join("brew/share/android-commandlinetools"));
    let r = detect(&env(&home, &[]), None);
    assert_eq!(r.android.sdk.as_deref(), Some(brew.as_path()));
}

#[test]
fn avds_are_listed_from_the_avd_home_in_order() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    sdk(&default_sdk(&home));

    // Default location, empty.
    let r = detect(&env(&home, &[]), None);
    assert!(r.android.avds.is_empty());
    assert_eq!(r.android.avd_home, home.join(".android/avd"));
    assert!(
        r.android
            .avd_help_url
            .starts_with("https://developer.android.com/")
    );

    avd(&home.join(".android/avd"), "Pixel_8_API_34", "Pixel 8", 34);
    avd(&home.join(".android/avd"), "Old_Tablet", "Old Tablet", 30);
    let r = detect(&env(&home, &[]), None);
    let names: Vec<_> = r.android.avds.iter().map(|a| a.name.as_str()).collect();
    assert_eq!(names, vec!["Old_Tablet", "Pixel_8_API_34"]);
    let pixel = &r.android.avds[1];
    assert_eq!(pixel.display_name, "Pixel 8");
    assert_eq!(pixel.api_level, Some(34));
    assert_eq!(pixel.os_version.as_deref(), Some("Android 14"));
    assert_eq!(pixel.screen, Some((1080, 2400)));

    // Overrides, in order.
    let user_home = tmp.path().join("user");
    avd(&user_home.join("avd"), "From_User_Home", "U", 35);
    let emu_home = tmp.path().join("emu");
    avd(&emu_home.join("avd"), "From_Emulator_Home", "E", 35);
    let avd_home = tmp.path().join("avds");
    avd(&avd_home, "From_Avd_Home", "A", 35);
    let only = |vars: &[(&str, &Path)]| {
        detect(&env(&home, vars), None)
            .android
            .avds
            .into_iter()
            .map(|a| a.name)
            .collect::<Vec<_>>()
    };
    assert_eq!(
        only(&[("ANDROID_USER_HOME", &user_home)]),
        vec!["From_User_Home"]
    );
    assert_eq!(
        only(&[
            ("ANDROID_USER_HOME", &user_home),
            ("ANDROID_EMULATOR_HOME", &emu_home)
        ]),
        vec!["From_Emulator_Home"]
    );
    assert_eq!(
        only(&[
            ("ANDROID_EMULATOR_HOME", &emu_home),
            ("ANDROID_AVD_HOME", &avd_home)
        ]),
        vec!["From_Avd_Home"]
    );
}

#[test]
fn ini_and_version_helpers() {
    let ini = parse_ini("a=1\n# comment\n b = two words \n\nbad line\n");
    assert_eq!(ini.get("a").map(String::as_str), Some("1"));
    assert_eq!(ini.get("b").map(String::as_str), Some("two words"));
    assert_eq!(ini.len(), 2);
    assert_eq!(android_version(34), "Android 14");
    assert_eq!(android_version(32), "Android 12L");
    assert_eq!(android_version(36), "Android 16");
    assert_eq!(android_version(99), "API 99");
    let tmp = tempfile::tempdir().unwrap();
    assert!(list_avds(&tmp.path().join("missing")).is_empty());
}

#[test]
fn choosing_an_sdk_folder_saves_it_only_when_complete() {
    let sb = common::Sandbox::new();
    let (app, _) = common::test_app(&sb);
    let empty = sb.tmp.path().join("empty");
    std::fs::create_dir_all(&empty).unwrap();
    let r = app.device_choose_sdk(&empty);
    assert!(r.android.looked.contains(&empty));
    assert_ne!(r.android.sdk.as_deref(), Some(empty.as_path()));
    assert!(
        app.device_tools()
            .android
            .looked
            .iter()
            .all(|p| p != &empty)
    );

    let good = sdk(&sb.tmp.path().join("good"));
    let r = app.device_choose_sdk(&good);
    assert_eq!(r.android.sdk.as_deref(), Some(good.as_path()));
    assert_eq!(
        app.device_tools().android.sdk.as_deref(),
        Some(good.as_path())
    );
    let saved = sonata_core::persistence::store::Store::new(sb.root.clone()).load_preferences();
    assert_eq!(
        saved.value.android_sdk_path.as_deref(),
        Some(good.as_path())
    );
    app.quit();
}

#[test]
fn only_known_help_pages_open() {
    let sb = common::Sandbox::new();
    let (app, _) = common::test_app(&sb);
    assert!(app.device_open_help("https://evil.example").is_err());
    app.quit();
}

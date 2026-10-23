//! Finding the Android SDK, its virtual devices, and Xcode (research R6, FR-009, FR-010).
//!
//! Detection only reads files and environment variables. It runs no program, so it can never
//! start an installer (for example the developer-tools prompt that `xcrun` shows on a Mac
//! without them), and it never changes anything on the system.

use std::collections::HashMap;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use serde::Serialize;

use super::avd::{Avd, list_avds};

pub const ANDROID_HELP_URL: &str = "https://developer.android.com/studio/install";
pub const AVD_HELP_URL: &str = "https://developer.android.com/studio/run/managing-avds";
pub const XCODE_HELP_URL: &str = "https://developer.apple.com/xcode/";

/// The parts of the process environment detection reads; tests build their own.
#[derive(Debug, Clone)]
pub struct Env {
    pub home: PathBuf,
    pub vars: HashMap<String, String>,
}

impl Env {
    pub fn current() -> Self {
        Self {
            home: std::env::var_os("HOME")
                .map(PathBuf::from)
                .unwrap_or_default(),
            vars: std::env::vars().collect(),
        }
    }

    fn path(&self, key: &str) -> Option<PathBuf> {
        self.vars
            .get(key)
            .map(|v| v.trim())
            .filter(|v| !v.is_empty())
            .map(PathBuf::from)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AndroidReport {
    /// The SDK folder in use; `None` when no candidate has the emulator and adb.
    pub sdk: Option<PathBuf>,
    /// Candidates checked, in order, up to the one found.
    pub looked: Vec<PathBuf>,
    pub avd_home: PathBuf,
    pub avds: Vec<Avd>,
    pub help_url: &'static str,
    pub avd_help_url: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IosReport {
    /// Xcode's `Developer` folder when it holds `simctl`.
    pub developer_dir: Option<PathBuf>,
    pub help_url: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolReport {
    pub android: AndroidReport,
    pub ios: IosReport,
}

impl AndroidReport {
    pub fn emulator(&self) -> Option<PathBuf> {
        self.sdk.as_ref().map(|s| s.join("emulator/emulator"))
    }

    pub fn adb(&self) -> Option<PathBuf> {
        self.sdk.as_ref().map(|s| s.join("platform-tools/adb"))
    }
}

fn executable(path: &Path) -> bool {
    std::fs::metadata(path).is_ok_and(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
}

fn complete_sdk(dir: &Path) -> bool {
    executable(&dir.join("emulator/emulator")) && executable(&dir.join("platform-tools/adb"))
}

/// Preference, `ANDROID_HOME`, `ANDROID_SDK_ROOT`, then Android Studio's default folder.
fn sdk_candidates(env: &Env, preference: Option<&Path>) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = Vec::new();
    let all = [
        preference.map(Path::to_path_buf),
        env.path("ANDROID_HOME"),
        env.path("ANDROID_SDK_ROOT"),
        Some(env.home.join("Library/Android/sdk")),
    ];
    for c in all.into_iter().flatten() {
        if !out.contains(&c) {
            out.push(c);
        }
    }
    out
}

/// `ANDROID_AVD_HOME`, `ANDROID_EMULATOR_HOME/avd`, `ANDROID_USER_HOME/avd`, `~/.android/avd`.
pub fn avd_home(env: &Env) -> PathBuf {
    env.path("ANDROID_AVD_HOME")
        .or_else(|| env.path("ANDROID_EMULATOR_HOME").map(|p| p.join("avd")))
        .or_else(|| env.path("ANDROID_USER_HOME").map(|p| p.join("avd")))
        .unwrap_or_else(|| env.home.join(".android/avd"))
}

/// `DEVELOPER_DIR`, the folder chosen with `xcode-select`, then `/Applications/Xcode.app`.
fn developer_dir(env: &Env) -> Option<PathBuf> {
    [
        env.path("DEVELOPER_DIR"),
        std::fs::read_link("/var/db/xcode_select_link").ok(),
        Some(PathBuf::from("/Applications/Xcode.app/Contents/Developer")),
    ]
    .into_iter()
    .flatten()
    .find(|d| executable(&d.join("usr/bin/simctl")))
}

pub fn detect(env: &Env, preference: Option<&Path>) -> ToolReport {
    let mut looked = Vec::new();
    let mut sdk = None;
    for c in sdk_candidates(env, preference) {
        looked.push(c.clone());
        if complete_sdk(&c) {
            sdk = Some(c);
            break;
        }
    }
    let home = avd_home(env);
    ToolReport {
        android: AndroidReport {
            avds: if sdk.is_some() {
                list_avds(&home)
            } else {
                Vec::new()
            },
            sdk,
            looked,
            avd_home: home,
            help_url: ANDROID_HELP_URL,
            avd_help_url: AVD_HELP_URL,
        },
        ios: IosReport {
            developer_dir: developer_dir(env),
            help_url: XCODE_HELP_URL,
        },
    }
}

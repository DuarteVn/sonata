//! Android virtual device files (research R6). Pure parsing: no SDK tool is run to list devices.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Avd {
    /// The id the emulator takes with `-avd`.
    pub name: String,
    pub display_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_level: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub os_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub screen: Option<(u32, u32)>,
}

/// `key=value` lines; blank lines, comments, and lines without `=` are skipped.
pub fn parse_ini(text: &str) -> HashMap<String, String> {
    text.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .filter_map(|l| l.split_once('='))
        .map(|(k, v)| (k.trim().to_string(), v.trim().to_string()))
        .collect()
}

/// Marketing version for an API level.
pub fn android_version(api: u32) -> String {
    let v = match api {
        21 => "5.0",
        22 => "5.1",
        23 => "6",
        24 => "7.0",
        25 => "7.1",
        26 => "8.0",
        27 => "8.1",
        28 => "9",
        29 => "10",
        30 => "11",
        31 => "12",
        32 => "12L",
        33 => "13",
        34 => "14",
        35 => "15",
        36 => "16",
        _ => return format!("API {api}"),
    };
    format!("Android {v}")
}

/// First number after `android-` in values like `android-34` or
/// `system-images/android-34/google_apis/arm64-v8a/`.
fn api_from(value: &str) -> Option<u32> {
    let rest = &value[value.find("android-")? + "android-".len()..];
    let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
    digits.parse().ok()
}

/// Every `<name>.ini` in the AVD folder, sorted by name. Unreadable entries are skipped.
pub fn list_avds(avd_home: &Path) -> Vec<Avd> {
    let Ok(entries) = std::fs::read_dir(avd_home) else {
        return Vec::new();
    };
    let mut avds: Vec<Avd> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "ini") && p.is_file())
        .filter_map(|p| read_avd(avd_home, &p))
        .collect();
    avds.sort_by(|a, b| a.name.cmp(&b.name));
    avds
}

fn read_avd(avd_home: &Path, ini_path: &Path) -> Option<Avd> {
    let name = ini_path.file_stem()?.to_str()?.to_string();
    let ini = parse_ini(&std::fs::read_to_string(ini_path).ok()?);
    let dir = ini
        .get("path")
        .map(PathBuf::from)
        .filter(|p| p.is_dir())
        .unwrap_or_else(|| avd_home.join(format!("{name}.avd")));
    let config = std::fs::read_to_string(dir.join("config.ini"))
        .map(|t| parse_ini(&t))
        .unwrap_or_default();
    let api_level = config
        .get("image.sysdir.1")
        .and_then(|v| api_from(v))
        .or_else(|| ini.get("target").and_then(|v| api_from(v)));
    let size = |k: &str| config.get(k).and_then(|v| v.parse::<u32>().ok());
    Some(Avd {
        display_name: config
            .get("avd.ini.displayname")
            .filter(|d| !d.is_empty())
            .cloned()
            .unwrap_or_else(|| name.replace('_', " ")),
        name,
        os_version: api_level.map(android_version),
        api_level,
        screen: size("hw.lcd.width").zip(size("hw.lcd.height")),
    })
}

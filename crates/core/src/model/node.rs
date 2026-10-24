use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const TERMINAL_MIN_SIZE: (f64, f64) = (240.0, 120.0);
pub const NOTE_MIN_SIZE: (f64, f64) = (120.0, 80.0);
pub const BROWSER_MIN_SIZE: (f64, f64) = (320.0, 240.0);
pub const DEVICE_MIN_SIZE: (f64, f64) = (200.0, 300.0);
pub const TERMINAL_NAME_MAX_CHARS: usize = 60;
pub const FONT_SIZE_RANGE: std::ops::RangeInclusive<u32> = 9..=24;
pub const NOTE_TEXT_SIZES: std::ops::RangeInclusive<u8> = 1..=5;
pub const ROLE_TEXT_MAX_CHARS: usize = 20_000;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Node {
    pub id: Uuid,
    pub frame: Frame,
    pub z: i64,
    pub created_at: DateTime<Utc>,
    pub modified_at: DateTime<Utc>,
    pub content: NodeContent,
}

impl Node {
    pub fn min_size(&self) -> (f64, f64) {
        match self.content {
            NodeContent::Terminal(_) => TERMINAL_MIN_SIZE,
            NodeContent::Note(_) => NOTE_MIN_SIZE,
            NodeContent::Browser(_) => BROWSER_MIN_SIZE,
            NodeContent::Device(_) => DEVICE_MIN_SIZE,
        }
    }

    pub fn terminal(&self) -> Option<&TerminalContent> {
        match &self.content {
            NodeContent::Terminal(t) => Some(t),
            _ => None,
        }
    }

    pub fn terminal_mut(&mut self) -> Option<&mut TerminalContent> {
        match &mut self.content {
            NodeContent::Terminal(t) => Some(t),
            _ => None,
        }
    }

    pub fn is_note(&self) -> bool {
        matches!(self.content, NodeContent::Note(_))
    }

    pub fn browser(&self) -> Option<&BrowserContent> {
        match &self.content {
            NodeContent::Browser(b) => Some(b),
            _ => None,
        }
    }

    pub fn browser_mut(&mut self) -> Option<&mut BrowserContent> {
        match &mut self.content {
            NodeContent::Browser(b) => Some(b),
            _ => None,
        }
    }

    pub fn device(&self) -> Option<&DeviceContent> {
        match &self.content {
            NodeContent::Device(d) => Some(d),
            _ => None,
        }
    }

    pub fn device_mut(&mut self) -> Option<&mut DeviceContent> {
        match &mut self.content {
            NodeContent::Device(d) => Some(d),
            _ => None,
        }
    }

    pub fn note_style(&self) -> Option<&NoteContent> {
        match &self.content {
            NodeContent::Note(n) => Some(n),
            _ => None,
        }
    }

    pub fn note_style_mut(&mut self) -> Option<&mut NoteContent> {
        match &mut self.content {
            NodeContent::Note(n) => Some(n),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Frame {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl Frame {
    /// Enforces a minimum size and replaces non-finite numbers.
    pub fn clamped(self, (min_w, min_h): (f64, f64)) -> Self {
        let finite = |v: f64, d: f64| if v.is_finite() { v } else { d };
        Self {
            x: finite(self.x, 0.0),
            y: finite(self.y, 0.0),
            width: finite(self.width, min_w).max(min_w),
            height: finite(self.height, min_h).max(min_h),
        }
    }
}

/// Exactly one key in JSON: `{"terminal": {...}}` or `{"note": {}}`. Serde's externally tagged
/// representation rejects objects with zero or several keys.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum NodeContent {
    Terminal(TerminalContent),
    Note(NoteContent),
    Browser(BrowserContent),
    Device(DeviceContent),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Platform {
    Android,
    Ios,
}

/// A virtual device on the canvas (feature 007). The running device lives in `core::device`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceContent {
    pub platform: Platform,
    /// AVD name (Android) or simulator UDID (iOS); empty until the user picks one.
    #[serde(default)]
    pub device_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub os_version: Option<String>,
}

/// A web page on the canvas (feature 004). The page itself lives in the shell.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowserContent {
    pub url: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminalContent {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub launch_command: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub working_directory: Option<PathBuf>,
    #[serde(default = "default_true")]
    pub save_history: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub accent_color: Option<String>,
    /// Overrides the app's terminal font size (9–24).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_size: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role: Option<RoleRef>,
    #[serde(default = "default_true")]
    pub monitor_activity: bool,
    /// Gives the terminal the orchestrator commands of feature 003.
    #[serde(default)]
    pub orchestrator: bool,
    /// The orchestrator that recruited this terminal; only it may dismiss it (003 FR-022).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recruited_by: Option<Uuid>,
    /// Type a short Sonata instruction once the agent is ready; `None` = automatic (003 FR-019).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub send_instructions: Option<bool>,
    /// The terminal's own git checkout and branch (feature 008); set only at creation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lane: Option<Box<Lane>>,
}

impl TerminalContent {
    pub fn new(name: String) -> Self {
        Self {
            name,
            launch_command: None,
            working_directory: None,
            save_history: true,
            accent_color: None,
            font_size: None,
            role: None,
            monitor_activity: true,
            orchestrator: false,
            recruited_by: None,
            send_instructions: None,
            lane: None,
        }
    }
}

/// A terminal's isolated checkout (feature 008, data-model.md).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Lane {
    pub branch: String,
    pub base_branch: String,
    /// Top folder of the workspace's repository; land runs here.
    pub main_checkout: PathBuf,
    /// `<data root>/lanes/<workspace id>/<slug>`.
    pub checkout_path: PathBuf,
    /// The workspace folder relative to `main_checkout`, when it is a subfolder.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subdirectory: Option<PathBuf>,
    pub created_at: DateTime<Utc>,
    /// Set by a successful land; the checkout is gone after that.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub landed_at: Option<DateTime<Utc>>,
}

impl Lane {
    /// Where the terminal runs while the lane is active.
    pub fn working_directory(&self) -> PathBuf {
        match &self.subdirectory {
            Some(sub) => self.checkout_path.join(sub),
            None => self.checkout_path.clone(),
        }
    }

    /// The checkout folder name, which matches the branch after `lane/`.
    pub fn slug(&self) -> &str {
        self.branch
            .strip_prefix(crate::lanes::naming::BRANCH_PREFIX)
            .unwrap_or(&self.branch)
    }

    pub fn is_landed(&self) -> bool {
        self.landed_at.is_some()
    }
}

/// A terminal's own copy of a role. `name` is set when it came from the role library.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoleRef {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub text: String,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum NoteColor {
    #[default]
    Yellow,
    Blue,
    Green,
    Pink,
    Purple,
    Gray,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NoteContent {
    #[serde(default)]
    pub color: NoteColor,
    /// 1–5; 3 is the default size.
    #[serde(default = "default_text_size")]
    pub text_size: u8,
}

impl Default for NoteContent {
    fn default() -> Self {
        Self {
            color: NoteColor::Yellow,
            text_size: default_text_size(),
        }
    }
}

fn default_text_size() -> u8 {
    3
}

fn default_true() -> bool {
    true
}

use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const TERMINAL_MIN_SIZE: (f64, f64) = (240.0, 120.0);
pub const NOTE_MIN_SIZE: (f64, f64) = (120.0, 80.0);
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
        }
    }

    pub fn terminal(&self) -> Option<&TerminalContent> {
        match &self.content {
            NodeContent::Terminal(t) => Some(t),
            NodeContent::Note(_) => None,
        }
    }

    pub fn terminal_mut(&mut self) -> Option<&mut TerminalContent> {
        match &mut self.content {
            NodeContent::Terminal(t) => Some(t),
            NodeContent::Note(_) => None,
        }
    }

    pub fn is_note(&self) -> bool {
        matches!(self.content, NodeContent::Note(_))
    }

    pub fn note_style(&self) -> Option<&NoteContent> {
        match &self.content {
            NodeContent::Note(n) => Some(n),
            NodeContent::Terminal(_) => None,
        }
    }

    pub fn note_style_mut(&mut self) -> Option<&mut NoteContent> {
        match &mut self.content {
            NodeContent::Note(n) => Some(n),
            NodeContent::Terminal(_) => None,
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
        }
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

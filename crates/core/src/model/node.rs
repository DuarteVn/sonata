use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const TERMINAL_MIN_SIZE: (f64, f64) = (240.0, 120.0);
pub const NOTE_MIN_SIZE: (f64, f64) = (120.0, 80.0);
pub const TERMINAL_NAME_MAX_CHARS: usize = 60;

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
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct NoteContent {}

fn default_true() -> bool {
    true
}

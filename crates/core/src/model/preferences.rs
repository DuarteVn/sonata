use serde::{Deserialize, Serialize};

pub const PREFERENCES_TYPE: &str = "preferences";
pub const PREFERENCES_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Preferences {
    /// 5–600, default 30.
    pub autosave_interval_seconds: u32,
    /// 9–24, default 13.
    pub terminal_font_size: u32,
    /// 1,000–100,000, default 10,000.
    pub history_max_rows: u32,
    /// Check for a newer release at launch and daily (feature 005). Written only when off.
    #[serde(skip_serializing_if = "is_true")]
    pub auto_update_check: bool,
}

fn is_true(value: &bool) -> bool {
    *value
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            autosave_interval_seconds: 30,
            terminal_font_size: 13,
            history_max_rows: 10_000,
            auto_update_check: true,
        }
    }
}

impl Preferences {
    pub fn normalized(self) -> Self {
        Self {
            autosave_interval_seconds: self.autosave_interval_seconds.clamp(5, 600),
            terminal_font_size: self.terminal_font_size.clamp(9, 24),
            history_max_rows: self.history_max_rows.clamp(1_000, 100_000),
            auto_update_check: self.auto_update_check,
        }
    }
}

use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::usage::consent::Consent;

pub const PREFERENCES_TYPE: &str = "preferences";
pub const PREFERENCES_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
    /// Usage meter settings (feature 006). Written only when not the default.
    #[serde(skip_serializing_if = "UsagePreferences::is_default")]
    pub usage: UsagePreferences,
    /// Android SDK folder chosen in a device node (feature 007, FR-009).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub android_sdk_path: Option<PathBuf>,
}

/// Usage meter switches and per-provider choices (data-model "UsagePreferences").
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct UsagePreferences {
    /// Master switch (FR-016). Written only when off.
    #[serde(skip_serializing_if = "is_true")]
    pub enabled: bool,
    /// Notifications at 80% and 100% (FR-011). Written only when off.
    #[serde(skip_serializing_if = "is_true")]
    pub warnings: bool,
    #[serde(
        skip_serializing_if = "all_empty",
        serialize_with = "serialize_non_empty"
    )]
    pub providers: BTreeMap<String, ProviderPrefs>,
}

fn all_empty(providers: &BTreeMap<String, ProviderPrefs>) -> bool {
    providers.values().all(ProviderPrefs::is_empty)
}

fn serialize_non_empty<S: serde::Serializer>(
    providers: &BTreeMap<String, ProviderPrefs>,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    serializer.collect_map(providers.iter().filter(|(_, p)| !p.is_empty()))
}

impl Default for UsagePreferences {
    fn default() -> Self {
        Self {
            enabled: true,
            warnings: true,
            providers: BTreeMap::new(),
        }
    }
}

impl UsagePreferences {
    pub fn is_default(&self) -> bool {
        self.enabled && self.warnings && all_empty(&self.providers)
    }

    /// Drops empty provider entries, so they are not written.
    pub fn compacted(mut self) -> Self {
        self.providers.retain(|_, p| !p.is_empty());
        self
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ProviderPrefs {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub consent: Option<Consent>,
}

impl ProviderPrefs {
    pub fn is_empty(&self) -> bool {
        self.enabled.is_none() && self.color.is_none() && self.consent.is_none()
    }
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
            usage: UsagePreferences::default(),
            android_sdk_path: None,
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
            usage: self.usage.compacted(),
            android_sdk_path: self.android_sdk_path,
        }
    }
}

//! Agent usage meter (feature 006): plan limits per agent account, read through each agent's
//! own command. Sonata never reads credential stores itself (research R1, R2, R5).

pub mod cache;
pub mod claude;
pub mod codex;
pub mod consent;
pub mod custom;
pub mod detect;
pub mod runner;
pub mod schedule;
pub mod service;
pub mod system;
pub mod window;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// `claude`, `codex`, or a custom definition id.
pub type ProviderId = String;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ProviderKind {
    BuiltIn,
    Custom,
}

/// Whether the provider's command was found. `Unknown` when detection did not finish in time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Detection {
    Found,
    NotFound,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum WindowKind {
    Short,
    Weekly,
    Other,
}

/// One limit of an account: share used and when it resets.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Window {
    pub kind: WindowKind,
    pub label: String,
    pub used_percent: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resets_at: Option<DateTime<Utc>>,
}

/// Keeps windows with a finite, non-negative share whose period has not ended yet.
pub fn current_windows(windows: Vec<Window>, now: DateTime<Utc>) -> Vec<Window> {
    windows
        .into_iter()
        .filter(|w| w.used_percent.is_finite() && w.used_percent >= 0.0)
        .filter(|w| w.resets_at.is_none_or(|t| t > now))
        .collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ReadingStatus {
    Ok,
    NoPlanLimits,
    NotLoggedIn,
    NotInstalled,
    Unavailable,
    Off,
}

/// A snapshot of all windows of one account (data-model "Reading").
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Reading {
    pub provider_id: ProviderId,
    pub status: ReadingStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plan: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub account: Option<String>,
    /// Shown in the panel; never written to the cache or the log.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub account_email: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub windows: Vec<Window>,
    pub read_at: DateTime<Utc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

impl Reading {
    pub fn new(provider_id: impl Into<String>, status: ReadingStatus, now: DateTime<Utc>) -> Self {
        Self {
            provider_id: provider_id.into(),
            status,
            plan: None,
            account: None,
            account_email: None,
            windows: Vec::new(),
            read_at: now,
            message: None,
        }
    }

    pub fn with_message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
    }

    pub fn unavailable(
        provider_id: impl Into<String>,
        now: DateTime<Utc>,
        reason: impl Into<String>,
    ) -> Self {
        Self::new(provider_id, ReadingStatus::Unavailable, now).with_message(reason)
    }
}

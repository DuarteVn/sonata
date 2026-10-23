//! Limit windows, warning levels per window period, and the compact meter summary.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::WindowKind;

/// Highest warning already sent for a window period.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum WarningLevel {
    #[default]
    None,
    Warn80,
    Full100,
}

/// One window period: `(provider, window kind, reset time)` and the level sent in it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WarningEntry {
    pub provider_id: String,
    pub window_kind: WindowKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resets_at: Option<DateTime<Utc>>,
    pub sent: WarningLevel,
}

/// Compact meter state (FR-005).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SummaryState {
    Off,
    Empty,
    Normal,
    Warning,
    Full,
}

/// The window with the highest share used, for the Usage button.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageSummary {
    pub state: SummaryState,
    pub percent: Option<f64>,
    pub provider_id: Option<String>,
    pub window_kind: Option<WindowKind>,
    pub resets_at: Option<DateTime<Utc>>,
}

/// `readings`: the windows of each readable provider with a good (possibly stale) reading.
pub fn summary<'a>(
    enabled: bool,
    readings: impl IntoIterator<Item = (&'a str, &'a [super::Window])>,
) -> UsageSummary {
    let mut out = UsageSummary {
        state: if enabled {
            SummaryState::Empty
        } else {
            SummaryState::Off
        },
        percent: None,
        provider_id: None,
        window_kind: None,
        resets_at: None,
    };
    if !enabled {
        return out;
    }
    for (id, windows) in readings {
        for w in windows {
            if out.percent.is_none_or(|p| w.used_percent > p) {
                out.state = SummaryState::Normal;
                out.percent = Some(w.used_percent);
                out.provider_id = Some(id.to_string());
                out.window_kind = Some(w.kind);
                out.resets_at = w.resets_at;
            }
        }
    }
    out
}

impl WarningLevel {
    pub fn percent(self) -> u8 {
        match self {
            Self::None => 0,
            Self::Warn80 => 80,
            Self::Full100 => 100,
        }
    }
}

/// Records the levels reached by `windows` of `provider_id`; returns new crossings.
/// Filled in by T042 (US2).
pub fn observe(
    _entries: &mut Vec<WarningEntry>,
    _provider_id: &str,
    _windows: &[super::Window],
    _now: DateTime<Utc>,
) -> Vec<(super::Window, WarningLevel)> {
    Vec::new()
}

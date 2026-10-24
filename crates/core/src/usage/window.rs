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
                out.state = match level_for(w.used_percent) {
                    WarningLevel::Full100 => SummaryState::Full,
                    WarningLevel::Warn80 => SummaryState::Warning,
                    WarningLevel::None => SummaryState::Normal,
                };
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

/// Share at which a warning is sent and the meter turns amber (FR-010).
pub const WARNING_PERCENT: f64 = 80.0;

pub fn level_for(percent: f64) -> WarningLevel {
    if percent >= 100.0 {
        WarningLevel::Full100
    } else if percent >= WARNING_PERCENT {
        WarningLevel::Warn80
    } else {
        WarningLevel::None
    }
}

/// Reset times within this margin name the same period: some sources derive the reset time
/// from "seconds left", which moves a little between readings.
const PERIOD_SLACK_SECONDS: i64 = 120;

fn same_period(a: Option<DateTime<Utc>>, b: Option<DateTime<Utc>>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(a), Some(b)) => (a - b).num_seconds().abs() <= PERIOD_SLACK_SECONDS,
        _ => false,
    }
}

/// Records the levels reached by `windows` of `provider_id` and returns the windows that
/// crossed a level not yet sent in their period. Periods that ended are dropped.
pub fn observe(
    entries: &mut Vec<WarningEntry>,
    provider_id: &str,
    windows: &[super::Window],
    now: DateTime<Utc>,
) -> Vec<(super::Window, WarningLevel)> {
    entries.retain(|e| e.resets_at.is_none_or(|t| t > now));
    let mut crossed = Vec::new();
    for w in windows {
        let level = level_for(w.used_percent);
        let entry = entries.iter_mut().find(|e| {
            e.provider_id == provider_id
                && e.window_kind == w.kind
                && same_period(e.resets_at, w.resets_at)
        });
        let sent = entry.as_ref().map(|e| e.sent).unwrap_or_default();
        if level <= sent {
            continue;
        }
        match entry {
            Some(e) => e.sent = level,
            None => entries.push(WarningEntry {
                provider_id: provider_id.to_string(),
                window_kind: w.kind,
                resets_at: w.resets_at,
                sent: level,
            }),
        }
        crossed.push((w.clone(), level));
    }
    crossed
}

/// Kind and label for a window known only by its length in minutes.
pub fn kind_for_minutes(minutes: i64) -> (WindowKind, String) {
    const DAY: i64 = 24 * 60;
    if minutes <= DAY {
        let label = if minutes % 60 == 0 {
            format!("{}-hour", minutes / 60)
        } else {
            format!("{minutes}-minute")
        };
        (WindowKind::Short, label)
    } else if (6 * DAY..=8 * DAY).contains(&minutes) {
        (WindowKind::Weekly, "Weekly".into())
    } else {
        (WindowKind::Other, format!("{}-day", minutes / DAY))
    }
}

//! When each provider is due for a reading (data-model "Schedule", FR-006).

use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Claude probes cost one small request, so they run less often (research R1).
pub const CLAUDE_INTERVAL: Duration = Duration::from_secs(5 * 60);
pub const DEFAULT_INTERVAL: Duration = Duration::from_secs(60);
/// The panel reads again on open when the cached value is older than this.
pub const PANEL_FRESH: Duration = Duration::from_secs(60);
/// No automatic or panel-open read within this time of the previous attempt.
pub const FLOOR: Duration = Duration::from_secs(60);

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub enum RefreshReason {
    Auto,
    PanelOpen,
    Manual,
}

pub fn interval(provider: &str) -> Duration {
    if provider == "claude" {
        CLAUDE_INTERVAL
    } else {
        DEFAULT_INTERVAL
    }
}

#[derive(Debug, Clone, Default)]
pub struct Schedule {
    last_attempt: HashMap<String, Instant>,
}

impl Schedule {
    /// `cached_age`: age of the last good reading. `reset_passed`: a window of the provider
    /// reached its reset time.
    pub fn due(
        &self,
        provider: &str,
        reason: RefreshReason,
        now: Instant,
        cached_age: Option<Duration>,
        reset_passed: bool,
    ) -> bool {
        if reason == RefreshReason::Manual {
            return true;
        }
        let since = self
            .last_attempt
            .get(provider)
            .map(|last| now.saturating_duration_since(*last));
        if since.is_some_and(|s| s < FLOOR) {
            return false;
        }
        match reason {
            RefreshReason::PanelOpen => cached_age.is_none_or(|age| age > PANEL_FRESH),
            _ => reset_passed || since.is_none_or(|s| s >= interval(provider)),
        }
    }

    pub fn mark(&mut self, provider: &str, now: Instant) {
        self.last_attempt.insert(provider.to_string(), now);
    }
}

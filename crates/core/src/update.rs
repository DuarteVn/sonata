//! Update rules (feature 005): when to check for a new release, and the state the sidebar banner
//! shows. The shell does the network and install work and reports results here.

use std::time::{Duration, Instant};

use serde::Serialize;

/// Automatic checks run at launch and then once a day.
pub const CHECK_INTERVAL: Duration = Duration::from_secs(24 * 60 * 60);

/// Decides when an automatic check is due. Manual checks never touch it.
#[derive(Debug, Clone)]
pub struct UpdateSchedule {
    enabled: bool,
    last_check: Option<Instant>,
}

impl UpdateSchedule {
    pub fn new(enabled: bool) -> Self {
        Self {
            enabled,
            last_check: None,
        }
    }

    pub fn enabled(&self) -> bool {
        self.enabled
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    pub fn last_check(&self) -> Option<Instant> {
        self.last_check
    }

    pub fn due(&self, now: Instant) -> bool {
        self.enabled
            && self
                .last_check
                .is_none_or(|last| now.saturating_duration_since(last) >= CHECK_INTERVAL)
    }

    /// Records an automatic check, whether it succeeded or failed.
    pub fn mark_checked(&mut self, now: Instant) {
        self.last_check = Some(now);
    }
}

/// A newer release found by a check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Release {
    pub version: String,
    pub notes: String,
    pub date: Option<String>,
}

/// What the update banner shows (contracts/ipc.md `UpdateState`).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum UpdateState {
    #[default]
    Idle,
    Available {
        version: String,
        notes: String,
        date: Option<String>,
    },
    Dismissed {
        version: String,
    },
    Downloading {
        version: String,
        received: u64,
        total: Option<u64>,
    },
    ReadyToInstall {
        version: String,
    },
    Installing {
        version: String,
    },
    Failed {
        version: String,
        reason: String,
    },
    Blocked {
        version: String,
        reason: String,
    },
}

impl UpdateState {
    pub fn version(&self) -> Option<&str> {
        match self {
            Self::Idle => None,
            Self::Available { version, .. }
            | Self::Dismissed { version }
            | Self::Downloading { version, .. }
            | Self::ReadyToInstall { version }
            | Self::Installing { version }
            | Self::Failed { version, .. }
            | Self::Blocked { version, .. } => Some(version),
        }
    }

    /// True while the banner (or the dot on the show-sidebar button) is visible.
    pub fn pending(&self) -> bool {
        !matches!(self, Self::Idle | Self::Dismissed { .. })
    }

    fn busy(&self) -> bool {
        matches!(
            self,
            Self::Downloading { .. } | Self::ReadyToInstall { .. } | Self::Installing { .. }
        )
    }

    /// A check found a newer release. Ignored while a download or install is under way.
    pub fn found(&mut self, release: Release) {
        if self.busy() {
            return;
        }
        *self = Self::Available {
            version: release.version,
            notes: release.notes,
            date: release.date,
        };
    }

    /// "Later": hide the banner until the next check finds the release again.
    pub fn dismiss(&mut self) {
        if let Self::Available { version, .. } = self {
            *self = Self::Dismissed {
                version: std::mem::take(version),
            };
        }
    }

    /// Starts a download from `Available` or `Failed`; returns false otherwise.
    pub fn start_download(&mut self) -> bool {
        match self {
            Self::Available { version, .. } | Self::Failed { version, .. } => {
                *self = Self::Downloading {
                    version: std::mem::take(version),
                    received: 0,
                    total: None,
                };
                true
            }
            _ => false,
        }
    }

    pub fn progress(&mut self, now_received: u64, now_total: Option<u64>) {
        if let Self::Downloading {
            received, total, ..
        } = self
        {
            *received = now_received;
            *total = now_total;
        }
    }

    /// The archive downloaded and its signature verified.
    pub fn downloaded(&mut self) {
        if let Self::Downloading { version, .. } = self {
            *self = Self::ReadyToInstall {
                version: std::mem::take(version),
            };
        }
    }

    pub fn installing(&mut self) {
        if let Self::ReadyToInstall { version } = self {
            *self = Self::Installing {
                version: std::mem::take(version),
            };
        }
    }

    /// A download or install failed; the running app stays as it is.
    pub fn fail(&mut self, reason: String) {
        if let Some(version) = self.version().map(str::to_owned) {
            *self = Self::Failed { version, reason };
        }
    }

    /// The app cannot replace itself where it runs (disk image, translocated copy).
    pub fn block(&mut self, reason: String) {
        if let Some(version) = self.version().map(str::to_owned) {
            *self = Self::Blocked { version, reason };
        }
    }
}

//! Busy/idle change notifications for terminals with "Monitor activity" on (FR-013).

use std::time::Duration;

pub const ACTIVITY_INTERVAL: Duration = Duration::from_millis(500);

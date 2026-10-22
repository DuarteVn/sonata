//! Crash-loop guard (FR-027, research R9): after two consecutive crashes shortly after launch,
//! the app stops reopening the last active workspace automatically.

use std::time::Duration;

use crate::model::app_state::AppState;

/// A launch that stays alive this long is not an "early" crash.
pub const EARLY_CRASH_WINDOW: Duration = Duration::from_secs(30);
pub const TRIP_THRESHOLD: u32 = 2;

/// Updates the counters for a new launch. Returns true when the guard trips.
pub fn on_launch(state: &mut AppState) -> bool {
    if !state.clean_shutdown && !state.outlived_launch_window {
        state.consecutive_early_crashes += 1;
    } else {
        state.consecutive_early_crashes = 0;
    }
    state.clean_shutdown = false;
    state.outlived_launch_window = false;
    state.consecutive_early_crashes >= TRIP_THRESHOLD
}

/// Called once the app has run for [`EARLY_CRASH_WINDOW`].
pub fn on_outlived_launch_window(state: &mut AppState) {
    state.outlived_launch_window = true;
    state.consecutive_early_crashes = 0;
}

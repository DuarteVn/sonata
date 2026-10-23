//! Screen signatures and stillness (research R5).
//!
//! A signature is the screen scaled down to 64 pixels wide in gray. Two signatures differ when
//! at least 0.5% of their pixels changed clearly, so a blinking text cursor does not count as a
//! change but a new screen, a dialog, or a scroll does.

use std::time::{Duration, Instant};

const WIDTH: u32 = 64;
/// A pixel changed when its gray value moved by more than this.
const PIXEL_DELTA: u8 = 24;
/// Signatures differ when at least this share of pixels changed (0.5%).
const CHANGED_SHARE: f64 = 0.005;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Signature {
    width: u32,
    height: u32,
    gray: Vec<u8>,
}

impl Signature {
    /// Box-averages an RGBA frame down to 64 pixels wide, keeping its aspect ratio.
    pub fn from_rgba(width: u32, height: u32, rgba: &[u8]) -> Self {
        let w = WIDTH.min(width.max(1));
        let h =
            ((u64::from(height) * u64::from(w)).div_ceil(u64::from(width.max(1))) as u32).max(1);
        let mut gray = Vec::with_capacity((w * h) as usize);
        for sy in 0..h {
            let (y0, y1) = span(sy, h, height);
            for sx in 0..w {
                let (x0, x1) = span(sx, w, width);
                let (mut sum, mut n) = (0u64, 0u64);
                for y in y0..y1 {
                    for x in x0..x1 {
                        let i = ((y * width + x) * 4) as usize;
                        if let Some(p) = rgba.get(i..i + 3) {
                            // Integer luma approximation.
                            sum +=
                                (u64::from(p[0]) * 3 + u64::from(p[1]) * 6 + u64::from(p[2])) / 10;
                            n += 1;
                        }
                    }
                }
                gray.push(sum.checked_div(n).unwrap_or(0) as u8);
            }
        }
        Self {
            width: w,
            height: h,
            gray,
        }
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn differs(&self, other: &Signature) -> bool {
        if (self.width, self.height) != (other.width, other.height) {
            return true;
        }
        let changed = self
            .gray
            .iter()
            .zip(&other.gray)
            .filter(|(a, b)| a.abs_diff(**b) > PIXEL_DELTA)
            .count();
        changed as f64 >= self.gray.len() as f64 * CHANGED_SHARE
    }
}

/// Source pixel range `[start, end)` covered by target pixel `i` of `target` pixels.
fn span(i: u32, target: u32, source: u32) -> (u32, u32) {
    let start = (u64::from(i) * u64::from(source) / u64::from(target)) as u32;
    let end = ((u64::from(i + 1) * u64::from(source)).div_ceil(u64::from(target)) as u32)
        .clamp(start + 1, source.max(start + 1));
    (start, end)
}

/// Remembers the last clearly different screen and when it appeared.
#[derive(Debug, Clone)]
pub struct StillTracker {
    reference: Option<Signature>,
    last_change: Instant,
}

impl StillTracker {
    pub fn new(now: Instant) -> Self {
        Self {
            reference: None,
            last_change: now,
        }
    }

    pub fn push(&mut self, sig: Signature, now: Instant) {
        let changed = self.reference.as_ref().is_none_or(|r| r.differs(&sig));
        if changed {
            self.reference = Some(sig);
            self.last_change = now;
        }
    }

    /// How long the screen has been still, counting only from `since` (the agent's action).
    pub fn still_for(&self, since: Instant, now: Instant) -> Duration {
        now.saturating_duration_since(self.last_change.max(since))
    }

    /// The current screen, for element maps (research R4).
    pub fn signature(&self) -> Option<&Signature> {
        self.reference.as_ref()
    }
}

/// Polls `still_for` until it reaches `quiet` (true) or `limit` passes (false).
pub fn wait_until_still(
    quiet: Duration,
    limit: Duration,
    mut still_for: impl FnMut() -> Duration,
) -> bool {
    let start = Instant::now();
    loop {
        if still_for() >= quiet {
            return true;
        }
        if start.elapsed() >= limit {
            return false;
        }
        std::thread::sleep(Duration::from_millis(50).min(limit.saturating_sub(start.elapsed())));
    }
}

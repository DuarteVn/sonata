//! What agents do with a device: read, tap, type, swipe, keys, apps (US3, FR-011–FR-017).
//! Generic over [`DeviceDriver`], so it is tested with a fake device.

use std::time::{Duration, Instant};

use super::still::{Signature, wait_until_still};
use super::ui_tree::{Element, Screen, format_read, parse_dump};
use super::{DeviceDriver, Key};

/// The screen is still after this long without a real change (FR-014).
pub const STILL: Duration = Duration::from_millis(500);
/// Longest wait for a still screen.
pub const STILL_LIMIT: Duration = Duration::from_secs(5);
pub const DEFAULT_SWIPE: Duration = Duration::from_millis(300);
const TAP_HOLD: Duration = Duration::from_millis(50);
const MOVE_STEP: Duration = Duration::from_millis(16);

/// Numbers from the last `read` and the screen they belong to (data-model.md).
#[derive(Debug, Clone)]
pub struct ElementMap {
    pub elements: Vec<Element>,
    pub signature: Option<Signature>,
}

/// A place to tap: an element number from `read`, or coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    Element(usize),
    Point(i32, i32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

impl Direction {
    pub fn parse(s: &str) -> Option<Self> {
        Some(match s.to_lowercase().as_str() {
            "up" => Self::Up,
            "down" => Self::Down,
            "left" => Self::Left,
            "right" => Self::Right,
            _ => return None,
        })
    }
}

/// Reads the screen and returns the text for the agent plus the new element map.
pub fn read(d: &dyn DeviceDriver) -> Result<(String, ElementMap), String> {
    let mut screen: Screen = parse_dump(&d.ui_dump()?)?;
    if screen.size == (0, 0) {
        screen.size = d.screen_size().unwrap_or_default();
    }
    let map = ElementMap {
        elements: screen.elements.clone(),
        signature: d.signature(),
    };
    Ok((format_read(&screen), map))
}

/// The point to touch for a target; element numbers must come from a current `read`.
pub fn point(
    d: &dyn DeviceDriver,
    map: Option<&ElementMap>,
    target: Target,
) -> Result<(i32, i32), String> {
    match target {
        Target::Point(x, y) => Ok((x, y)),
        Target::Element(n) => {
            let map = map.ok_or("no element numbers yet; run read first")?;
            let e = map
                .elements
                .iter()
                .find(|e| e.number == n)
                .ok_or_else(|| format!("element {n} not found; run read again"))?;
            let changed = match (&map.signature, d.signature()) {
                (Some(then), Some(now)) => then.differs(&now),
                _ => false,
            };
            if changed {
                return Err("the screen changed; run read again".into());
            }
            Ok(e.center())
        }
    }
}

/// Waits until the screen is still after an action (FR-014). False when it kept changing.
pub fn settle(d: &dyn DeviceDriver, since: Instant) -> bool {
    wait_until_still(STILL, STILL_LIMIT, || d.still_for(since))
}

pub fn tap(d: &dyn DeviceDriver, (x, y): (i32, i32)) -> Result<(), String> {
    d.touch(x, y, true)?;
    std::thread::sleep(TAP_HOLD);
    d.touch(x, y, false)
}

/// Start and end of a direction swipe: from 70% to 30% of the screen on that axis, centered.
pub fn direction_points((w, h): (u32, u32), dir: Direction) -> ((i32, i32), (i32, i32)) {
    let (w, h) = (w as i32, h as i32);
    let (cx, cy) = (w / 2, h / 2);
    let (lo_x, hi_x) = (w * 3 / 10, w * 7 / 10);
    let (lo_y, hi_y) = (h * 3 / 10, h * 7 / 10);
    match dir {
        Direction::Up => ((cx, hi_y), (cx, lo_y)),
        Direction::Down => ((cx, lo_y), (cx, hi_y)),
        Direction::Left => ((hi_x, cy), (lo_x, cy)),
        Direction::Right => ((lo_x, cy), (hi_x, cy)),
    }
}

/// One finger from `from` to `to` over `duration`, moving every 16 ms.
pub fn swipe(
    d: &dyn DeviceDriver,
    from: (i32, i32),
    to: (i32, i32),
    duration: Duration,
) -> Result<(), String> {
    d.touch(from.0, from.1, true)?;
    let steps = (duration.as_millis() / MOVE_STEP.as_millis()).max(1) as i32;
    for i in 1..steps {
        std::thread::sleep(MOVE_STEP);
        let x = from.0 + (to.0 - from.0) * i / steps;
        let y = from.1 + (to.1 - from.1) * i / steps;
        d.touch(x, y, true)?;
    }
    std::thread::sleep(MOVE_STEP);
    d.touch(to.0, to.1, true)?;
    d.touch(to.0, to.1, false)
}

pub fn key(d: &dyn DeviceDriver, name: &str) -> Result<(), String> {
    let key = Key::parse(name).ok_or("keys: back, home, recents, enter, delete")?;
    d.key(&key)
}

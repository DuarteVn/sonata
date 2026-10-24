//! Screen signatures and "the screen is still" (research R5, FR-014).

use std::time::{Duration, Instant};

use sonata_core::device::still::{Signature, StillTracker, wait_until_still};

/// A 640×1280 RGBA frame filled with `v`, with an optional `n`×`n` square of white at (x, y).
fn frame(v: u8, square: Option<(u32, u32, u32)>) -> Signature {
    let (w, h) = (640u32, 1280u32);
    let mut rgba = vec![v; (w * h * 4) as usize];
    if let Some((x0, y0, n)) = square {
        for y in y0..y0 + n {
            for x in x0..x0 + n {
                let i = ((y * w + x) * 4) as usize;
                rgba[i..i + 3].copy_from_slice(&[255, 255, 255]);
            }
        }
    }
    Signature::from_rgba(w, h, &rgba)
}

#[test]
fn signature_is_small_and_compares_with_tolerance() {
    let a = frame(0, None);
    assert_eq!(a.width(), 64);
    assert_eq!(a.height(), 128);
    // A blinking cursor (a few device pixels) is not a change.
    assert!(!a.differs(&frame(0, Some((300, 600, 20)))));
    // A new screen is.
    assert!(a.differs(&frame(200, None)));
    // A dialog over part of the screen is.
    assert!(a.differs(&frame(0, Some((100, 300, 400)))));
}

#[test]
fn tracker_reports_time_since_last_real_change() {
    let t0 = Instant::now();
    let ms = |n| t0 + Duration::from_millis(n);
    let mut t = StillTracker::new(t0);
    t.push(frame(0, None), ms(0));
    assert_eq!(t.still_for(ms(0), ms(400)), Duration::from_millis(400));
    // Cursor blinks do not reset the clock.
    t.push(frame(0, Some((300, 600, 20))), ms(500));
    t.push(frame(0, None), ms(1000));
    assert_eq!(t.still_for(ms(0), ms(1200)), Duration::from_millis(1200));
    // A new screen does.
    t.push(frame(200, None), ms(1300));
    assert_eq!(t.still_for(ms(0), ms(1500)), Duration::from_millis(200));
    // Stillness is counted from the action, not from before it.
    assert_eq!(t.still_for(ms(1450), ms(1500)), Duration::from_millis(50));
    assert!(!t.signature().unwrap().differs(&frame(200, None)));
}

#[test]
fn waiting_returns_once_still_or_at_the_limit() {
    let start = Instant::now();
    let mut calls = 0;
    assert!(wait_until_still(
        Duration::from_millis(500),
        Duration::from_secs(5),
        || {
            calls += 1;
            Duration::from_millis(if calls < 3 { 100 } else { 600 })
        }
    ));
    assert!(start.elapsed() < Duration::from_secs(1));

    let start = Instant::now();
    assert!(!wait_until_still(
        Duration::from_millis(500),
        Duration::from_millis(300),
        || Duration::ZERO
    ));
    let took = start.elapsed();
    assert!(
        took >= Duration::from_millis(300) && took < Duration::from_secs(1),
        "{took:?}"
    );
}

//! A scripted device launcher and driver for core tests (feature 007).

use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use sonata_core::device::grpc::Frame;
use sonata_core::device::still::Signature;
use sonata_core::device::{DeviceDriver, DeviceLauncher, Key, Poll, Session, StreamHandle, Target};

/// Shared script and call log for every fake device.
#[derive(Default)]
pub struct FakeWorld {
    pub calls: Mutex<Vec<String>>,
    /// Polls before a started device is ready.
    pub boot_polls: AtomicUsize,
    /// AVD names already running outside Sonata (attached, not started).
    pub outside: Mutex<Vec<String>>,
    /// Make every running device exit on its next poll.
    pub crash: AtomicBool,
    pub dump: Mutex<String>,
    /// Bumped to simulate a screen change after `read`.
    pub screen: AtomicUsize,
    pub apps: Mutex<Vec<String>>,
    pub fail_start: Mutex<Option<String>>,
}

impl FakeWorld {
    pub fn log(&self, s: impl Into<String>) {
        self.calls.lock().unwrap().push(s.into());
    }
    pub fn calls(&self) -> Vec<String> {
        self.calls.lock().unwrap().clone()
    }
    pub fn take_calls(&self) -> Vec<String> {
        std::mem::take(&mut self.calls.lock().unwrap())
    }
}

pub struct FakeLauncher(pub Arc<FakeWorld>);

impl DeviceLauncher for FakeLauncher {
    fn start(&self, target: &Target) -> Result<Box<dyn Session>, String> {
        if let Some(err) = self.0.fail_start.lock().unwrap().clone() {
            return Err(err);
        }
        let outside = self.0.outside.lock().unwrap().contains(&target.device_id);
        self.0.log(if outside {
            format!("attach {}", target.device_id)
        } else {
            format!("start {}", target.device_id)
        });
        Ok(Box::new(FakeSession {
            world: self.0.clone(),
            id: target.device_id.clone(),
            polls: 0,
            outside,
            stopped: false,
        }))
    }
}

struct FakeSession {
    world: Arc<FakeWorld>,
    id: String,
    polls: usize,
    outside: bool,
    stopped: bool,
}

impl Session for FakeSession {
    fn poll(&mut self) -> Poll {
        if self.stopped || self.world.crash.load(Ordering::SeqCst) {
            return Poll::Exited(None);
        }
        self.polls += 1;
        if !self.outside && self.polls <= self.world.boot_polls.load(Ordering::SeqCst) {
            return Poll::Booting;
        }
        Poll::Ready(Arc::new(FakeDriver(self.world.clone())))
    }
    fn stop(&mut self) {
        self.world.log(format!("stop {}", self.id));
        self.stopped = true;
    }
    fn started_by_sonata(&self) -> bool {
        !self.outside
    }
}

pub struct FakeDriver(pub Arc<FakeWorld>);

impl FakeDriver {
    fn ok(&self, s: String) -> Result<(), String> {
        self.0.log(s);
        Ok(())
    }
}

impl DeviceDriver for FakeDriver {
    fn screen_size(&self) -> Result<(u32, u32), String> {
        Ok((1080, 2400))
    }
    fn touch(&self, x: i32, y: i32, down: bool) -> Result<(), String> {
        self.ok(format!(
            "touch {x} {y} {}",
            if down { "down" } else { "up" }
        ))
    }
    fn wheel(&self, x: i32, y: i32, dx: i32, dy: i32) -> Result<(), String> {
        self.ok(format!("wheel {x} {y} {dx} {dy}"))
    }
    fn text(&self, text: &str) -> Result<(), String> {
        self.ok(format!("text {text}"))
    }
    fn key(&self, key: &Key) -> Result<(), String> {
        self.ok(format!("key {key:?}"))
    }
    fn ui_dump(&self) -> Result<String, String> {
        self.0.log("dump");
        Ok(self.0.dump.lock().unwrap().clone())
    }
    fn screenshot_png(&self) -> Result<Vec<u8>, String> {
        self.0.log("screenshot");
        Ok(b"\x89PNG fake".to_vec())
    }
    fn apps(&self) -> Result<Vec<String>, String> {
        Ok(self.0.apps.lock().unwrap().clone())
    }
    fn launch(&self, app: &str) -> Result<(), String> {
        if self.0.apps.lock().unwrap().iter().any(|a| a == app) {
            self.ok(format!("launch {app}"))
        } else {
            Err("not installed or not launchable".into())
        }
    }
    fn install(&self, file: &Path) -> Result<String, String> {
        self.0.log(format!("install {}", file.display()));
        Ok("com.example.app".into())
    }
    fn still_for(&self, since: Instant) -> Duration {
        Instant::now().saturating_duration_since(since) + Duration::from_millis(500)
    }
    fn signature(&self) -> Option<Signature> {
        let v = (self.0.screen.load(Ordering::SeqCst) * 80 % 256) as u8;
        Some(Signature::from_rgba(4, 8, &[v; 4 * 8 * 4]))
    }
    fn stream(
        &self,
        width: u32,
        height: u32,
        mut on_frame: Box<dyn FnMut(Frame) + Send>,
    ) -> Result<StreamHandle, String> {
        self.0.log(format!("stream {width}x{height}"));
        on_frame(Frame {
            width,
            height,
            seq: 1,
            rgba: vec![0; (width * height * 4) as usize],
        });
        Ok(Box::new(()))
    }
}

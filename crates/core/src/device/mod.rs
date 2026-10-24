//! Device nodes (feature 007): Android emulators and iOS simulators on the canvas. Core owns
//! detection, the emulator processes, adb and gRPC calls, and agent commands; the shell only
//! forwards frames and intents (research R12).

pub mod adb;
pub mod android;
pub mod avd;
pub mod control;
pub mod emulator;
pub mod grpc;
pub mod ios;
pub mod live;
pub mod process;
pub mod still;
pub mod tools;
pub mod ui_tree;

use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use serde::Serialize;
use uuid::Uuid;

use crate::error::{AppError, AppResult, ErrorCode};
use crate::model::node::Platform;
pub use grpc::Frame;
use still::Signature;

/// Default node frame: a phone screen plus the node's chrome (data-model.md).
pub const DEFAULT_SIZE: (f64, f64) = (380.0, 820.0);
/// How often running devices are checked for readiness and exit.
pub const TICK: Duration = Duration::from_millis(250);
/// Longest wait for `sonata device boot` and `install` (FR-017, AS9).
pub const BOOT_TIMEOUT: Duration = Duration::from_secs(180);

/// `3m`, `45s`, `300ms` for messages.
pub fn human(d: Duration) -> String {
    let ms = d.as_millis();
    if ms.is_multiple_of(60_000) {
        format!("{}m", ms / 60_000)
    } else if ms.is_multiple_of(1000) {
        format!("{}s", ms / 1000)
    } else {
        format!("{ms}ms")
    }
}

/// Short id printed by `sonata list` for devices (`d-xxxx`).
pub fn short_id(id: Uuid) -> String {
    format!("d-{}", &id.simple().to_string()[..4])
}

/// Which device a node shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Target {
    pub platform: Platform,
    pub device_id: String,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", tag = "kind", content = "message")]
pub enum DeviceState {
    Off,
    Booting,
    Ready,
    Stopped,
    Error(String),
}

impl DeviceState {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Booting => "booting",
            Self::Ready => "ready",
            Self::Stopped => "stopped",
            Self::Error(_) => "error",
        }
    }
}

/// Keys agents and node buttons can send.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Key {
    Back,
    Home,
    Recents,
    Enter,
    Delete,
    /// A W3C key name from the keyboard (live input only).
    Named(String),
}

impl Key {
    /// `back`, `home`, `recents`, `enter`, `delete` (contracts/cli.md).
    pub fn parse(name: &str) -> Option<Self> {
        Some(match name.trim().to_lowercase().as_str() {
            "back" => Self::Back,
            "home" => Self::Home,
            "recents" | "recent" | "overview" => Self::Recents,
            "enter" | "return" => Self::Enter,
            "delete" | "backspace" => Self::Delete,
            _ => return None,
        })
    }
}

/// Keeps a frame stream open; dropping it ends the stream.
pub type StreamHandle = Box<dyn Send>;

/// Controls of one running device. Calls block and are made from request threads.
pub trait DeviceDriver: Send + Sync {
    fn screen_size(&self) -> Result<(u32, u32), String>;
    /// One finger at device pixels; `down` false releases it.
    fn touch(&self, x: i32, y: i32, down: bool) -> Result<(), String>;
    /// Wheel at a device position; 120 units are one click.
    fn wheel(&self, x: i32, y: i32, dx: i32, dy: i32) -> Result<(), String>;
    fn text(&self, text: &str) -> Result<(), String>;
    fn key(&self, key: &Key) -> Result<(), String>;
    /// The raw accessibility hierarchy (uiautomator XML on Android).
    fn ui_dump(&self) -> Result<String, String>;
    fn screenshot_png(&self) -> Result<Vec<u8>, String>;
    /// Ids of installed apps that can be launched, sorted.
    fn apps(&self) -> Result<Vec<String>, String>;
    fn launch(&self, app: &str) -> Result<(), String>;
    /// Installs a package and returns the installed app id.
    fn install(&self, file: &Path) -> Result<String, String>;
    /// How long the screen has been still, counted from `since` (research R5).
    fn still_for(&self, since: Instant) -> Duration;
    /// The current screen signature, for element maps.
    fn signature(&self) -> Option<Signature>;
    /// Streams frames scaled to `width`×`height` until the handle is dropped.
    fn stream(
        &self,
        width: u32,
        height: u32,
        on_frame: Box<dyn FnMut(Frame) + Send>,
    ) -> Result<StreamHandle, String>;
    /// True when only slow fallbacks are available (research R7).
    fn reduced(&self) -> bool {
        false
    }
}

pub enum Poll {
    Booting,
    Ready(Arc<dyn DeviceDriver>),
    /// The device process ended; the text is its last error output, if any.
    Exited(Option<String>),
}

/// One device Sonata started or attached to.
pub trait Session: Send {
    fn poll(&mut self) -> Poll;
    /// Stops the device. Called only for devices Sonata started.
    fn stop(&mut self);
    fn started_by_sonata(&self) -> bool;
}

/// Starts devices, or attaches to ones already running (research R1, R7).
pub trait DeviceLauncher: Send + Sync {
    fn start(&self, target: &Target) -> Result<Box<dyn Session>, String>;
}

fn next_generation() -> u64 {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
    NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

/// Sends each start to the launcher of the target's platform.
pub struct PlatformLauncher {
    pub android: Box<dyn DeviceLauncher>,
    pub ios: Box<dyn DeviceLauncher>,
}

impl DeviceLauncher for PlatformLauncher {
    fn start(&self, target: &Target) -> Result<Box<dyn Session>, String> {
        match target.platform {
            Platform::Android => self.android.start(target),
            Platform::Ios => self.ios.start(target),
        }
    }
}

/// `device://state` (data-model.md).
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceStateEvent {
    pub node_id: Uuid,
    pub state: DeviceState,
    pub started_by_sonata: bool,
    pub reduced: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub screen: Option<(u32, u32)>,
}

/// A running device listed in the quit confirmation (FR-008).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunningDevice {
    pub node_id: Uuid,
    pub name: String,
    pub started_by_sonata: bool,
}

struct Runtime {
    target: Target,
    /// Bumped on each boot and stop, so a poll that ran meanwhile is discarded.
    generation: u64,
    state: DeviceState,
    session: Option<Box<dyn Session>>,
    driver: Option<Arc<dyn DeviceDriver>>,
    started_by_sonata: bool,
    screen: Option<(u32, u32)>,
}

impl Runtime {
    fn event(&self, node_id: Uuid) -> DeviceStateEvent {
        DeviceStateEvent {
            node_id,
            state: self.state.clone(),
            started_by_sonata: self.started_by_sonata,
            reduced: self.driver.as_ref().is_some_and(|d| d.reduced()),
            screen: self.screen,
        }
    }

    fn active(&self) -> bool {
        matches!(self.state, DeviceState::Booting | DeviceState::Ready)
    }
}

/// Every device runtime, keyed by node. In memory only.
#[derive(Default)]
pub struct DeviceManager {
    launcher: Mutex<Option<Arc<dyn DeviceLauncher>>>,
    runtimes: Mutex<HashMap<Uuid, Runtime>>,
    streams: Mutex<HashMap<Uuid, live::Live>>,
    maps: Mutex<HashMap<Uuid, control::ElementMap>>,
    /// Last frame seen per node, kept after its stream ends for the card image.
    last_frames: Mutex<HashMap<Uuid, live::LastFrame>>,
}

impl DeviceManager {
    fn runtimes(&self) -> MutexGuard<'_, HashMap<Uuid, Runtime>> {
        self.runtimes.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn set_launcher(&self, launcher: Arc<dyn DeviceLauncher>) {
        *self.launcher.lock().unwrap_or_else(|e| e.into_inner()) = Some(launcher);
    }

    fn launcher(&self) -> Option<Arc<dyn DeviceLauncher>> {
        self.launcher
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    pub fn state(&self, node: Uuid) -> DeviceStateEvent {
        self.runtimes()
            .get(&node)
            .map(|r| r.event(node))
            .unwrap_or(DeviceStateEvent {
                node_id: node,
                state: DeviceState::Off,
                started_by_sonata: false,
                reduced: false,
                screen: None,
            })
    }

    /// Starts (or attaches to) the node's device. Returns the new state to emit, if it changed.
    pub fn boot(&self, node: Uuid, target: Target) -> AppResult<Option<DeviceStateEvent>> {
        {
            let rt = self.runtimes();
            if rt.get(&node).is_some_and(Runtime::active) {
                return Ok(None);
            }
            if rt
                .iter()
                .any(|(id, r)| *id != node && r.active() && r.target.device_id == target.device_id)
            {
                return Err(AppError::new(
                    ErrorCode::DeviceInUse,
                    "Another device node is already running this device.",
                ));
            }
        }
        let launcher = self
            .launcher()
            .ok_or_else(|| AppError::internal("Devices are not available yet."))?;
        let started = launcher.start(&target);
        let mut rt = self.runtimes();
        let (runtime, result) = match started {
            Ok(session) => (
                Runtime {
                    started_by_sonata: session.started_by_sonata(),
                    generation: next_generation(),
                    target,
                    state: DeviceState::Booting,
                    session: Some(session),
                    driver: None,
                    screen: None,
                },
                Ok(()),
            ),
            Err(msg) => (
                Runtime {
                    target,
                    generation: next_generation(),
                    state: DeviceState::Error(msg.clone()),
                    session: None,
                    driver: None,
                    started_by_sonata: false,
                    screen: None,
                },
                Err(AppError::new(ErrorCode::DeviceNotRunning, msg)),
            ),
        };
        let event = runtime.event(node);
        rt.insert(node, runtime);
        result.map(|()| Some(event))
    }

    /// Polls every session once; returns the state changes to emit. Sessions are polled with
    /// no lock held (a poll may run adb), then their results applied if nothing changed meanwhile.
    pub fn tick(&self) -> Vec<DeviceStateEvent> {
        let taken: Vec<(Uuid, u64, Box<dyn Session>)> = self
            .runtimes()
            .iter_mut()
            .filter_map(|(id, r)| Some((*id, r.generation, r.session.take()?)))
            .collect();
        let mut changed = Vec::new();
        for (id, generation, mut session) in taken {
            let poll = session.poll();
            let mut rt = self.runtimes();
            let Some(r) = rt.get_mut(&id).filter(|r| r.generation == generation) else {
                // Stopped, rebooted, or deleted while polling: this session is no longer wanted.
                drop(rt);
                if session.started_by_sonata() {
                    session.stop();
                }
                continue;
            };
            let before = r.state.clone();
            match poll {
                Poll::Booting => r.session = Some(session),
                Poll::Ready(driver) => {
                    if r.driver.is_none() {
                        r.screen = driver.screen_size().ok();
                        r.driver = Some(driver);
                    }
                    r.state = DeviceState::Ready;
                    r.session = Some(session);
                }
                Poll::Exited(msg) => {
                    r.driver = None;
                    r.state = match msg {
                        Some(m) if before == DeviceState::Booting => DeviceState::Error(m),
                        _ => DeviceState::Stopped,
                    };
                }
            }
            if r.state != before {
                changed.push(r.event(id));
            }
        }
        changed
    }

    /// Opens (or reopens at a new size) the node's live stream.
    pub fn stream(
        &self,
        node: Uuid,
        width: u32,
        height: u32,
        send: live::FrameSink,
    ) -> AppResult<()> {
        let driver = self.driver(node)?;
        let last = self
            .last_frames
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .entry(node)
            .or_default()
            .clone();
        // Close the old stream first so two never run for one node.
        self.streams
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(&node);
        let live = live::Live::start(send, last, |on_frame| {
            driver.stream(width.max(1), height.max(1), on_frame)
        })
        .map_err(|e| AppError::new(ErrorCode::DeviceNotRunning, e))?;
        self.streams
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(node, live);
        Ok(())
    }

    pub fn frame_ack(&self, node: Uuid) {
        if let Some(l) = self
            .streams
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(&node)
        {
            l.ack();
        }
    }

    /// Ends the live stream; returns the last frame for the card image.
    pub fn unstream(&self, node: Uuid) -> Option<Arc<Frame>> {
        let live = self
            .streams
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(&node);
        live.and_then(|l| l.last())
    }

    /// Element numbers from the node's last `read` (research R4).
    pub fn element_map(&self, node: Uuid) -> Option<control::ElementMap> {
        self.maps
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(&node)
            .cloned()
    }

    pub fn set_element_map(&self, node: Uuid, map: control::ElementMap) {
        self.maps
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(node, map);
    }

    /// Stops the device if Sonata started it; the node goes back to off.
    pub fn stop(&self, node: Uuid) -> Option<DeviceStateEvent> {
        self.streams
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(&node);
        self.maps
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(&node);
        let mut rt = self.runtimes();
        let r = rt.get_mut(&node)?;
        r.generation = next_generation();
        if let Some(mut s) = r.session.take()
            && r.started_by_sonata
        {
            s.stop();
        }
        r.driver = None;
        r.state = DeviceState::Off;
        Some(r.event(node))
    }

    /// The node was deleted (FR-008).
    pub fn forget(&self, node: Uuid) {
        self.stop(node);
        self.runtimes().remove(&node);
        self.last_frames
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(&node);
    }

    /// Quit: stops every device Sonata started (FR-008).
    pub fn stop_all(&self) {
        let ids: Vec<Uuid> = self.runtimes().keys().copied().collect();
        for id in ids {
            self.forget(id);
        }
    }

    pub fn running(&self) -> Vec<RunningDevice> {
        let mut out: Vec<RunningDevice> = self
            .runtimes()
            .iter()
            .filter(|(_, r)| r.active())
            .map(|(id, r)| RunningDevice {
                node_id: *id,
                name: r.target.name.clone(),
                started_by_sonata: r.started_by_sonata,
            })
            .collect();
        out.sort_by(|a, b| a.name.cmp(&b.name));
        out
    }

    /// The controls of a ready device, or "device not running" / "device is booting".
    pub fn driver(&self, node: Uuid) -> AppResult<Arc<dyn DeviceDriver>> {
        let rt = self.runtimes();
        let r = rt.get(&node);
        match r.map(|r| (&r.state, &r.driver)) {
            Some((DeviceState::Ready, Some(d))) => Ok(d.clone()),
            Some((DeviceState::Booting, _)) => Err(AppError::new(
                ErrorCode::DeviceNotRunning,
                "device is booting",
            )),
            _ => Err(AppError::new(
                ErrorCode::DeviceNotRunning,
                "device not running",
            )),
        }
    }
}

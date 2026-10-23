//! Device nodes (feature 007): Android emulators and iOS simulators on the canvas. Core owns
//! detection, the emulator processes, adb and gRPC calls, and agent commands; the shell only
//! forwards frames and intents (research R12).

pub mod adb;
pub mod android;
pub mod avd;
pub mod emulator;
pub mod grpc;
pub mod process;
pub mod still;
pub mod tools;

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

    /// Polls every session once; returns the state changes to emit.
    pub fn tick(&self) -> Vec<DeviceStateEvent> {
        let mut changed = Vec::new();
        for (id, r) in self.runtimes().iter_mut() {
            let Some(session) = r.session.as_mut() else {
                continue;
            };
            let before = r.state.clone();
            match session.poll() {
                Poll::Booting => {}
                Poll::Ready(driver) => {
                    if r.driver.is_none() {
                        r.screen = driver.screen_size().ok();
                        r.driver = Some(driver);
                    }
                    r.state = DeviceState::Ready;
                }
                Poll::Exited(msg) => {
                    r.session = None;
                    r.driver = None;
                    r.state = match msg {
                        Some(m) if before == DeviceState::Booting => DeviceState::Error(m),
                        _ => DeviceState::Stopped,
                    };
                }
            }
            if r.state != before {
                changed.push(r.event(*id));
            }
        }
        changed
    }

    /// Stops the device if Sonata started it; the node goes back to off.
    pub fn stop(&self, node: Uuid) -> Option<DeviceStateEvent> {
        let mut rt = self.runtimes();
        let r = rt.get_mut(&node)?;
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

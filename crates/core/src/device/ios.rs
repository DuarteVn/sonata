//! iOS simulators through Xcode's `simctl` (research R11, decided 2026-10-23): list, boot, stop,
//! screenshot, apps, launch, install. There is no public way to send taps, keys, or read the
//! screen tree without third-party tools, so those calls are refused with no effect.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use serde::Serialize;

use super::android::{decode_png, scale_rgba};
use super::grpc::Frame;
use super::process::Processes;
use super::still::Signature;
use super::{BOOT_TIMEOUT, DeviceDriver, DeviceLauncher, Key, Poll, Session, StreamHandle, Target};

const UNSUPPORTED: &str = "not supported on iOS simulator";
const SIMCTL_TIMEOUT: Duration = Duration::from_secs(30);
const INSTALL_TIMEOUT: Duration = BOOT_TIMEOUT;
/// Live view is a screenshot poll (2–4 fps), below FR-004; accepted for P3.
const POLL_INTERVAL: Duration = Duration::from_millis(300);

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Simulator {
    pub udid: String,
    pub name: String,
    pub os_version: String,
    pub booted: bool,
}

/// `com.apple.CoreSimulator.SimRuntime.iOS-18-2` → `iOS 18.2`; other platforms → `None`.
fn ios_version(runtime: &str) -> Option<String> {
    let v = runtime.rsplit('.').next()?.strip_prefix("iOS-")?;
    Some(format!("iOS {}", v.replace('-', ".")))
}

/// `simctl list devices available --json`: available iOS simulators, booted first, then by name.
pub fn parse_simulators(json: &str) -> Result<Vec<Simulator>, String> {
    let v: serde_json::Value =
        serde_json::from_str(json).map_err(|e| format!("bad simulator list: {e}"))?;
    let mut out = Vec::new();
    for (runtime, devices) in v["devices"].as_object().into_iter().flatten() {
        let Some(os_version) = ios_version(runtime) else {
            continue;
        };
        for d in devices.as_array().into_iter().flatten() {
            if d["isAvailable"].as_bool() == Some(false) {
                continue;
            }
            let (Some(udid), Some(name)) = (d["udid"].as_str(), d["name"].as_str()) else {
                continue;
            };
            out.push(Simulator {
                udid: udid.to_string(),
                name: name.to_string(),
                os_version: os_version.clone(),
                booted: d["state"].as_str() == Some("Booted"),
            });
        }
    }
    out.sort_by(|a, b| b.booted.cmp(&a.booted).then(a.name.cmp(&b.name)));
    Ok(out)
}

/// `simctl listapps`: `CFBundleIdentifier = "…";` lines, sorted.
pub fn parse_listapps(text: &str) -> Vec<String> {
    let mut apps: Vec<String> = text
        .lines()
        .filter_map(|l| l.trim().strip_prefix("CFBundleIdentifier = "))
        .map(|v| v.trim_end_matches(';').trim_matches('"').to_string())
        .filter(|v| !v.is_empty())
        .collect();
    apps.sort();
    apps.dedup();
    apps
}

type DevFinder = Box<dyn Fn() -> Result<PathBuf, String> + Send + Sync>;

pub struct IosLauncher {
    developer_dir: DevFinder,
    procs: Arc<dyn Processes>,
}

#[derive(Clone)]
struct Simctl {
    procs: Arc<dyn Processes>,
    path: PathBuf,
}

impl Simctl {
    fn run(&self, args: &[&str], timeout: Duration) -> Result<String, String> {
        let out = self.procs.run(&self.path, args, timeout)?;
        if out.success {
            Ok(out.text())
        } else {
            Err(out
                .stderr
                .lines()
                .map(str::trim)
                .find(|l| !l.is_empty())
                .unwrap_or("simctl failed")
                .to_string())
        }
    }

    fn list(&self) -> Result<Vec<Simulator>, String> {
        parse_simulators(&self.run(&["list", "devices", "available", "--json"], SIMCTL_TIMEOUT)?)
    }
}

/// Simulators of the Xcode in `developer_dir` (runs `simctl`; Xcode is present, so this can
/// never trigger an install prompt).
pub fn list_simulators(procs: Arc<dyn Processes>, developer_dir: &Path) -> Vec<Simulator> {
    Simctl {
        procs,
        path: developer_dir.join("usr/bin/simctl"),
    }
    .list()
    .unwrap_or_default()
}

impl IosLauncher {
    pub fn new(developer_dir: DevFinder, procs: Arc<dyn Processes>) -> Self {
        Self {
            developer_dir,
            procs,
        }
    }
}

impl DeviceLauncher for IosLauncher {
    fn start(&self, target: &Target) -> Result<Box<dyn Session>, String> {
        let simctl = Simctl {
            procs: self.procs.clone(),
            path: (self.developer_dir)()?.join("usr/bin/simctl"),
        };
        let sim = simctl
            .list()?
            .into_iter()
            .find(|s| s.udid == target.device_id)
            .ok_or("This simulator is no longer available.")?;
        let owned = !sim.booted;
        if owned {
            simctl.run(&["boot", &sim.udid], SIMCTL_TIMEOUT)?;
        }
        Ok(Box::new(IosSession {
            simctl,
            udid: sim.udid,
            owned,
            started: Instant::now(),
            driver: None,
        }))
    }
}

struct IosSession {
    simctl: Simctl,
    udid: String,
    owned: bool,
    started: Instant,
    driver: Option<Arc<IosDriver>>,
}

impl Session for IosSession {
    fn poll(&mut self) -> Poll {
        let booted = match self.simctl.list() {
            Ok(list) => list.iter().any(|s| s.udid == self.udid && s.booted),
            Err(_) => return Poll::Booting,
        };
        match (&self.driver, booted) {
            (Some(_), false) => Poll::Exited(None),
            (Some(d), true) => Poll::Ready(d.clone()),
            (None, true) => {
                let d = Arc::new(IosDriver {
                    simctl: self.simctl.clone(),
                    udid: self.udid.clone(),
                });
                self.driver = Some(d.clone());
                Poll::Ready(d)
            }
            (None, false) if self.started.elapsed() > BOOT_TIMEOUT => {
                self.stop();
                Poll::Exited(Some("The simulator did not finish booting.".into()))
            }
            (None, false) => Poll::Booting,
        }
    }

    fn stop(&mut self) {
        self.driver = None;
        if self.owned {
            let _ = self.simctl.run(&["shutdown", &self.udid], SIMCTL_TIMEOUT);
        }
    }

    fn started_by_sonata(&self) -> bool {
        self.owned
    }
}

pub struct IosDriver {
    simctl: Simctl,
    udid: String,
}

impl IosDriver {
    fn shot(simctl: &Simctl, udid: &str) -> Result<Vec<u8>, String> {
        let tmp = tempfile::Builder::new()
            .suffix(".png")
            .tempfile()
            .map_err(|e| e.to_string())?;
        let path = tmp.path().to_str().ok_or("bad temp path")?.to_string();
        let file = path.as_str();
        simctl.run(
            &["io", udid, "screenshot", "--type=png", file],
            SIMCTL_TIMEOUT,
        )?;
        std::fs::read(tmp.path()).map_err(|e| e.to_string())
    }
}

impl DeviceDriver for IosDriver {
    fn screen_size(&self) -> Result<(u32, u32), String> {
        let png = self.screenshot_png()?;
        decode_png(&png)
            .map(|(w, h, _)| (w, h))
            .ok_or_else(|| "could not read the screen size".into())
    }
    fn touch(&self, _: i32, _: i32, _: bool) -> Result<(), String> {
        Err(UNSUPPORTED.into())
    }
    fn wheel(&self, _: i32, _: i32, _: i32, _: i32) -> Result<(), String> {
        Err(UNSUPPORTED.into())
    }
    fn text(&self, _: &str) -> Result<(), String> {
        Err(UNSUPPORTED.into())
    }
    fn key(&self, _: &Key) -> Result<(), String> {
        Err(UNSUPPORTED.into())
    }
    fn ui_dump(&self) -> Result<String, String> {
        Err(UNSUPPORTED.into())
    }
    fn screenshot_png(&self) -> Result<Vec<u8>, String> {
        Self::shot(&self.simctl, &self.udid)
    }
    fn apps(&self) -> Result<Vec<String>, String> {
        Ok(parse_listapps(
            &self.simctl.run(&["listapps", &self.udid], SIMCTL_TIMEOUT)?,
        ))
    }
    fn launch(&self, app: &str) -> Result<(), String> {
        if !app
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || ".-_".contains(c))
        {
            return Err("not installed or not launchable".into());
        }
        self.simctl
            .run(&["launch", &self.udid, app], SIMCTL_TIMEOUT)
            .map(|_| ())
            .map_err(|_| "not installed or not launchable".into())
    }
    fn install(&self, file: &Path) -> Result<String, String> {
        let path = file.to_str().ok_or("the file path is not valid text")?;
        self.simctl
            .run(&["install", &self.udid, path], INSTALL_TIMEOUT)?;
        let plist = file.join("Info.plist");
        let plist = plist.to_str().unwrap_or_default();
        let id = self
            .simctl
            .procs
            .run(
                Path::new("/usr/bin/plutil"),
                &["-extract", "CFBundleIdentifier", "raw", "-o", "-", plist],
                SIMCTL_TIMEOUT,
            )
            .map(|o| o.text().trim().to_string())
            .unwrap_or_default();
        Ok(if id.is_empty() {
            "installed".into()
        } else {
            id
        })
    }
    fn still_for(&self, since: Instant) -> Duration {
        Instant::now()
            .saturating_duration_since(since)
            .saturating_sub(Duration::from_millis(500))
    }
    fn signature(&self) -> Option<Signature> {
        None
    }
    fn stream(
        &self,
        width: u32,
        height: u32,
        mut on_frame: Box<dyn FnMut(Frame) + Send>,
    ) -> Result<StreamHandle, String> {
        let stop = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let flag = stop.clone();
        let (simctl, udid) = (self.simctl.clone(), self.udid.clone());
        std::thread::spawn(move || {
            let mut seq = 0;
            while !flag.load(std::sync::atomic::Ordering::SeqCst) {
                if let Some((w, h, rgba)) =
                    Self::shot(&simctl, &udid).ok().and_then(|p| decode_png(&p))
                {
                    seq += 1;
                    // Keep the aspect ratio inside the requested box.
                    let scale = (f64::from(width) / f64::from(w))
                        .min(f64::from(height) / f64::from(h))
                        .min(1.0);
                    let (tw, th) = (
                        ((f64::from(w) * scale) as u32).max(1),
                        ((f64::from(h) * scale) as u32).max(1),
                    );
                    on_frame(Frame {
                        width: tw,
                        height: th,
                        seq,
                        rgba: scale_rgba(&rgba, w, h, tw, th),
                    });
                }
                std::thread::sleep(POLL_INTERVAL);
            }
        });
        Ok(Box::new(super::android::StopOnDrop::new(stop)))
    }
    /// View only: no input reaches an iOS simulator.
    fn reduced(&self) -> bool {
        true
    }
}

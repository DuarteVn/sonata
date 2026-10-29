//! Starting, attaching to, and stopping Android emulators (research R1, R7).

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use super::android::{AndroidDriver, Grpc};
use super::avd::parse_ini;
use super::process::{Child, Processes};
use super::{BOOT_TIMEOUT, DeviceLauncher, Poll, Session, Target};
use crate::model::node::Platform;

const CONSOLE_PORTS: std::ops::RangeInclusive<u16> = 5554..=5682;
const FIRST_GRPC_PORT: u16 = 8554;
const ADB_TIMEOUT: Duration = Duration::from_secs(5);
/// After `emu kill`, the emulator is killed if it is still running after this.
const STOP_GRACE: Duration = Duration::from_secs(20);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sdk {
    pub emulator: PathBuf,
    pub adb: PathBuf,
}

type SdkFinder = Box<dyn Fn() -> Result<Sdk, String> + Send + Sync>;

pub struct AndroidLauncher {
    sdk: SdkFinder,
    procs: Arc<dyn Processes>,
    /// Folders holding the emulator's `pid_<pid>.ini` discovery files.
    discovery_dirs: Vec<PathBuf>,
}

/// One running emulator as described by its discovery file.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Instance {
    pid: u32,
    avd: String,
    console: Option<u16>,
    grpc_port: Option<u16>,
    token: Option<String>,
    avd_dir: Option<PathBuf>,
}

impl Instance {
    /// `hw.keyboard=yes` in the AVD's `config.ini`; unknown counts as no (adb always works).
    fn hw_keyboard(&self) -> bool {
        self.avd_dir
            .as_ref()
            .and_then(|d| std::fs::read_to_string(d.join("config.ini")).ok())
            .is_some_and(|t| parse_ini(&t).get("hw.keyboard").is_some_and(|v| v == "yes"))
    }
}

fn read_instance(path: &Path) -> Option<Instance> {
    let pid = path
        .file_name()?
        .to_str()?
        .strip_prefix("pid_")?
        .strip_suffix(".ini")?
        .parse()
        .ok()?;
    let ini = parse_ini(&std::fs::read_to_string(path).ok()?);
    let port = |k: &str| ini.get(k).and_then(|v| v.parse::<u16>().ok());
    Some(Instance {
        pid,
        avd: ini
            .get("avd.name")
            .or_else(|| ini.get("avd.id"))
            .cloned()
            .unwrap_or_default(),
        console: port("port.serial"),
        grpc_port: port("grpc.port"),
        token: ini.get("grpc.token").filter(|t| !t.is_empty()).cloned(),
        avd_dir: ini.get("avd.dir").map(PathBuf::from),
    })
}

/// Default discovery folders of the platform (research R7; Linux: 011 research R12), then the
/// ones named by variables.
pub fn discovery_dirs(env: &super::tools::Env) -> Vec<PathBuf> {
    let mut dirs = crate::platform::emulator_discovery_dirs(&env.home, &env.vars);
    for key in ["ANDROID_EMULATOR_HOME", "ANDROID_USER_HOME"] {
        if let Some(v) = env.vars.get(key).filter(|v| !v.is_empty()) {
            dirs.push(PathBuf::from(v).join("avd/running"));
        }
    }
    dirs.push(env.home.join(".android/avd/running"));
    dirs
}

impl AndroidLauncher {
    pub fn new(sdk: SdkFinder, procs: Arc<dyn Processes>, discovery_dirs: Vec<PathBuf>) -> Self {
        Self {
            sdk,
            procs,
            discovery_dirs,
        }
    }

    fn instances(&self) -> Vec<Instance> {
        self.discovery_dirs
            .iter()
            .filter_map(|d| std::fs::read_dir(d).ok())
            .flat_map(|entries| entries.flatten().map(|e| e.path()))
            .filter_map(|p| read_instance(&p))
            .collect()
    }

    /// Console ports of emulators adb already knows; physical phones are ignored (FR-018).
    fn used_consoles(&self, adb: &Path) -> Vec<u16> {
        self.procs
            .run(adb, &["devices"], ADB_TIMEOUT)
            .map(|o| {
                o.text()
                    .lines()
                    .filter_map(|l| l.split_whitespace().next()?.strip_prefix("emulator-"))
                    .filter_map(|p| p.parse().ok())
                    .collect()
            })
            .unwrap_or_default()
    }

    fn free_ports(&self, adb: &Path) -> Result<(u16, u16), String> {
        let used = self.used_consoles(adb);
        let console = CONSOLE_PORTS
            .step_by(2)
            .find(|p| !used.contains(p) && self.procs.port_free(*p) && self.procs.port_free(p + 1))
            .ok_or("No free emulator port: too many emulators are running.")?;
        let grpc = (FIRST_GRPC_PORT..FIRST_GRPC_PORT + 200)
            .find(|p| self.procs.port_free(*p))
            .ok_or("No free port for the emulator's control channel.")?;
        Ok((console, grpc))
    }
}

impl DeviceLauncher for AndroidLauncher {
    fn start(&self, target: &Target) -> Result<Box<dyn Session>, String> {
        if target.platform != Platform::Android {
            return Err("not supported on iOS simulator".into());
        }
        let sdk = (self.sdk)()?;
        let running = self.instances().into_iter().find(|i| {
            i.avd == target.device_id && i.console.is_some() && self.procs.pid_alive(i.pid)
        });
        if let Some(i) = running {
            return Ok(Box::new(AndroidSession {
                procs: self.procs.clone(),
                adb: sdk.adb,
                serial: format!("emulator-{}", i.console.unwrap_or_default()),
                child: None,
                owned: false,
                pid: i.pid,
                grpc: i
                    .grpc_port
                    .zip(i.token)
                    .map(|(port, token)| Grpc { port, token }),
                discovery_dirs: self.discovery_dirs.clone(),
                started: Instant::now(),
                driver: None,
            }));
        }
        let (console, grpc_port) = self.free_ports(&sdk.adb)?;
        let args: Vec<String> = [
            "-avd",
            &target.device_id,
            "-port",
            &console.to_string(),
            "-grpc",
            &grpc_port.to_string(),
            "-grpc-use-token",
            "-qt-hide-window",
            "-no-boot-anim",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        let child = self.procs.spawn(&sdk.emulator, &args)?;
        Ok(Box::new(AndroidSession {
            procs: self.procs.clone(),
            adb: sdk.adb,
            serial: format!("emulator-{console}"),
            pid: child.pid(),
            child: Some(child),
            owned: true,
            grpc: None,
            discovery_dirs: self.discovery_dirs.clone(),
            started: Instant::now(),
            driver: None,
        }))
    }
}

struct AndroidSession {
    procs: Arc<dyn Processes>,
    adb: PathBuf,
    serial: String,
    /// `None` when attached to an emulator started outside Sonata, or after `stop`.
    child: Option<Box<dyn Child>>,
    owned: bool,
    pid: u32,
    grpc: Option<Grpc>,
    discovery_dirs: Vec<PathBuf>,
    started: Instant,
    driver: Option<Arc<AndroidDriver>>,
}

impl AndroidSession {
    /// This instance's discovery file: gRPC port, token (never passed on a command line), AVD.
    fn instance(&self) -> Option<Instance> {
        let name = format!("pid_{}.ini", self.pid);
        self.discovery_dirs
            .iter()
            .find_map(|d| read_instance(&d.join(&name)))
    }

    fn read_grpc(&self) -> Option<Grpc> {
        self.instance()
            .and_then(|i| i.grpc_port.zip(i.token))
            .map(|(port, token)| Grpc { port, token })
    }

    fn boot_completed(&self) -> bool {
        self.procs
            .run(
                &self.adb,
                &["-s", &self.serial, "shell", "getprop", "sys.boot_completed"],
                ADB_TIMEOUT,
            )
            .is_ok_and(|o| o.text().trim() == "1")
    }
}

impl Session for AndroidSession {
    fn poll(&mut self) -> Poll {
        match self.child.as_mut() {
            Some(child) => {
                if let Some(output) = child.exited() {
                    return Poll::Exited(output);
                }
            }
            None if self.owned => return Poll::Exited(None),
            None => {
                if !self.procs.pid_alive(self.pid) {
                    return Poll::Exited(None);
                }
            }
        }
        if let Some(d) = &self.driver {
            return Poll::Ready(d.clone());
        }
        if self.owned && self.grpc.is_none() {
            self.grpc = self.read_grpc();
        }
        if !self.boot_completed() {
            if self.started.elapsed() > BOOT_TIMEOUT {
                self.stop();
                return Poll::Exited(Some(format!(
                    "The device did not finish booting in {} minutes.",
                    BOOT_TIMEOUT.as_secs() / 60
                )));
            }
            return Poll::Booting;
        }
        let hw_keyboard = self.instance().is_some_and(|i| i.hw_keyboard());
        let driver = Arc::new(AndroidDriver::new(
            self.procs.clone(),
            self.adb.clone(),
            self.serial.clone(),
            self.grpc.clone(),
            hw_keyboard,
        ));
        self.driver = Some(driver.clone());
        Poll::Ready(driver)
    }

    fn stop(&mut self) {
        self.driver = None;
        let Some(mut child) = self.child.take() else {
            return;
        };
        let _ = self
            .procs
            .run(&self.adb, &["-s", &self.serial, "emu", "kill"], ADB_TIMEOUT);
        // The emulator saves its quick-boot snapshot while exiting; kill it only if it hangs.
        std::thread::spawn(move || {
            let start = Instant::now();
            while start.elapsed() < STOP_GRACE {
                if child.exited().is_some() {
                    return;
                }
                std::thread::sleep(Duration::from_millis(250));
            }
            child.kill();
        });
    }

    fn started_by_sonata(&self) -> bool {
        self.owned
    }
}

/// The launcher the app uses: the SDK is looked up again at each start, so a folder chosen or
/// installed while Sonata runs works without a restart (US2 AS4).
pub fn system_launcher(
    preference: impl Fn() -> Option<PathBuf> + Send + Sync + 'static,
) -> AndroidLauncher {
    let env = super::tools::Env::current();
    let dirs = discovery_dirs(&env);
    AndroidLauncher::new(
        Box::new(move || {
            let report =
                super::tools::detect(&super::tools::Env::current(), preference().as_deref());
            match (report.android.emulator(), report.android.adb()) {
                (Some(emulator), Some(adb)) => Ok(Sdk { emulator, adb }),
                _ => Err("Android SDK not found".into()),
            }
        }),
        Arc::new(super::process::SystemProcesses),
        dirs,
    )
}

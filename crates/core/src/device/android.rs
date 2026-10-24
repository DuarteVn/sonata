//! Controls of one running Android emulator: gRPC for screen and input, adb for the rest
//! (research R2–R4, R7–R9).

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use super::adb;
use super::grpc::{EmulatorClient, Frame};
use super::process::Processes;
use super::still::{Signature, StillTracker};
use super::{DeviceDriver, Key, StreamHandle};

const ADB_TIMEOUT: Duration = Duration::from_secs(10);
const DUMP_TIMEOUT: Duration = Duration::from_secs(20);
const INSTALL_TIMEOUT: Duration = super::BOOT_TIMEOUT;
/// Width of the small stream that watches for screen changes (research R5).
const WATCH_WIDTH: u32 = 64;
/// Reduced mode: one screenshot this often.
const POLL_INTERVAL: Duration = Duration::from_millis(300);

/// Where the emulator's gRPC service listens and its token.
#[derive(Clone, PartialEq, Eq)]
pub struct Grpc {
    pub port: u16,
    pub token: String,
}

impl std::fmt::Debug for Grpc {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Grpc")
            .field("port", &self.port)
            .finish_non_exhaustive()
    }
}

pub struct AndroidDriver {
    procs: Arc<dyn Processes>,
    adb: PathBuf,
    serial: String,
    /// `None`: reduced mode, adb only (research R7).
    grpc: Option<EmulatorClient>,
    /// The AVD has a hardware keyboard; without one the emulator drops gRPC key events, so
    /// text and keys go through adb (found on a real AVD).
    grpc_keys: bool,
    tracker: Arc<Mutex<StillTracker>>,
    watch: Mutex<Option<StreamHandle>>,
    size: OnceLock<(u32, u32)>,
    /// Reduced mode: where the finger went down, to turn down/up into a tap or a swipe.
    down_at: Mutex<Option<(i32, i32, Instant)>>,
}

impl AndroidDriver {
    pub fn new(
        procs: Arc<dyn Processes>,
        adb: PathBuf,
        serial: String,
        grpc: Option<Grpc>,
        hw_keyboard: bool,
    ) -> Self {
        Self {
            procs,
            adb,
            serial,
            grpc_keys: grpc.is_some() && hw_keyboard,
            grpc: grpc.map(|g| EmulatorClient::connect(g.port, &g.token)),
            tracker: Arc::new(Mutex::new(StillTracker::new(Instant::now()))),
            watch: Mutex::new(None),
            size: OnceLock::new(),
            down_at: Mutex::new(None),
        }
    }

    fn adb(&self, args: &[&str], timeout: Duration) -> Result<String, String> {
        let mut all = vec!["-s", self.serial.as_str()];
        all.extend_from_slice(args);
        let out = self.procs.run(&self.adb, &all, timeout)?;
        if out.success {
            Ok(out.text())
        } else {
            Err(first_line(&out.stderr).unwrap_or("adb failed").to_string())
        }
    }

    fn shell(&self, args: &[&str]) -> Result<String, String> {
        let mut all = vec!["shell"];
        all.extend_from_slice(args);
        self.adb(&all, ADB_TIMEOUT)
    }

    /// Starts the small stream that feeds the stillness tracker, once.
    fn ensure_watch(&self) {
        let mut watch = self.watch.lock().unwrap_or_else(|e| e.into_inner());
        if watch.is_some() {
            return;
        }
        let Some(client) = &self.grpc else { return };
        let (w, h) = self.screen_size().unwrap_or((1080, 2400));
        let height = (u64::from(h) * u64::from(WATCH_WIDTH) / u64::from(w.max(1))).max(1) as u32;
        let tracker = self.tracker.clone();
        *watch = Some(Box::new(client.stream(WATCH_WIDTH, height, move |f| {
            let sig = Signature::from_rgba(f.width, f.height, &f.rgba);
            tracker
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .push(sig, Instant::now());
        })));
    }

    fn key_code(key: &Key) -> &str {
        match key {
            Key::Back => "KEYCODE_BACK",
            Key::Home => "KEYCODE_HOME",
            Key::Recents => "KEYCODE_APP_SWITCH",
            Key::Enter => "KEYCODE_ENTER",
            Key::Delete => "KEYCODE_DEL",
            Key::Named(n) => match n.as_str() {
                "Backspace" => "KEYCODE_DEL",
                "Tab" => "KEYCODE_TAB",
                "Escape" => "KEYCODE_ESCAPE",
                "ArrowUp" => "KEYCODE_DPAD_UP",
                "ArrowDown" => "KEYCODE_DPAD_DOWN",
                "ArrowLeft" => "KEYCODE_DPAD_LEFT",
                "ArrowRight" => "KEYCODE_DPAD_RIGHT",
                _ => "KEYCODE_UNKNOWN",
            },
        }
    }

    /// W3C names the emulator understands (from the public proto's comments).
    fn key_name(key: &Key) -> &str {
        match key {
            Key::Back => "GoBack",
            Key::Home => "GoHome",
            Key::Recents => "AppSwitch",
            Key::Enter => "Enter",
            Key::Delete => "Backspace",
            Key::Named(n) => n,
        }
    }
}

fn first_line(s: &str) -> Option<&str> {
    s.lines().map(str::trim).find(|l| !l.is_empty())
}

impl DeviceDriver for AndroidDriver {
    fn screen_size(&self) -> Result<(u32, u32), String> {
        if let Some(s) = self.size.get() {
            return Ok(*s);
        }
        let size = adb::parse_wm_size(&self.shell(&["wm", "size"])?)
            .ok_or("could not read the screen size")?;
        Ok(*self.size.get_or_init(|| size))
    }

    fn touch(&self, x: i32, y: i32, down: bool) -> Result<(), String> {
        if let Some(c) = &self.grpc {
            self.ensure_watch();
            return c.touch(x, y, down);
        }
        let mut at = self.down_at.lock().unwrap_or_else(|e| e.into_inner());
        if down {
            *at = Some((x, y, Instant::now()));
            return Ok(());
        }
        let Some((x0, y0, t0)) = at.take() else {
            return Ok(());
        };
        let ms = t0.elapsed().as_millis().max(50).to_string();
        let (x0s, y0s, xs, ys) = (x0.to_string(), y0.to_string(), x.to_string(), y.to_string());
        if (x - x0).abs() < 10 && (y - y0).abs() < 10 && t0.elapsed() < Duration::from_millis(500) {
            self.shell(&["input", "tap", &xs, &ys]).map(|_| ())
        } else {
            self.shell(&["input", "swipe", &x0s, &y0s, &xs, &ys, &ms])
                .map(|_| ())
        }
    }

    fn wheel(&self, x: i32, y: i32, dx: i32, dy: i32) -> Result<(), String> {
        if let Some(c) = &self.grpc {
            self.ensure_watch();
            c.mouse_move(x, y)?;
            return c.wheel(dx, dy);
        }
        // Reduced mode: a short swipe in the scroll direction (one click ≈ 1/10 of a screen).
        let (_, h) = self.screen_size()?;
        let dist = (dy.signum() * (h as i32 / 10)).clamp(-(y - 1), h as i32 - y - 1);
        let (xs, ys, ye) = (x.to_string(), y.to_string(), (y + dist).to_string());
        self.shell(&["input", "swipe", &xs, &ys, &xs, &ye, "150"])
            .map(|_| ())
    }

    fn text(&self, text: &str) -> Result<(), String> {
        if !text.chars().all(|c| (' '..='~').contains(&c)) {
            return Err("only plain ASCII text can be typed on Android emulators".into());
        }
        if let Some(c) = self.grpc.as_ref().filter(|_| self.grpc_keys) {
            self.ensure_watch();
            return c.text(text);
        }
        // `input text` treats spaces as separators; `%s` is its escape for a space.
        let escaped: String = text
            .chars()
            .map(|c| match c {
                ' ' => "%s".to_string(),
                c if "\\\"'`$&|;<>()*?!#~[]{}".contains(c) => format!("\\{c}"),
                c => c.to_string(),
            })
            .collect();
        self.shell(&["input", "text", &escaped]).map(|_| ())
    }

    fn key(&self, key: &Key) -> Result<(), String> {
        if let Some(c) = self.grpc.as_ref().filter(|_| self.grpc_keys) {
            self.ensure_watch();
            return c.key(Self::key_name(key));
        }
        self.shell(&["input", "keyevent", Self::key_code(key)])
            .map(|_| ())
    }

    fn ui_dump(&self) -> Result<String, String> {
        // Right after an app starts, uiautomator can answer "null root node"; try again.
        let mut out = String::new();
        for attempt in 0..3 {
            if attempt > 0 {
                std::thread::sleep(Duration::from_millis(400));
            }
            out = self.adb(
                &["exec-out", "uiautomator", "dump", "/dev/tty"],
                DUMP_TIMEOUT,
            )?;
            if !out.contains("null root node") {
                break;
            }
        }
        if out.contains("could not get idle state") {
            return Err("screen never settled; try again or use coordinates".into());
        }
        Ok(adb::strip_dump_status(&out).to_string())
    }

    fn screenshot_png(&self) -> Result<Vec<u8>, String> {
        if let Some(c) = &self.grpc {
            return c.screenshot_png();
        }
        let out = self.procs.run(
            &self.adb,
            &["-s", &self.serial, "exec-out", "screencap", "-p"],
            ADB_TIMEOUT,
        )?;
        Ok(out.stdout)
    }

    fn apps(&self) -> Result<Vec<String>, String> {
        let out = self.shell(&[
            "cmd",
            "package",
            "query-activities",
            "--brief",
            "-a",
            "android.intent.action.MAIN",
            "-c",
            "android.intent.category.LAUNCHER",
        ])?;
        Ok(adb::parse_launchable(&out))
    }

    fn launch(&self, app: &str) -> Result<(), String> {
        if !app
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '_')
        {
            return Err("not installed or not launchable".into());
        }
        // `monkey` exits non-zero on emulators even when it works; resolve and start instead.
        let out = self.shell(&[
            "cmd",
            "package",
            "resolve-activity",
            "--brief",
            "-c",
            "android.intent.category.LAUNCHER",
            app,
        ])?;
        let component = out
            .lines()
            .map(str::trim)
            .rfind(|l| l.starts_with(app) && l.contains('/'))
            .ok_or("not installed or not launchable")?
            .to_string();
        let started = self.shell(&["am", "start", "-n", &component])?;
        if started.contains("Error") {
            return Err("not installed or not launchable".into());
        }
        Ok(())
    }

    fn install(&self, file: &Path) -> Result<String, String> {
        let before = adb::parse_packages(&self.shell(&["pm", "list", "packages"])?);
        let path = file.to_str().ok_or("the file path is not valid text")?;
        let out = self.adb(&["install", "-r", path], INSTALL_TIMEOUT)?;
        if !out.contains("Success") {
            return Err(first_line(&out).unwrap_or("install failed").to_string());
        }
        if let Some(id) = aapt2(&self.adb)
            .and_then(|tool| {
                self.procs
                    .run(&tool, &["dump", "packagename", path], ADB_TIMEOUT)
                    .ok()
            })
            .and_then(|o| adb::parse_aapt_package(&o.text()))
        {
            return Ok(id);
        }
        let after = adb::parse_packages(&self.shell(&["pm", "list", "packages"])?);
        Ok(after
            .into_iter()
            .find(|p| !before.contains(p))
            .unwrap_or_else(|| "installed (update of an existing app)".into()))
    }

    fn still_for(&self, since: Instant) -> Duration {
        if self.grpc.is_none() {
            // Reduced mode has no change feed: wait a fixed second after the action.
            return Instant::now()
                .saturating_duration_since(since)
                .saturating_sub(Duration::from_millis(500));
        }
        self.ensure_watch();
        self.tracker
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .still_for(since, Instant::now())
    }

    fn signature(&self) -> Option<Signature> {
        self.ensure_watch();
        self.tracker
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .signature()
            .cloned()
    }

    fn stream(
        &self,
        width: u32,
        height: u32,
        mut on_frame: Box<dyn FnMut(Frame) + Send>,
    ) -> Result<StreamHandle, String> {
        if let Some(c) = &self.grpc {
            return Ok(Box::new(c.stream(width, height, on_frame)));
        }
        let stop = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let flag = stop.clone();
        let (procs, adb, serial) = (self.procs.clone(), self.adb.clone(), self.serial.clone());
        std::thread::spawn(move || {
            let mut seq = 0;
            while !flag.load(std::sync::atomic::Ordering::SeqCst) {
                let png = procs
                    .run(
                        &adb,
                        &["-s", &serial, "exec-out", "screencap", "-p"],
                        ADB_TIMEOUT,
                    )
                    .map(|o| o.stdout);
                if let Some((w, h, rgba)) = png.ok().and_then(|p| decode_png(&p)) {
                    seq += 1;
                    let (tw, th) = (width.min(w).max(1), height.min(h).max(1));
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
        Ok(Box::new(StopOnDrop(stop)))
    }

    fn reduced(&self) -> bool {
        self.grpc.is_none()
    }
}

/// Stops a polling thread when dropped.
pub struct StopOnDrop(Arc<std::sync::atomic::AtomicBool>);

impl StopOnDrop {
    pub fn new(flag: Arc<std::sync::atomic::AtomicBool>) -> Self {
        Self(flag)
    }
}

impl Drop for StopOnDrop {
    fn drop(&mut self) {
        self.0.store(true, std::sync::atomic::Ordering::SeqCst);
    }
}

/// `aapt2` from the newest `build-tools` folder of the SDK holding `adb`, if any.
fn aapt2(adb: &Path) -> Option<PathBuf> {
    let sdk = adb.parent()?.parent()?;
    let mut versions: Vec<PathBuf> = std::fs::read_dir(sdk.join("build-tools"))
        .ok()?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.join("aapt2").is_file())
        .collect();
    versions.sort();
    versions.pop().map(|p| p.join("aapt2"))
}

/// PNG bytes to RGBA; `None` when not a PNG this decoder handles.
pub fn decode_png(bytes: &[u8]) -> Option<(u32, u32, Vec<u8>)> {
    let mut decoder = png::Decoder::new(std::io::Cursor::new(bytes));
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = decoder.read_info().ok()?;
    let mut buf = vec![0; reader.output_buffer_size()?];
    let info = reader.next_frame(&mut buf).ok()?;
    let (w, h) = (info.width, info.height);
    let px = (w * h) as usize;
    let rgba = match info.color_type {
        png::ColorType::Rgba => buf[..px * 4].to_vec(),
        png::ColorType::Rgb => buf[..px * 3]
            .as_chunks::<3>()
            .0
            .iter()
            .flat_map(|p| [p[0], p[1], p[2], 255])
            .collect(),
        png::ColorType::GrayscaleAlpha => buf[..px * 2]
            .as_chunks::<2>()
            .0
            .iter()
            .flat_map(|p| [p[0], p[0], p[0], p[1]])
            .collect(),
        png::ColorType::Grayscale => buf[..px].iter().flat_map(|g| [*g, *g, *g, 255]).collect(),
        png::ColorType::Indexed => return None,
    };
    Some((w, h, rgba))
}

/// Nearest-neighbor scaling of an RGBA image.
pub fn scale_rgba(src: &[u8], w: u32, h: u32, tw: u32, th: u32) -> Vec<u8> {
    if (w, h) == (tw, th) {
        return src.to_vec();
    }
    let mut out = Vec::with_capacity((tw * th * 4) as usize);
    for y in 0..th {
        let sy = (u64::from(y) * u64::from(h) / u64::from(th)) as u32;
        for x in 0..tw {
            let sx = (u64::from(x) * u64::from(w) / u64::from(tw)) as u32;
            let i = ((sy * w + sx) * 4) as usize;
            out.extend_from_slice(src.get(i..i + 4).unwrap_or(&[0, 0, 0, 255]));
        }
    }
    out
}

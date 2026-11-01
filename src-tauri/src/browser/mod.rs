//! Browser nodes (feature 004): real web pages in native child webviews over the canvas.
//! The frontend decides which pages are live and where (research R7); this module creates,
//! positions, and drives the webviews, and implements `sonata_core::browser::BrowserHost`.

mod snapshot;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc::channel;
use std::sync::{Arc, Condvar, Mutex, Weak};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sonata_core::App;
use sonata_core::browser::{BrowserHost, PageInfo};
use tauri::webview::{DownloadEvent, NewWindowResponse, PageLoadEvent};
use tauri::{
    AppHandle, Emitter, LogicalPosition, LogicalSize, Manager, Url, Webview, WebviewBuilder,
    WebviewUrl,
};
use uuid::Uuid;

const READ_JS: &str = include_str!("scripts/read.js");
const CLICK_JS: &str = include_str!("scripts/click.js");
const FILL_JS: &str = include_str!("scripts/fill.js");
const PRESS_JS: &str = include_str!("scripts/press.js");

/// Pages that are not live sit here, still rendering, out of sight (research R1).
const OFFSCREEN_X: f64 = -20_000.0;
const EVAL_TIMEOUT: Duration = Duration::from_secs(10);
/// A navigation must start this soon after an action, or the action did not navigate.
const NAV_START_GRACE: Duration = Duration::from_millis(500);
pub const STATE_EVENT: &str = "browser://state";

#[derive(Debug, Clone, Copy, Deserialize)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PageState {
    pub node_id: Uuid,
    pub url: String,
    pub title: String,
    pub loading: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<PathBuf>,
    pub image_version: u64,
    pub live: bool,
}

#[derive(Default)]
struct Load {
    /// Incremented when a navigation starts.
    started: u64,
    /// Incremented when a navigation finishes.
    finished: u64,
}

struct View {
    size: (f64, f64),
    state: PageState,
    load: Arc<(Mutex<Load>, Condvar)>,
}

pub struct Browsers {
    app: AppHandle,
    core: Weak<App>,
    image_dir: PathBuf,
    /// Linux and Windows keep each workspace's browsing data in a folder of its own (011
    /// research R10, 012 research R13); macOS uses WebKit data stores instead.
    #[cfg_attr(target_os = "macos", allow(dead_code))]
    data_dir: PathBuf,
    views: Mutex<HashMap<Uuid, View>>,
}

fn label_for(node: Uuid) -> String {
    format!("browser-{}", node.simple())
}

fn allowed(url: &Url) -> bool {
    matches!(url.scheme(), "http" | "https") || url.as_str() == "about:blank"
}

fn open_externally(url: &str) {
    let _ = sonata_core::platform::open_url_command(url).spawn();
}

impl Browsers {
    pub fn new(app: AppHandle, core: Weak<App>, data_root: &Path) -> Arc<Self> {
        let image_dir = data_root.join("run").join("browser");
        let _ = sonata_core::paths::ensure_private_dir(&image_dir);
        Arc::new(Self {
            app,
            core,
            image_dir,
            data_dir: data_root.join("browser-data"),
            views: Mutex::default(),
        })
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<Uuid, View>> {
        self.views.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn webview(&self, node: Uuid) -> Option<Webview> {
        self.app.get_webview(&label_for(node))
    }

    fn emit(&self, node: Uuid) {
        if let Some(state) = self.lock().get(&node).map(|v| v.state.clone()) {
            let _ = self.app.emit(STATE_EVENT, state);
        }
    }

    /// Creates the node's webview off screen if needed (from the node's saved URL).
    fn ensure(self: &Arc<Self>, node: Uuid) -> Result<Webview, String> {
        if let Some(w) = self.webview(node) {
            return Ok(w);
        }
        let core = self.core.upgrade().ok_or("Sonata is closing.")?;
        let ws = core.workspace_of_node(node).map_err(|e| e.message)?;
        let (url, frame) = {
            let m = core.workspaces();
            let n = m
                .workspace(ws)
                .and_then(|w| w.node(node))
                .ok_or("The browser is gone.")?;
            let b = n.browser().ok_or("That item is not a browser.")?;
            (b.url.clone(), n.frame)
        };
        let window = self
            .app
            .get_window("main")
            .ok_or("The Sonata window is not ready.")?;
        let label = label_for(node);
        let parsed: Url = url
            .parse()
            .unwrap_or_else(|_| "about:blank".parse().expect("valid"));
        let load = Arc::new((Mutex::new(Load::default()), Condvar::new()));

        let this = Arc::downgrade(self);
        let load_cb = load.clone();
        let app_nav = self.app.clone();
        let label_nav = label.clone();
        let builder = WebviewBuilder::new(&label, WebviewUrl::External(parsed));
        #[cfg(target_os = "macos")]
        let builder = builder.data_store_identifier(*ws.as_bytes());
        #[cfg(not(target_os = "macos"))]
        let builder = {
            let dir = self.data_dir.join(ws.simple().to_string());
            sonata_core::paths::ensure_private_dir(&dir)
                .map_err(|e| format!("Could not create the browser data folder: {e}"))?;
            builder.data_directory(dir)
        };
        let builder = builder
            .devtools(true)
            .on_navigation(allowed)
            .on_new_window(move |url, _features| {
                // Popups and target=_blank open in the same node.
                if allowed(&url)
                    && let Some(w) = app_nav.get_webview(&label_nav)
                {
                    let _ = w.navigate(url);
                }
                NewWindowResponse::Deny
            })
            .on_download(|_webview, event| {
                if let DownloadEvent::Requested { url, .. } = event {
                    open_externally(url.as_str());
                }
                false
            })
            .on_page_load(move |webview, payload| {
                let (lock, cvar) = &*load_cb;
                let mut l = lock.lock().unwrap_or_else(|e| e.into_inner());
                match payload.event() {
                    PageLoadEvent::Started => l.started += 1,
                    PageLoadEvent::Finished => l.finished += 1,
                }
                drop(l);
                cvar.notify_all();
                let url = payload.url().to_string();
                let finished = matches!(payload.event(), PageLoadEvent::Finished);
                if let Some(this) = this.upgrade() {
                    this.on_load(node, webview, url, finished);
                }
            });
        let webview = window
            .add_child(
                builder,
                LogicalPosition::new(OFFSCREEN_X, 0.0),
                LogicalSize::new(frame.width, frame.height),
            )
            .map_err(|e| format!("Could not open the browser: {e}"))?;
        self.lock().insert(
            node,
            View {
                size: (frame.width, frame.height),
                state: PageState {
                    node_id: node,
                    url,
                    loading: true,
                    ..Default::default()
                },
                load,
            },
        );
        Ok(webview)
    }

    fn on_load(self: Arc<Self>, node: Uuid, webview: Webview, url: String, finished: bool) {
        if let Some(v) = self.lock().get_mut(&node) {
            v.state.url = url.clone();
            v.state.loading = !finished;
            v.state.error = None;
        }
        if finished {
            let this = self.clone();
            let _ = webview.eval_with_callback("document.title", move |title| {
                let title: String = serde_json::from_str(&title).unwrap_or_default();
                if let Some(v) = this.lock().get_mut(&node) {
                    v.state.title = title.clone();
                }
                if let Some(core) = this.core.upgrade() {
                    let _ = core.update_browser_page(node, &url, Some(&title));
                }
                this.emit(node);
            });
        }
        self.emit(node);
    }

    /// Applies the frontend's layout: listed nodes are live at their rect; every other page is
    /// captured for its card and moved off screen.
    pub fn layout(self: &Arc<Self>, items: Vec<(Uuid, Option<Rect>)>) {
        let wanted: HashMap<Uuid, Option<Rect>> = items.into_iter().collect();
        for (node, rect) in &wanted {
            if rect.is_some() && self.ensure(*node).is_err() {
                continue;
            }
            if let Some(r) = rect {
                if let Some(w) = self.webview(*node) {
                    let _ = w.set_position(LogicalPosition::new(r.x, r.y));
                    let _ = w.set_size(LogicalSize::new(r.width, r.height));
                }
                if let Some(v) = self.lock().get_mut(node) {
                    v.size = (r.width, r.height);
                    v.state.live = true;
                }
            }
        }
        let to_hide: Vec<Uuid> = self
            .lock()
            .iter()
            .filter(|(id, v)| v.state.live && !wanted.get(*id).is_some_and(Option::is_some))
            .map(|(id, _)| *id)
            .collect();
        for node in to_hide {
            self.capture_card(node);
            if let Some(w) = self.webview(node) {
                let _ = w.set_position(LogicalPosition::new(OFFSCREEN_X, 0.0));
            }
            if let Some(v) = self.lock().get_mut(&node) {
                v.state.live = false;
            }
            self.emit(node);
        }
    }

    /// Refreshes the card image of a page (FR-008).
    pub fn capture_card(self: &Arc<Self>, node: Uuid) {
        let Some(w) = self.webview(node) else { return };
        let path = self.image_dir.join(format!("{}.png", node.simple()));
        let this = Arc::downgrade(self);
        snapshot::capture(&w, path.clone(), move |ok| {
            let Some(this) = this.upgrade() else { return };
            if ok && let Some(v) = this.lock().get_mut(&node) {
                v.state.image = Some(path.clone());
                v.state.image_version += 1;
            }
            this.emit(node);
        });
    }

    pub fn refresh_live_cards(self: &Arc<Self>) {
        let live: Vec<Uuid> = self
            .lock()
            .iter()
            .filter(|(_, v)| v.state.live)
            .map(|(id, _)| *id)
            .collect();
        for node in live {
            self.capture_card(node);
        }
    }

    /// Navigates without waiting (address bar).
    pub fn navigate(self: &Arc<Self>, node: Uuid, url: &str) -> Result<(), String> {
        let w = self.ensure(node)?;
        let url: Url = url.parse().map_err(|e| format!("Invalid address: {e}"))?;
        w.navigate(url).map_err(|e| e.to_string())
    }

    pub fn action(self: &Arc<Self>, node: Uuid, action: &str) -> Result<(), String> {
        let w = self.ensure(node)?;
        match action {
            "back" => w.eval("history.back()").map_err(|e| e.to_string()),
            "forward" => w.eval("history.forward()").map_err(|e| e.to_string()),
            "reload" => w.reload().map_err(|e| e.to_string()),
            "stop" => w.eval("window.stop()").map_err(|e| e.to_string()),
            "inspect" => {
                w.open_devtools();
                Ok(())
            }
            "external" => {
                let url = w.url().map_err(|e| e.to_string())?;
                open_externally(url.as_str());
                Ok(())
            }
            other => Err(format!("Unknown browser action {other}.")),
        }
    }

    pub fn close(&self, node: Uuid) {
        if let Some(w) = self.webview(node) {
            let _ = w.close();
        }
        self.lock().remove(&node);
        let _ = std::fs::remove_file(self.image_dir.join(format!("{}.png", node.simple())));
    }

    /// Closes the views of a workspace and deletes its browsing data (FR-010).
    pub fn forget_workspace(&self, ws: Uuid, nodes: &[Uuid]) {
        for n in nodes {
            self.close(*n);
        }
        #[cfg(target_os = "macos")]
        snapshot::remove_data_store(&self.app, ws);
        #[cfg(target_os = "linux")]
        if let Err(err) = std::fs::remove_dir_all(self.data_dir.join(ws.simple().to_string()))
            && err.kind() != std::io::ErrorKind::NotFound
        {
            tracing::warn!(error = %err, "could not delete the workspace's browsing data");
        }
        // WebView2 holds the folder for a moment after its views close (feature 012).
        #[cfg(windows)]
        {
            let dir = self.data_dir.join(ws.simple().to_string());
            std::thread::spawn(move || {
                for _ in 0..20 {
                    if !dir.exists() || std::fs::remove_dir_all(&dir).is_ok() {
                        return;
                    }
                    std::thread::sleep(Duration::from_millis(500));
                }
                tracing::warn!("could not delete the workspace's browsing data");
            });
        }
    }

    fn load_counts(&self, node: Uuid) -> Option<(u64, u64)> {
        let load = self.lock().get(&node).map(|v| v.load.clone())?;
        let l = load.0.lock().unwrap_or_else(|e| e.into_inner());
        Some((l.started, l.finished))
    }

    /// Waits for a navigation that started after `before` to finish. With `optional`, returns
    /// at once when no navigation starts within a short grace period.
    fn wait_load(
        &self,
        node: Uuid,
        before: (u64, u64),
        timeout: Duration,
        optional: bool,
    ) -> Result<(), String> {
        let load = self
            .lock()
            .get(&node)
            .map(|v| v.load.clone())
            .ok_or("The browser is gone.")?;
        let (lock, cvar) = &*load;
        let start = Instant::now();
        let mut l = lock.lock().unwrap_or_else(|e| e.into_inner());
        loop {
            let started = l.started > before.0;
            if started && l.finished >= l.started && l.finished > before.1 {
                return Ok(());
            }
            if optional && !started && start.elapsed() >= NAV_START_GRACE {
                return Ok(());
            }
            let left = timeout.checked_sub(start.elapsed()).ok_or_else(|| {
                format!(
                    "The page did not finish loading in {}s (the server may be down or slow).",
                    timeout.as_secs()
                )
            })?;
            l = cvar
                .wait_timeout(l, left.min(Duration::from_millis(100)))
                .unwrap_or_else(|e| e.into_inner())
                .0;
        }
    }

    /// Runs a script and returns its JSON result.
    fn eval_json(&self, w: &Webview, script: String) -> Result<String, String> {
        let (tx, rx) = channel();
        w.eval_with_callback(script, move |out| {
            let _ = tx.send(out);
        })
        .map_err(|e| e.to_string())?;
        rx.recv_timeout(EVAL_TIMEOUT)
            .map_err(|_| "The page did not answer in time.".to_string())
    }

    /// Runs one of our page scripts with arguments; the script returns a JSON string.
    fn run_script(&self, w: &Webview, script: &str, args: Value) -> Result<Value, String> {
        let wrapped = format!("(() => {{ const __SONATA_ARGS = {args}; return {script}; }})()");
        let raw = self.eval_json(w, wrapped)?;
        let inner: String =
            serde_json::from_str(&raw).map_err(|e| format!("Unexpected page answer: {e}"))?;
        let v: Value =
            serde_json::from_str(&inner).map_err(|e| format!("Unexpected page answer: {e}"))?;
        match v.get("error").and_then(Value::as_str) {
            Some(e) => Err(e.to_string()),
            None => Ok(v),
        }
    }

    fn page_info(&self, w: &Webview, node: Uuid) -> PageInfo {
        let url = w.url().map(|u| u.to_string()).unwrap_or_default();
        let title = self
            .eval_json(w, "document.title".into())
            .ok()
            .and_then(|t| serde_json::from_str::<String>(&t).ok())
            .or_else(|| self.lock().get(&node).map(|v| v.state.title.clone()))
            .unwrap_or_default();
        PageInfo { url, title }
    }
}

/// Shares the browsers with core through an owned handle (core needs `'static` + Send + Sync).
pub struct Host(pub Arc<Browsers>);

impl Host {
    fn nav(
        &self,
        node: Uuid,
        timeout: Duration,
        optional: bool,
        act: impl FnOnce(&Webview) -> Result<(), String>,
    ) -> Result<PageInfo, String> {
        let w = self.0.ensure(node)?;
        let before = self.0.load_counts(node).unwrap_or_default();
        act(&w)?;
        self.0.wait_load(node, before, timeout, optional)?;
        Ok(self.0.page_info(&w, node))
    }
}

impl BrowserHost for Host {
    fn open(&self, node: Uuid, url: &str, timeout: Duration) -> Result<PageInfo, String> {
        let parsed: Url = url.parse().map_err(|e| format!("Invalid address: {e}"))?;
        self.nav(node, timeout, false, |w| {
            w.navigate(parsed).map_err(|e| e.to_string())
        })
    }

    fn back(&self, node: Uuid, timeout: Duration) -> Result<PageInfo, String> {
        self.nav(node, timeout, true, |w| {
            w.eval("history.back()").map_err(|e| e.to_string())
        })
    }

    fn forward(&self, node: Uuid, timeout: Duration) -> Result<PageInfo, String> {
        self.nav(node, timeout, true, |w| {
            w.eval("history.forward()").map_err(|e| e.to_string())
        })
    }

    fn reload(&self, node: Uuid, timeout: Duration) -> Result<PageInfo, String> {
        self.nav(node, timeout, false, |w| {
            w.reload().map_err(|e| e.to_string())
        })
    }

    fn read(&self, node: Uuid) -> Result<String, String> {
        let w = self.0.ensure(node)?;
        let v = self.0.run_script(&w, READ_JS, Value::Null)?;
        let mut out = format!(
            "Title: {}\nURL: {}\n\n{}\n",
            v["title"].as_str().unwrap_or_default(),
            v["url"].as_str().unwrap_or_default(),
            v["text"].as_str().unwrap_or_default()
        );
        let elements = v["elements"].as_array().cloned().unwrap_or_default();
        if !elements.is_empty() {
            out.push_str("\nInteractive:\n");
            for e in elements {
                let extra = match (e["kind"].as_str(), e["extra"].as_str()) {
                    (Some("link"), Some(href)) => format!(" → {href}"),
                    (_, Some(x)) => format!(" ({x})"),
                    _ => String::new(),
                };
                out.push_str(&format!(
                    "  [{}] {:<8} \"{}\"{extra}\n",
                    e["n"],
                    e["kind"].as_str().unwrap_or_default(),
                    e["label"].as_str().unwrap_or_default()
                ));
            }
        }
        Ok(out)
    }

    fn click(&self, node: Uuid, target: &str, timeout: Duration) -> Result<PageInfo, String> {
        self.nav(node, timeout, true, |w| {
            self.0
                .run_script(w, CLICK_JS, json!({"target": target}))
                .map(|_| ())
        })
    }

    fn fill(&self, node: Uuid, target: &str, text: &str) -> Result<(), String> {
        let w = self.0.ensure(node)?;
        self.0
            .run_script(&w, FILL_JS, json!({"target": target, "text": text}))
            .map(|_| ())
    }

    fn press(&self, node: Uuid, key: &str) -> Result<(), String> {
        let w = self.0.ensure(node)?;
        self.0
            .run_script(&w, PRESS_JS, json!({"key": key}))
            .map(|_| ())
    }

    fn screenshot(&self, node: Uuid, out: &Path) -> Result<(), String> {
        let w = self.0.ensure(node)?;
        let (tx, rx) = channel();
        snapshot::capture(&w, out.to_path_buf(), move |ok| {
            let _ = tx.send(ok);
        });
        match rx.recv_timeout(EVAL_TIMEOUT) {
            Ok(true) => Ok(()),
            _ => Err("Could not capture the page.".into()),
        }
    }

    fn eval(&self, node: Uuid, js: &str) -> Result<String, String> {
        let w = self.0.ensure(node)?;
        self.0.eval_json(&w, js.to_string())
    }

    fn close(&self, node: Uuid) {
        self.0.close(node);
    }

    fn forget_workspace(&self, ws: Uuid, nodes: &[Uuid]) {
        self.0.forget_workspace(ws, nodes);
    }
}

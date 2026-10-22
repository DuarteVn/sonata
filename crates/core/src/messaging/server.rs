//! The Unix socket the `sonata` CLI talks to (research 003 R1). One request per connection,
//! JSON lines. Each connection runs on its own thread because asks can wait for minutes.

use std::io::{self, BufRead, BufReader, Read, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::Weak;
use std::sync::atomic::{AtomicBool, Ordering};

use serde_json::json;

use super::{EXIT_DENIED, EXIT_USAGE, MAX_REQUEST_BYTES, Reply, Request};
use crate::App;
use crate::paths::{ensure_private_dir, set_mode};

/// macOS limits socket paths to 104 bytes.
const MAX_SOCKET_PATH: usize = 100;

/// `<root>/run/sonata.sock`, or a private temp folder when that path is too long.
pub fn socket_path(root: &Path) -> io::Result<PathBuf> {
    let preferred = root.join("run").join("sonata.sock");
    if preferred.as_os_str().len() <= MAX_SOCKET_PATH {
        ensure_private_dir(&root.join("run"))?;
        return Ok(preferred);
    }
    use std::hash::{DefaultHasher, Hash, Hasher};
    let mut h = DefaultHasher::new();
    root.hash(&mut h);
    // SAFETY: getuid has no preconditions.
    let uid = unsafe { libc::getuid() };
    let dir = std::env::temp_dir().join(format!("sonata-{uid}"));
    ensure_private_dir(&dir)?;
    Ok(dir.join(format!("{:016x}.sock", h.finish())))
}

pub struct Server {
    path: PathBuf,
    stop: Arc<AtomicBool>,
}

impl Server {
    pub fn start(app: Weak<App>, path: PathBuf) -> io::Result<Self> {
        let _ = std::fs::remove_file(&path);
        let listener = UnixListener::bind(&path)?;
        set_mode(&path, 0o600)?;
        let stop = Arc::new(AtomicBool::new(false));
        let stop_flag = stop.clone();
        std::thread::Builder::new()
            .name("sonata-socket".into())
            .spawn(move || {
                for stream in listener.incoming() {
                    if stop_flag.load(Ordering::SeqCst) {
                        break;
                    }
                    let Ok(stream) = stream else { continue };
                    let app = app.clone();
                    let _ = std::thread::Builder::new()
                        .name("sonata-request".into())
                        .spawn(move || handle(app, stream));
                }
            })?;
        Ok(Self { path, stop })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        let _ = UnixStream::connect(&self.path);
        let _ = std::fs::remove_file(&self.path);
    }
}

fn send_line(stream: &mut UnixStream, value: &serde_json::Value) -> io::Result<()> {
    let mut line = serde_json::to_vec(value)?;
    line.push(b'\n');
    stream.write_all(&line)
}

fn send_result(stream: &mut UnixStream, reply: &Reply) {
    let mut v = serde_json::to_value(reply).unwrap_or_default();
    v["type"] = json!("result");
    let _ = send_line(stream, &v);
}

fn handle(app: Weak<App>, mut stream: UnixStream) {
    let mut line = String::new();
    let read = BufReader::new((&stream).take(MAX_REQUEST_BYTES as u64 + 1)).read_line(&mut line);
    if read.is_err() || line.len() > MAX_REQUEST_BYTES {
        send_result(&mut stream, &Reply::err(EXIT_USAGE, "Request too large."));
        return;
    }
    let req: Request = match serde_json::from_str(&line) {
        Ok(r) => r,
        Err(e) => {
            send_result(
                &mut stream,
                &Reply::err(EXIT_USAGE, format!("Malformed request: {e}")),
            );
            return;
        }
    };
    let Some(app) = app.upgrade() else { return };
    if req.v != 1 || !app.terminals().check_token(req.terminal_id, &req.token) {
        tracing::warn!(terminal = %req.terminal_id, "refused CLI request with a bad identity");
        send_result(
            &mut stream,
            &Reply::err(EXIT_DENIED, "This terminal's Sonata identity is not valid."),
        );
        return;
    }
    let mut progress_stream = stream.try_clone().ok();
    let mut progress = move |message: &str| {
        if let Some(s) = progress_stream.as_mut() {
            let _ = send_line(s, &json!({"type": "progress", "message": message}));
        }
    };
    let reply = super::commands::handle(&app, &req, &mut progress);
    send_result(&mut stream, &reply);
}

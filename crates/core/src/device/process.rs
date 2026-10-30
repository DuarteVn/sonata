//! Running SDK tools. A trait so tests can script them (constitution III).

use std::io::Read;
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Output {
    pub success: bool,
    /// Raw bytes: screenshots are binary.
    pub stdout: Vec<u8>,
    pub stderr: String,
}

impl Output {
    pub fn text(&self) -> String {
        String::from_utf8_lossy(&self.stdout).into_owned()
    }
}

/// A started background program (the emulator).
pub trait Child: Send {
    fn pid(&self) -> u32;
    /// `Some(last output)` once the program ended.
    fn exited(&mut self) -> Option<Option<String>>;
    fn kill(&mut self);
}

pub trait Processes: Send + Sync {
    /// Runs a program to completion, killing it after `timeout`.
    fn run(&self, program: &Path, args: &[&str], timeout: Duration) -> Result<Output, String>;
    fn spawn(&self, program: &Path, args: &[String]) -> Result<Box<dyn Child>, String>;
    /// True when a localhost TCP port can be bound.
    fn port_free(&self, port: u16) -> bool;
    fn pid_alive(&self, pid: u32) -> bool;
}

/// Real processes.
pub struct SystemProcesses;

fn read_all(mut r: impl Read + Send + 'static) -> std::thread::JoinHandle<Vec<u8>> {
    std::thread::spawn(move || {
        let mut buf = Vec::new();
        let _ = r.read_to_end(&mut buf);
        buf
    })
}

impl Processes for SystemProcesses {
    fn run(&self, program: &Path, args: &[&str], timeout: Duration) -> Result<Output, String> {
        let mut cmd = Command::new(program);
        crate::platform::child_env().apply_std(&mut cmd);
        let mut child = crate::platform::no_window(&mut cmd)
            .args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("could not run {}: {e}", program.display()))?;
        let out = read_all(child.stdout.take().expect("piped"));
        let err = read_all(child.stderr.take().expect("piped"));
        let start = Instant::now();
        let status = loop {
            if let Some(status) = child.try_wait().map_err(|e| e.to_string())? {
                break status;
            }
            if start.elapsed() >= timeout {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!("timed out after {}s", timeout.as_secs()));
            }
            std::thread::sleep(Duration::from_millis(10));
        };
        Ok(Output {
            success: status.success(),
            stdout: out.join().unwrap_or_default(),
            stderr: String::from_utf8_lossy(&err.join().unwrap_or_default()).into_owned(),
        })
    }

    fn spawn(&self, program: &Path, args: &[String]) -> Result<Box<dyn Child>, String> {
        let mut cmd = Command::new(program);
        crate::platform::child_env().apply_std(&mut cmd);
        let mut child = crate::platform::no_window(&mut cmd)
            .args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("could not start {}: {e}", program.display()))?;
        let tail = Arc::new(Mutex::new(Tail::default()));
        for stream in [
            Box::new(child.stdout.take().expect("piped")) as Box<dyn Read + Send>,
            Box::new(child.stderr.take().expect("piped")),
        ] {
            let tail = tail.clone();
            std::thread::spawn(move || {
                use std::io::BufRead;
                for line in std::io::BufReader::new(stream)
                    .lines()
                    .map_while(Result::ok)
                {
                    tail.lock().unwrap_or_else(|e| e.into_inner()).push(line);
                }
            });
        }
        Ok(Box::new(SystemChild { child, tail }))
    }

    fn port_free(&self, port: u16) -> bool {
        std::net::TcpListener::bind(("127.0.0.1", port)).is_ok()
    }

    fn pid_alive(&self, pid: u32) -> bool {
        crate::platform::process_alive(pid)
    }
}

/// The last lines a program printed, for error messages.
#[derive(Default)]
struct Tail(std::collections::VecDeque<String>);

impl Tail {
    const LINES: usize = 12;

    fn push(&mut self, line: String) {
        if self.0.len() == Self::LINES {
            self.0.pop_front();
        }
        self.0.push_back(line);
    }

    /// Error lines when there are some, else everything kept.
    fn summary(&self) -> String {
        let errors: Vec<&str> = self
            .0
            .iter()
            .map(String::as_str)
            .filter(|l| l.contains("ERROR") || l.contains("FATAL"))
            .collect();
        if errors.is_empty() {
            self.0.iter().cloned().collect::<Vec<_>>().join("\n")
        } else {
            errors.join("\n")
        }
    }
}

struct SystemChild {
    child: std::process::Child,
    tail: Arc<Mutex<Tail>>,
}

impl Child for SystemChild {
    fn pid(&self) -> u32 {
        self.child.id()
    }

    fn exited(&mut self) -> Option<Option<String>> {
        match self.child.try_wait() {
            Ok(Some(_)) | Err(_) => {
                let text = self
                    .tail
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .summary();
                Some((!text.trim().is_empty()).then_some(text))
            }
            Ok(None) => None,
        }
    }

    fn kill(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

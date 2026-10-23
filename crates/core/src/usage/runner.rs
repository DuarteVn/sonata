//! Runs provider commands: argv only, scrubbed environment, time limit, output caps (research R5).

use std::io::{Read, Write};
use std::os::unix::process::CommandExt;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

pub const STDOUT_CAP: usize = 64 * 1024;
pub const STDERR_CAP: usize = 4 * 1024;
/// Variables passed through from Sonata's own environment. Everything else is dropped.
pub const ENV_ALLOWLIST: [&str; 6] = ["HOME", "USER", "LOGNAME", "LANG", "TMPDIR", "PATH"];

const POLL: Duration = Duration::from_millis(10);
const DRAIN: Duration = Duration::from_millis(500);

/// What to run. `env` adds or replaces variables after the allowlist.
pub struct RunSpec {
    pub argv: Vec<String>,
    pub env: Vec<(String, String)>,
    pub timeout: Duration,
    pub stdout_cap: usize,
    /// Written to the command's stdin, which stays open until the run ends. `None`: no stdin.
    pub stdin: Option<Vec<u8>>,
    /// Ends the run early (the process group is killed) once stdout satisfies it.
    pub stop_when: Option<fn(&[u8]) -> bool>,
}

impl RunSpec {
    pub fn new(argv: Vec<String>, timeout: Duration) -> Self {
        Self {
            argv,
            env: Vec::new(),
            timeout,
            stdout_cap: STDOUT_CAP,
            stdin: None,
            stop_when: None,
        }
    }
}

#[derive(Debug)]
pub struct RunOutput {
    pub stdout: Vec<u8>,
    /// First 4 KiB of stderr, lossy UTF-8. Redact before logging.
    pub stderr: String,
    /// `None` when the command was stopped or killed by a signal.
    pub code: Option<i32>,
    /// True when `stop_when` ended the run.
    pub stopped: bool,
}

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum RunError {
    #[error("command not found")]
    NotFound,
    #[error("could not start: {0}")]
    Spawn(String),
    #[error("timed out after {} s", fmt_secs(*.0))]
    TimedOut(Duration),
    #[error("printed more than {} KiB", .0 / 1024)]
    OutputTooLarge(usize),
}

fn fmt_secs(d: Duration) -> String {
    let s = d.as_secs_f64();
    if s.fract() == 0.0 {
        format!("{s:.0}")
    } else {
        format!("{s:.1}")
    }
}

/// Runs `spec` in a new process group inside an empty temporary folder. The whole group is
/// killed when the command exits, times out, overflows, or meets `stop_when`, so no child
/// process outlives the run.
pub fn run(spec: &RunSpec) -> Result<RunOutput, RunError> {
    let (program, args) = spec
        .argv
        .split_first()
        .ok_or_else(|| RunError::Spawn("empty command".into()))?;
    let cwd = tempfile::tempdir().map_err(|e| RunError::Spawn(e.to_string()))?;
    let mut cmd = Command::new(program);
    cmd.args(args)
        .env_clear()
        .current_dir(cwd.path())
        .process_group(0)
        .stdin(if spec.stdin.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for key in ENV_ALLOWLIST {
        if let Some(value) = std::env::var_os(key) {
            cmd.env(key, value);
        }
    }
    for (key, value) in &spec.env {
        cmd.env(key, value);
    }
    let mut child = cmd.spawn().map_err(|e| match e.kind() {
        std::io::ErrorKind::NotFound => RunError::NotFound,
        _ => RunError::Spawn(e.to_string()),
    })?;
    let pid = child.id() as i32;

    // Kept open until the function returns; some servers exit on end of input.
    let _stdin = child.stdin.take().map(|mut pipe| {
        if let Some(data) = &spec.stdin {
            let _ = pipe.write_all(data);
            let _ = pipe.flush();
        }
        pipe
    });
    let stdout = Reader::spawn(child.stdout.take(), spec.stdout_cap + 1);
    let stderr = Reader::spawn(child.stderr.take(), STDERR_CAP);

    let start = Instant::now();
    let mut stopped = false;
    let outcome = loop {
        if stdout.len() > spec.stdout_cap {
            break Err(RunError::OutputTooLarge(spec.stdout_cap));
        }
        if let Some(stop) = spec.stop_when
            && stop(&stdout.snapshot())
        {
            stopped = true;
            break Ok(());
        }
        if has_exited(pid) {
            break Ok(());
        }
        if start.elapsed() >= spec.timeout {
            break Err(RunError::TimedOut(spec.timeout));
        }
        std::thread::sleep(POLL);
    };
    // The leader is not reaped yet, so its pid still names this group.
    unsafe {
        libc::killpg(pid, libc::SIGKILL);
    }
    let status = child.wait().ok();
    outcome?;
    stdout.wait_done(DRAIN);
    stderr.wait_done(DRAIN);
    let out = stdout.snapshot();
    if out.len() > spec.stdout_cap {
        return Err(RunError::OutputTooLarge(spec.stdout_cap));
    }
    Ok(RunOutput {
        stdout: out,
        stderr: String::from_utf8_lossy(&stderr.snapshot()).into_owned(),
        code: if stopped {
            None
        } else {
            status.and_then(|s| s.code())
        },
        stopped,
    })
}

/// True once the process has exited, without reaping it.
fn has_exited(pid: i32) -> bool {
    let mut info: libc::siginfo_t = unsafe { std::mem::zeroed() };
    let flags = libc::WEXITED | libc::WNOHANG | libc::WNOWAIT;
    let r = unsafe { libc::waitid(libc::P_PID, pid as libc::id_t, &mut info, flags) };
    r == 0 && info.si_pid != 0
}

/// Reads a pipe on its own thread into a buffer of at most `cap` bytes.
struct Reader {
    buf: Arc<Mutex<Vec<u8>>>,
    done: Arc<AtomicBool>,
}

impl Reader {
    fn spawn(pipe: Option<impl Read + Send + 'static>, cap: usize) -> Self {
        let buf = Arc::new(Mutex::new(Vec::new()));
        let done = Arc::new(AtomicBool::new(pipe.is_none()));
        if let Some(mut pipe) = pipe {
            let (b, d) = (buf.clone(), done.clone());
            std::thread::spawn(move || {
                let mut chunk = [0u8; 8192];
                while let Ok(n) = pipe.read(&mut chunk) {
                    if n == 0 {
                        break;
                    }
                    let mut out = b.lock().unwrap_or_else(|e| e.into_inner());
                    let room = cap.saturating_sub(out.len());
                    out.extend_from_slice(&chunk[..n.min(room)]);
                    // Keep draining past the cap so the writer never blocks on a full pipe.
                }
                d.store(true, Ordering::SeqCst);
            });
        }
        Self { buf, done }
    }

    fn len(&self) -> usize {
        self.buf.lock().unwrap_or_else(|e| e.into_inner()).len()
    }

    fn snapshot(&self) -> Vec<u8> {
        self.buf.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }

    fn wait_done(&self, limit: Duration) {
        let start = Instant::now();
        while !self.done.load(Ordering::SeqCst) && start.elapsed() < limit {
            std::thread::sleep(POLL);
        }
    }
}

/// Replaces runs of 32 or more token characters (letters, digits, `-`, `_`) with `[redacted]`,
/// so stderr lines can be logged without leaking keys.
pub fn redact(text: &str) -> String {
    let is_token = |c: char| c.is_ascii_alphanumeric() || c == '-' || c == '_';
    let mut out = String::with_capacity(text.len());
    let mut run = String::new();
    let flush = |run: &mut String, out: &mut String| {
        if run.len() >= 32 {
            out.push_str("[redacted]");
        } else {
            out.push_str(run);
        }
        run.clear();
    };
    for c in text.chars() {
        if is_token(c) {
            run.push(c);
        } else {
            flush(&mut run, &mut out);
            out.push(c);
        }
    }
    flush(&mut run, &mut out);
    out
}

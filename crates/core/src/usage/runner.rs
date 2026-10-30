//! Runs provider commands: argv only, scrubbed environment, time limit, output caps (research R5).

use std::io::{Read, Write};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

pub const STDOUT_CAP: usize = 64 * 1024;
pub const STDERR_CAP: usize = 4 * 1024;
/// Variables passed through from Sonata's own environment. Everything else is dropped.
#[cfg(unix)]
pub const ENV_ALLOWLIST: &[&str] = &["HOME", "USER", "LOGNAME", "LANG", "TMPDIR", "PATH"];
/// Windows programs need the system folders and the profile variables (feature 012, R11).
#[cfg(windows)]
pub const ENV_ALLOWLIST: &[&str] = &[
    "USERPROFILE",
    "USERNAME",
    "APPDATA",
    "LOCALAPPDATA",
    "TEMP",
    "TMP",
    "PATH",
    "PATHEXT",
    "SystemRoot",
    "ComSpec",
    "HOMEDRIVE",
    "HOMEPATH",
];

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

/// Spawns `cmd`, retrying briefly when Linux reports the program busy (`ETXTBSY`): a script
/// written a moment ago can still be open for writing in a process forked by another thread.
fn spawn_retrying(cmd: &mut Command) -> std::io::Result<std::process::Child> {
    let mut tries = 0;
    loop {
        match cmd.spawn() {
            Err(e) if e.kind() == std::io::ErrorKind::ExecutableFileBusy && tries < 10 => {
                tries += 1;
                std::thread::sleep(std::time::Duration::from_millis(20));
            }
            result => return result,
        }
    }
}

/// Runs `spec` in a new process group (a Job Object on Windows) inside an empty temporary
/// folder. The whole group is killed when the command exits, times out, overflows, or meets
/// `stop_when`, so no child process outlives the run.
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
        .stdin(if spec.stdin.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    group::prepare(&mut cmd);
    for key in ENV_ALLOWLIST {
        // As seen by user programs: an AppImage's own paths are left out (feature 011, R6).
        if let Some(value) = crate::platform::host_var(key) {
            cmd.env(key, value);
        }
    }
    for (key, value) in &spec.env {
        cmd.env(key, value);
    }
    for key in crate::terminal::pty::AGENT_SESSION_MARKERS {
        cmd.env_remove(key);
    }
    let mut child = spawn_retrying(&mut cmd).map_err(|e| match e.kind() {
        std::io::ErrorKind::NotFound => RunError::NotFound,
        _ => RunError::Spawn(e.to_string()),
    })?;
    let group = group::Group::new(&child);

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
        if group.has_exited(&mut child) {
            break Ok(());
        }
        if start.elapsed() >= spec.timeout {
            break Err(RunError::TimedOut(spec.timeout));
        }
        std::thread::sleep(POLL);
    };
    group.kill();
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

#[cfg(unix)]
mod group {
    use std::os::unix::process::CommandExt;
    use std::process::{Child, Command};

    pub fn prepare(cmd: &mut Command) {
        cmd.process_group(0);
    }

    /// The child's process group; its id is the leader's pid.
    pub struct Group(i32);

    impl Group {
        pub fn new(child: &Child) -> Self {
            Self(child.id() as i32)
        }

        /// True once the process has exited, without reaping it.
        pub fn has_exited(&self, _child: &mut Child) -> bool {
            let mut info: libc::siginfo_t = unsafe { std::mem::zeroed() };
            let flags = libc::WEXITED | libc::WNOHANG | libc::WNOWAIT;
            let r = unsafe { libc::waitid(libc::P_PID, self.0 as libc::id_t, &mut info, flags) };
            r == 0 && exited_pid(&info) != 0
        }

        pub fn kill(&self) {
            // The leader is not reaped yet, so its pid still names this group.
            unsafe {
                libc::killpg(self.0, libc::SIGKILL);
            }
        }
    }

    /// `si_pid` is a plain field on macOS but an accessor method in the Linux `libc` bindings.
    #[cfg(target_os = "linux")]
    fn exited_pid(info: &libc::siginfo_t) -> libc::pid_t {
        // SAFETY: `waitid` with `WEXITED` fills the child-status variant of the union.
        unsafe { info.si_pid() }
    }

    #[cfg(not(target_os = "linux"))]
    fn exited_pid(info: &libc::siginfo_t) -> libc::pid_t {
        info.si_pid
    }
}

/// A Job Object that kills every process in it when it closes (feature 012, research R11).
#[cfg(windows)]
mod group {
    use std::os::windows::io::AsRawHandle;
    use std::os::windows::process::CommandExt;
    use std::process::{Child, Command};

    use windows_sys::Win32::Foundation::{CloseHandle, HANDLE};
    use windows_sys::Win32::System::JobObjects::{
        AssignProcessToJobObject, CreateJobObjectW, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
        JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JobObjectExtendedLimitInformation,
        SetInformationJobObject, TerminateJobObject,
    };
    use windows_sys::Win32::System::Threading::CREATE_NO_WINDOW;

    pub fn prepare(cmd: &mut Command) {
        cmd.creation_flags(CREATE_NO_WINDOW);
    }

    pub struct Group(HANDLE);

    impl Group {
        pub fn new(child: &Child) -> Self {
            // SAFETY: null name and attributes create an unnamed job; failures leave null.
            let job = unsafe { CreateJobObjectW(std::ptr::null(), std::ptr::null()) };
            if !job.is_null() {
                let mut info = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
                info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
                // SAFETY: `info` has the size passed; the child handle is valid while it runs.
                unsafe {
                    SetInformationJobObject(
                        job,
                        JobObjectExtendedLimitInformation,
                        (&info as *const JOBOBJECT_EXTENDED_LIMIT_INFORMATION).cast(),
                        size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
                    );
                    AssignProcessToJobObject(job, child.as_raw_handle() as HANDLE);
                }
            }
            Self(job)
        }

        pub fn has_exited(&self, child: &mut Child) -> bool {
            // A handle keeps the exit code; checking it does not free the pid for reuse.
            matches!(child.try_wait(), Ok(Some(_)))
        }

        pub fn kill(&self) {
            if !self.0.is_null() {
                // SAFETY: the job handle is owned.
                unsafe { TerminateJobObject(self.0, 1) };
            }
        }
    }

    impl Drop for Group {
        fn drop(&mut self) {
            if !self.0.is_null() {
                // SAFETY: closed once; closing also kills what is left (KILL_ON_JOB_CLOSE).
                unsafe { CloseHandle(self.0) };
            }
        }
    }
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

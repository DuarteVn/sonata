//! Kimi Code CLI integration: the Sonata hook block in Kimi's config file (feature 010,
//! research R4, contracts/kimi-config.md).
//!
//! Kimi reads hooks only from `$KIMI_CODE_HOME/config.toml`. Sonata changes that file only when
//! the user confirms Apply or Remove, and only adds or removes its own `[[hooks]]` tables. The
//! edit is line based: a table is Sonata's when its `command` calls `sonata _hook kimi`.

use std::io::Read;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use serde::Serialize;

use crate::error::{AppError, AppResult, ErrorCode};
use crate::paths::FILE_MODE;
use crate::persistence::atomic;

pub const KIMI_BLOCK_VERSION: u32 = 1;

/// The exact lines Apply appends (block version 1).
pub const BLOCK_V1: &str = r#"# >>> Sonata integration (v1). Managed by Sonata: Settings > Agents > Kimi Code. >>>
# These hooks only act inside Sonata terminals; elsewhere they exit at once.
[[hooks]]
event = "SessionStart"
command = "test -n \"$SONATA_TERMINAL_ID\" && sonata _hook kimi session-start; exit 0"
timeout = 5

[[hooks]]
event = "UserPromptSubmit"
command = "test -n \"$SONATA_TERMINAL_ID\" && sonata _hook kimi prompt-submit; exit 0"
timeout = 5

[[hooks]]
event = "Stop"
command = "test -n \"$SONATA_TERMINAL_ID\" && sonata _hook kimi stop; exit 0"
timeout = 5

[[hooks]]
event = "StopFailure"
command = "test -n \"$SONATA_TERMINAL_ID\" && sonata _hook kimi stop-failure; exit 0"
timeout = 5

[[hooks]]
event = "Interrupt"
command = "test -n \"$SONATA_TERMINAL_ID\" && sonata _hook kimi interrupt; exit 0"
timeout = 5
# <<< Sonata integration <<<
"#;

/// A `[[hooks]]` table is Sonata's when its command contains this.
const MARK: &str = "sonata _hook kimi";
const MARKER_LINES: [&str; 3] = [
    "# >>> Sonata integration",
    "# <<< Sonata integration",
    "# These hooks only act inside Sonata terminals",
];
const BACKUP_PREFIX: &str = "config.toml.sonata-backup-";
const BACKUPS_KEPT: usize = 3;
/// Limit for `kimi doctor` (research R4).
pub const DOCTOR_TIMEOUT: Duration = Duration::from_secs(10);
const SHELL_TIMEOUT: Duration = Duration::from_secs(2);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum KimiStatus {
    On,
    Outdated,
    Off,
    Unavailable,
}

/// What the Settings view and the terminal notice show (data-model.md).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KimiIntegration {
    pub status: KimiStatus,
    pub config_path: PathBuf,
    pub block: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    pub can_apply: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_backup: Option<PathBuf>,
    pub validated: bool,
}

// ---- Line scanner ----

/// One `[[hooks]]` table: its header line, its last key line, and its keys as raw text.
struct HookTable {
    start: usize,
    last: usize,
    keys: Vec<(String, String)>,
}

impl HookTable {
    fn get(&self, key: &str) -> Option<&str> {
        self.keys
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
    }

    fn is_sonata(&self) -> bool {
        self.get("command").is_some_and(|c| c.contains(MARK))
    }

    /// `event`, `command`, `timeout` as written, for comparison with the current block.
    fn entry(&self) -> (String, String, String) {
        let v = |k| self.get(k).unwrap_or_default().to_string();
        (v("event"), v("command"), v("timeout"))
    }
}

/// `[name]` → `(false, name)`, `[[name]]` → `(true, name)`; anything else is not a header.
fn header(line: &str) -> Option<(bool, String)> {
    let t = line.trim();
    if let Some(inner) = t.strip_prefix("[[") {
        let end = inner.find("]]")?;
        return Some((true, inner[..end].trim().to_string()));
    }
    let inner = t.strip_prefix('[')?;
    let end = inner.find(']')?;
    Some((false, inner[..end].trim().to_string()))
}

fn key_value(line: &str) -> Option<(String, String)> {
    let t = line.trim();
    if t.starts_with('#') || t.is_empty() {
        return None;
    }
    let (k, v) = t.split_once('=')?;
    Some((k.trim().trim_matches('"').to_string(), v.trim().to_string()))
}

fn hook_tables(lines: &[&str]) -> Vec<HookTable> {
    let mut tables = Vec::new();
    let mut current: Option<HookTable> = None;
    for (i, line) in lines.iter().enumerate() {
        if let Some((array, name)) = header(line) {
            tables.extend(current.take());
            if array && name == "hooks" {
                current = Some(HookTable {
                    start: i,
                    last: i,
                    keys: Vec::new(),
                });
            }
        } else if let Some(t) = current.as_mut()
            && let Some(kv) = key_value(line)
        {
            t.last = i;
            t.keys.push(kv);
        }
    }
    tables.extend(current);
    tables
}

fn is_marker(line: &str) -> bool {
    let t = line.trim();
    MARKER_LINES.iter().any(|m| t.starts_with(m))
}

fn entries(text: &str) -> Vec<(String, String, String)> {
    let lines: Vec<&str> = text.split('\n').collect();
    let mut v: Vec<_> = hook_tables(&lines)
        .iter()
        .filter(|t| t.is_sonata())
        .map(HookTable::entry)
        .collect();
    v.sort();
    v
}

/// On, out of date, or off, from the file's text (FR-007).
pub fn status(text: &str) -> KimiStatus {
    let found = entries(text);
    if found.is_empty() {
        KimiStatus::Off
    } else if found == entries(BLOCK_V1) {
        KimiStatus::On
    } else {
        KimiStatus::Outdated
    }
}

/// Why `[[hooks]]` cannot be added: the file already defines `hooks` in another form.
pub fn conflict(text: &str) -> Option<String> {
    let mut in_table = false;
    for (i, line) in text.split('\n').enumerate() {
        let found = match header(line) {
            Some((array, name)) => {
                in_table = true;
                !array && (name == "hooks" || name.starts_with("hooks."))
            }
            None => !in_table && key_value(line).is_some_and(|(k, _)| k == "hooks"),
        };
        if found {
            return Some(format!(
                "Kimi's config defines `hooks` on line {} (`{}`), so Sonata cannot add its \
                 `[[hooks]]` entries. Change that line to `[[hooks]]` tables first.",
                i + 1,
                line.trim()
            ));
        }
    }
    None
}

/// The text without Sonata's tables and marker lines. Unchanged when it has none.
pub fn without_block(text: &str) -> String {
    let lines: Vec<&str> = text.split('\n').collect();
    let mut drop = vec![false; lines.len()];
    let mut any = false;
    for t in hook_tables(&lines).iter().filter(|t| t.is_sonata()) {
        drop[t.start..=t.last].iter_mut().for_each(|d| *d = true);
        any = true;
    }
    for (i, line) in lines.iter().enumerate() {
        if is_marker(line) {
            drop[i] = true;
            any = true;
        }
    }
    if !any {
        return text.to_string();
    }
    let mut kept: Vec<&str> = lines
        .iter()
        .zip(&drop)
        .filter(|(_, d)| !**d)
        .map(|(l, _)| *l)
        .collect();
    while kept.last().is_some_and(|l| l.trim().is_empty()) {
        kept.pop();
    }
    if kept.is_empty() {
        String::new()
    } else {
        format!("{}\n", kept.join("\n"))
    }
}

/// The text with the current block at the end, after one blank line.
pub fn with_block(text: &str) -> AppResult<String> {
    if let Some(msg) = conflict(text) {
        return Err(AppError::new(ErrorCode::KimiHooksConflict, msg));
    }
    let base = without_block(text);
    if base.is_empty() {
        return Ok(BLOCK_V1.to_string());
    }
    let newline = if base.ends_with('\n') { "" } else { "\n" };
    Ok(format!("{base}{newline}\n{BLOCK_V1}"))
}

/// `(event, command)` of each table in the current block, with the command unescaped as Kimi
/// reads it before running it with `/bin/sh -c`.
pub fn block_commands() -> Vec<(String, String)> {
    let lines: Vec<&str> = BLOCK_V1.split('\n').collect();
    hook_tables(&lines)
        .iter()
        .map(|t| {
            (
                unquote(t.get("event").unwrap_or_default()),
                unquote(t.get("command").unwrap_or_default()),
            )
        })
        .collect()
}

/// A TOML basic string's value (the escapes the block uses).
fn unquote(raw: &str) -> String {
    let inner = raw
        .strip_prefix('"')
        .and_then(|s| s.strip_suffix('"'))
        .unwrap_or(raw);
    let mut out = String::new();
    let mut chars = inner.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('n') => out.push('\n'),
                Some('t') => out.push('\t'),
                Some(other) => out.push(other),
                None => {}
            }
        } else {
            out.push(c);
        }
    }
    out
}

// ---- Login shell ----

struct ShellRun {
    code: Option<i32>,
    output: String,
}

/// Runs `shell -l -c script` in its own process group; `None` on timeout (the group is killed).
fn run_shell(
    shell: &Path,
    script: &str,
    env: &[(&str, &Path)],
    timeout: Duration,
) -> Option<ShellRun> {
    let mut cmd = Command::new(shell);
    cmd.args(["-l", "-c", script])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .process_group(0);
    for (k, v) in env {
        cmd.env(k, v);
    }
    let mut child = cmd.spawn().ok()?;
    let mut stdout = child.stdout.take()?;
    let reader = std::thread::spawn(move || {
        let mut s = String::new();
        let _ = stdout.read_to_string(&mut s);
        s
    });
    let start = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if start.elapsed() < timeout => std::thread::sleep(Duration::from_millis(20)),
            _ => {
                if let Ok(pid) = libc::pid_t::try_from(child.id()) {
                    // SAFETY: killpg only sends a signal to the group this function created.
                    unsafe {
                        libc::killpg(pid, libc::SIGKILL);
                    }
                }
                let _ = child.wait();
                return None;
            }
        }
    };
    let output = reader.join().unwrap_or_default();
    Some(ShellRun {
        code: status.code(),
        output,
    })
}

/// Kimi's folder: `KIMI_CODE_HOME` from the user's login shell, else `~/.kimi-code`.
pub fn kimi_home(shell: &Path) -> PathBuf {
    run_shell(shell, "printf %s \"$KIMI_CODE_HOME\"", &[], SHELL_TIMEOUT)
        .map(|r| r.output.trim().to_string())
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            std::env::var_os("HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("/"))
                .join(".kimi-code")
        })
}

enum Doctor {
    /// `kimi` is not on the login shell's PATH: nothing was checked.
    NotFound,
    Passed,
    Failed(String),
}

fn doctor(home: &Path, shell: &Path, timeout: Duration) -> Doctor {
    let script = "command -v kimi >/dev/null 2>&1 || exit 127\nexec kimi doctor 2>&1";
    match run_shell(shell, script, &[("KIMI_CODE_HOME", home)], timeout) {
        Some(r) if r.code == Some(127) => Doctor::NotFound,
        Some(r) if r.code == Some(0) => Doctor::Passed,
        Some(r) => Doctor::Failed(tail(&r.output)),
        None => Doctor::Failed(format!(
            "`kimi doctor` did not finish within {} seconds.",
            timeout.as_secs_f32()
        )),
    }
}

fn kimi_found(shell: &Path) -> bool {
    run_shell(shell, "command -v kimi >/dev/null 2>&1", &[], SHELL_TIMEOUT)
        .is_some_and(|r| r.code == Some(0))
}

/// The last part of a long message.
fn tail(s: &str) -> String {
    let s = s.trim();
    let start = s.char_indices().rev().nth(800).map_or(0, |(i, _)| i);
    s[start..].to_string()
}

// ---- Backups ----

/// `(stamp, n)` of a Sonata backup name, for ordering.
fn backup_key(name: &str) -> Option<(String, u32)> {
    let rest = name.strip_prefix(BACKUP_PREFIX)?;
    let (stamp, n) = match rest.get(15..) {
        Some("") => (rest, 0),
        Some(suffix) => (&rest[..15], suffix.strip_prefix('-')?.parse().ok()?),
        None => return None,
    };
    let b = stamp.as_bytes();
    let ok = b.len() == 15
        && b[8] == b'-'
        && b.iter()
            .enumerate()
            .all(|(i, c)| i == 8 || c.is_ascii_digit());
    ok.then(|| (stamp.to_string(), n))
}

fn backups(home: &Path) -> Vec<((String, u32), PathBuf)> {
    let mut v: Vec<_> = std::fs::read_dir(home)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            backup_key(&name).map(|k| (k, e.path()))
        })
        .collect();
    v.sort();
    v
}

/// Copies the file next to itself with the same mode, then keeps only the newest backups.
fn write_backup(home: &Path, text: &str, mode: u32) -> AppResult<PathBuf> {
    let stamp = chrono::Local::now().format("%Y%m%d-%H%M%S").to_string();
    // Within one second, each backup gets a higher number than any before it, so the newest is
    // always last in order even after older ones were deleted.
    let next = backups(home)
        .iter()
        .filter(|((s, _), _)| *s == stamp)
        .map(|((_, n), _)| n + 1)
        .max()
        .unwrap_or(0);
    let suffix = if next == 0 {
        String::new()
    } else {
        format!("-{next}")
    };
    let path = home.join(format!("{BACKUP_PREFIX}{stamp}{suffix}"));
    write_keeping_mode(&path, text, mode)?;
    let all = backups(home);
    for (_, old) in all.iter().take(all.len().saturating_sub(BACKUPS_KEPT)) {
        let _ = std::fs::remove_file(old);
    }
    Ok(path)
}

// ---- Status and changes ----

fn config_path(home: &Path) -> PathBuf {
    home.join("config.toml")
}

fn missing_folder(home: &Path) -> String {
    format!(
        "Kimi's folder {} does not exist. Run Kimi once first, then set up the integration.",
        home.display()
    )
}

/// Current status of Sonata's integration (FR-007). Reads the file; never changes it.
pub fn integration(home: &Path, shell: &Path) -> KimiIntegration {
    let path = config_path(home);
    let mut out = KimiIntegration {
        status: KimiStatus::Unavailable,
        config_path: path.clone(),
        block: BLOCK_V1,
        message: None,
        can_apply: false,
        last_backup: backups(home).pop().map(|(_, p)| p),
        validated: false,
    };
    if !home.is_dir() {
        out.message = Some(missing_folder(home));
        return out;
    }
    let text = match std::fs::read_to_string(&path) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(e) => {
            out.message = Some(format!("Sonata cannot read {}: {e}", path.display()));
            return out;
        }
    };
    out.status = status(&text);
    out.message = conflict(&text);
    out.can_apply = out.message.is_none();
    out.validated = kimi_found(shell);
    out
}

/// Adds Sonata's block (user-confirmed Apply, FR-010).
pub fn apply(home: &Path, shell: &Path) -> AppResult<KimiIntegration> {
    apply_with(home, shell, DOCTOR_TIMEOUT)
}

/// [`apply`] with a custom `kimi doctor` limit (tests).
pub fn apply_with(home: &Path, shell: &Path, timeout: Duration) -> AppResult<KimiIntegration> {
    change(home, shell, timeout, with_block)
}

/// Removes Sonata's block (user-confirmed Remove, FR-010).
pub fn remove(home: &Path, shell: &Path) -> AppResult<KimiIntegration> {
    change(home, shell, DOCTOR_TIMEOUT, |t| Ok(without_block(t)))
}

fn change(
    home: &Path,
    shell: &Path,
    timeout: Duration,
    edit: impl FnOnce(&str) -> AppResult<String>,
) -> AppResult<KimiIntegration> {
    if !home.is_dir() {
        return Err(AppError::new(
            ErrorCode::KimiUnavailable,
            missing_folder(home),
        ));
    }
    let path = config_path(home);
    let (old, mode) = match std::fs::read_to_string(&path) {
        Ok(t) => {
            let mode = std::fs::metadata(&path)?.permissions().mode() & 0o777;
            (Some(t), mode)
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => (None, FILE_MODE),
        Err(e) => {
            return Err(AppError::new(
                ErrorCode::KimiUnavailable,
                format!("Sonata cannot read {}: {e}", path.display()),
            ));
        }
    };
    let next = edit(old.as_deref().unwrap_or_default())?;
    if old.as_deref() == Some(next.as_str()) || (old.is_none() && next.is_empty()) {
        return Ok(integration(home, shell));
    }

    // A file that is already invalid is reported, not changed (research R4).
    if old.is_some()
        && let Doctor::Failed(msg) = doctor(home, shell, timeout)
    {
        return Err(AppError::new(
            ErrorCode::KimiConfigInvalid,
            format!(
                "Kimi reports a problem in {} before any change, so Sonata changed nothing:\n{msg}",
                path.display()
            ),
        ));
    }
    if let Some(text) = &old {
        write_backup(home, text, mode)?;
    }
    write_keeping_mode(&path, &next, mode)?;

    if let Doctor::Failed(msg) = doctor(home, shell, timeout) {
        match &old {
            Some(text) => write_keeping_mode(&path, text, mode)?,
            None => std::fs::remove_file(&path)?,
        }
        return Err(AppError::new(
            ErrorCode::KimiCheckFailed,
            format!(
                "Kimi rejected the changed {}, so Sonata put the previous file back:\n{msg}",
                path.display()
            ),
        ));
    }
    Ok(integration(home, shell))
}

/// Atomic write; the file then gets `mode` back (it can hold API keys).
fn write_keeping_mode(path: &Path, text: &str, mode: u32) -> AppResult<()> {
    atomic::write(path, text.as_bytes())?;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scanner_splits_tables_and_finds_sonata_ones() {
        let text = "a = 1\n[[hooks]]\nevent = \"Stop\"\ncommand = \"x\"\n\n# note\n[[hooks]]\n\
                    event = \"Stop\"\ncommand = \"sonata _hook kimi stop\"\ntimeout = 5\n\n[tui]\nk = 1\n";
        let lines: Vec<&str> = text.split('\n').collect();
        let t = hook_tables(&lines);
        assert_eq!(t.len(), 2);
        assert!(!t[0].is_sonata() && t[1].is_sonata());
        assert_eq!((t[1].start, t[1].last), (6, 9));
        assert_eq!(t[0].last, 3, "comments and blank lines are not table keys");
    }

    #[test]
    fn marker_lines_belong_to_the_block() {
        assert!(is_marker(
            "# >>> Sonata integration (v1). Managed by Sonata"
        ));
        assert!(is_marker("  # <<< Sonata integration <<<"));
        assert!(!is_marker("# my own comment"));
    }

    #[test]
    fn conflicts_are_top_level_hooks_keys_and_hooks_tables() {
        assert!(conflict("hooks = []\n").is_some());
        assert!(conflict("[hooks]\n").is_some());
        assert!(conflict("[hooks.extra]\n").is_some());
        assert!(
            conflict("[tui]\nhooks = 1\n").is_none(),
            "tui.hooks is another key"
        );
        assert!(conflict("[[hooks]]\nevent = \"Stop\"\n").is_none());
    }

    #[test]
    fn backup_names_parse() {
        assert_eq!(
            backup_key("config.toml.sonata-backup-20261026-120000"),
            Some(("20261026-120000".into(), 0))
        );
        assert_eq!(
            backup_key("config.toml.sonata-backup-20261026-120000-12"),
            Some(("20261026-120000".into(), 12))
        );
        assert_eq!(backup_key("config.toml.sonata-backup-x"), None);
        assert_eq!(backup_key("config.toml.bak"), None);
    }
}

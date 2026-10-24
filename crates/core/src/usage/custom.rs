//! Custom providers from definition files in the usage providers folder (research R4, R5).
//!
//! A definition names a program and its arguments. The program prints Sonata's normalized
//! usage JSON. It runs without a shell, with a 10-second limit, inside a sandbox that blocks
//! credential stores and Sonata's own data (FR-019).

use std::collections::HashMap;
use std::io;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::time::Duration;

use chrono::{DateTime, TimeZone, Utc};
use serde::Deserialize;
use serde_json::Value;

use super::runner::{RunError, RunSpec, redact, run};
use super::service::{BUILT_INS, ProviderInfo};
use super::{Detection, ProviderKind, Reading, ReadingStatus, Window, WindowKind};
use crate::persistence::envelope;
use crate::persistence::migrate::Migrations;

pub const DEFINITION_TYPE: &str = "usageProvider";
pub const DEFINITION_SCHEMA_VERSION: u32 = 1;
pub const TIMEOUT: Duration = Duration::from_secs(10);
pub const SANDBOX_EXEC: &str = "/usr/bin/sandbox-exec";
pub const DEFAULT_COLOR: &str = "#8b93a7";
const MAX_ARGS: usize = 17;
const MAX_ARG_BYTES: usize = 1024;
const MAX_WINDOWS: usize = 4;

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Definition {
    pub id: String,
    pub name: String,
    pub command: Vec<String>,
    #[serde(default)]
    pub color: Option<String>,
}

fn valid_id(id: &str) -> bool {
    let mut chars = id.chars();
    id.len() <= 32
        && chars
            .next()
            .is_some_and(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

fn is_executable_file(path: &Path) -> bool {
    std::fs::metadata(path).is_ok_and(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
}

pub fn validate(def: &Definition) -> Result<(), String> {
    if !valid_id(&def.id) {
        return Err(
            "id must be 1 to 32 lowercase letters, digits, or dashes, starting with a letter \
             or digit"
                .into(),
        );
    }
    if BUILT_INS.iter().any(|(id, _, _)| *id == def.id) {
        return Err(format!("id '{}' belongs to a built-in agent", def.id));
    }
    let name_len = def.name.chars().count();
    if name_len == 0 || name_len > 40 {
        return Err("name must be 1 to 40 characters".into());
    }
    if def.command.is_empty() || def.command.len() > MAX_ARGS {
        return Err(format!("command must have 1 to {MAX_ARGS} items"));
    }
    if def.command.iter().any(|a| a.contains('\0')) {
        return Err("command items must not contain NUL".into());
    }
    if def.command.iter().any(|a| a.len() > MAX_ARG_BYTES) {
        return Err("each command item must be at most 1 KiB".into());
    }
    let program = Path::new(&def.command[0]);
    if !program.is_absolute() {
        return Err("command[0] must be an absolute path".into());
    }
    if !is_executable_file(program) {
        return Err(format!(
            "command[0] is not an executable file: {}",
            program.display()
        ));
    }
    if let Some(color) = &def.color
        && !super::service::valid_color(color)
    {
        return Err("color must look like #3b82f6".into());
    }
    Ok(())
}

fn load_file(path: &Path) -> Result<Definition, String> {
    let bytes =
        crate::persistence::atomic::read_capped(path, 64 * 1024).map_err(|e| e.to_string())?;
    let def: Definition = envelope::decode(
        &bytes,
        DEFINITION_TYPE,
        &Migrations::new(DEFINITION_SCHEMA_VERSION),
    )
    .map_err(|e| e.to_string())?;
    validate(&def)?;
    Ok(def)
}

fn explanation(file: &str) -> String {
    format!(
        "Sonata runs this command from {file} every minute while usage reading is on. It runs \
         without a shell, stops after 10 seconds, and cannot read your keychain, agent login \
         files, or Sonata's data."
    )
}

/// Every `*.json` file in `dir`, sorted by file name: valid ones as custom providers, invalid
/// ones as rows that carry the error. A missing folder has none.
pub fn provider_infos(dir: &Path) -> Vec<ProviderInfo> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut files: Vec<PathBuf> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "json") && p.is_file())
        .collect();
    files.sort();
    let loaded: Vec<(String, Result<Definition, String>)> = files
        .iter()
        .map(|p| {
            let name = p
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            (name, load_file(p))
        })
        .collect();
    let mut owners: HashMap<&str, Vec<&str>> = HashMap::new();
    for (file, def) in &loaded {
        if let Ok(def) = def {
            owners
                .entry(def.id.as_str())
                .or_default()
                .push(file.as_str());
        }
    }
    loaded
        .iter()
        .map(|(file, def)| {
            let def = def.clone().and_then(|d| {
                match owners[d.id.as_str()].iter().find(|f| **f != file.as_str()) {
                    Some(other) => Err(format!("id '{}' is also used by {other}", d.id)),
                    None => Ok(d),
                }
            });
            match def {
                Ok(d) => ProviderInfo {
                    id: d.id,
                    name: d.name,
                    kind: ProviderKind::Custom,
                    default_color: d
                        .color
                        .map(|c| c.to_ascii_lowercase())
                        .unwrap_or_else(|| DEFAULT_COLOR.into()),
                    detection: Detection::Found,
                    command_path: Some(PathBuf::from(&d.command[0])),
                    consent_key: d.command.clone(),
                    command: d.command,
                    explanation: explanation(file),
                    definition_error: None,
                },
                Err(error) => ProviderInfo {
                    id: format!("file:{file}"),
                    name: file.clone(),
                    kind: ProviderKind::Custom,
                    default_color: DEFAULT_COLOR.into(),
                    detection: Detection::NotFound,
                    command_path: None,
                    consent_key: Vec::new(),
                    command: Vec::new(),
                    explanation: String::new(),
                    definition_error: Some(error),
                },
            }
        })
        .collect()
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
enum OutStatus {
    Ok,
    NotLoggedIn,
    NoPlanLimits,
    Unavailable,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct OutWindow {
    kind: WindowKind,
    #[serde(default)]
    label: Option<String>,
    used_percent: f64,
    #[serde(default)]
    resets_at: Option<Value>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Output {
    #[serde(default)]
    status: Option<OutStatus>,
    #[serde(default)]
    message: Option<String>,
    #[serde(default)]
    plan: Option<String>,
    #[serde(default)]
    account: Option<String>,
    #[serde(default)]
    windows: Vec<OutWindow>,
}

fn check_len(field: &str, value: &Option<String>, max: usize) -> Result<(), String> {
    match value {
        Some(v) if v.chars().count() > max => {
            Err(format!("{field} is longer than {max} characters"))
        }
        _ => Ok(()),
    }
}

fn reset_time(v: &Value) -> Result<DateTime<Utc>, String> {
    let parsed = match v {
        Value::String(s) => DateTime::parse_from_rfc3339(s)
            .ok()
            .map(|t| t.with_timezone(&Utc)),
        Value::Number(n) => n.as_i64().and_then(|s| Utc.timestamp_opt(s, 0).single()),
        _ => None,
    };
    parsed.ok_or_else(|| "resetsAt must be an RFC 3339 time or Unix seconds".into())
}

fn to_reading(id: &str, out: Output, now: DateTime<Utc>) -> Result<Reading, String> {
    check_len("message", &out.message, 200)?;
    check_len("plan", &out.plan, 40)?;
    check_len("account", &out.account, 40)?;
    let status = match out.status.unwrap_or(OutStatus::Ok) {
        OutStatus::Ok => ReadingStatus::Ok,
        OutStatus::NotLoggedIn => ReadingStatus::NotLoggedIn,
        OutStatus::NoPlanLimits => ReadingStatus::NoPlanLimits,
        OutStatus::Unavailable => ReadingStatus::Unavailable,
    };
    let mut r = Reading::new(id, status, now);
    r.plan = out.plan;
    r.account = out.account;
    r.message = out.message;
    if status != ReadingStatus::Ok {
        return Ok(r);
    }
    if out.windows.is_empty() || out.windows.len() > MAX_WINDOWS {
        return Err(format!("an ok reading needs 1 to {MAX_WINDOWS} windows"));
    }
    for w in out.windows {
        check_len("label", &w.label, 24)?;
        if !w.used_percent.is_finite() || w.used_percent < 0.0 {
            return Err("usedPercent must be a number of 0 or more".into());
        }
        let label = w.label.unwrap_or_else(|| {
            match w.kind {
                WindowKind::Short => "Short window",
                WindowKind::Weekly => "Weekly",
                WindowKind::Other => "Limit",
            }
            .into()
        });
        r.windows.push(Window {
            kind: w.kind,
            label,
            used_percent: w.used_percent,
            resets_at: w.resets_at.as_ref().map(reset_time).transpose()?,
        });
    }
    Ok(r)
}

/// Turns a custom command's stdout into a reading; invalid output is `Unavailable` with why.
pub fn parse_output(id: &str, stdout: &str, now: DateTime<Utc>) -> Reading {
    let out: Output = match serde_json::from_str(stdout.trim()) {
        Ok(out) => out,
        Err(e) => {
            return Reading::unavailable(id, now, format!("printed invalid usage JSON: {e}"));
        }
    };
    to_reading(id, out, now)
        .unwrap_or_else(|e| Reading::unavailable(id, now, format!("printed invalid usage: {e}")))
}

/// Runs `argv` (wrapped in `sandbox-exec` when `sandbox` is given) and parses its output.
pub fn run_custom(
    id: &str,
    argv: &[String],
    env: &[(String, String)],
    sandbox: Option<(&Path, &str)>,
    timeout: Duration,
    now: DateTime<Utc>,
) -> Reading {
    let full: Vec<String> = match sandbox {
        Some((exec, profile)) => [exec.display().to_string(), "-p".into(), profile.into()]
            .into_iter()
            .chain(argv.iter().cloned())
            .collect(),
        None => argv.to_vec(),
    };
    let mut spec = RunSpec::new(full, timeout);
    spec.env = env.to_vec();
    match run(&spec) {
        Ok(out) if out.code == Some(0) => {
            parse_output(id, &String::from_utf8_lossy(&out.stdout), now)
        }
        Ok(out) => {
            if !out.stderr.trim().is_empty() {
                tracing::info!(
                    provider = id,
                    "custom usage stderr: {}",
                    redact(out.stderr.trim())
                );
            }
            let reason = match out.code {
                Some(code) => format!("exited with status {code}"),
                None => "was stopped by a signal".into(),
            };
            Reading::unavailable(id, now, reason)
        }
        Err(RunError::NotFound) => Reading::unavailable(id, now, "Command not found."),
        Err(e) => Reading::unavailable(id, now, e.to_string()),
    }
}

/// Reads a custom provider in the sandbox (see [`sandbox_profile`]).
pub fn read_custom(
    id: &str,
    argv: &[String],
    env: &[(String, String)],
    home: &Path,
    sonata_root: &Path,
    now: DateTime<Utc>,
) -> Reading {
    read_custom_with(
        Path::new(SANDBOX_EXEC),
        id,
        argv,
        env,
        home,
        sonata_root,
        now,
    )
}

/// Like [`read_custom`] with a given `sandbox-exec` path. Without it, nothing runs.
pub fn read_custom_with(
    sandbox_exec: &Path,
    id: &str,
    argv: &[String],
    env: &[(String, String)],
    home: &Path,
    sonata_root: &Path,
    now: DateTime<Utc>,
) -> Reading {
    if !is_executable_file(sandbox_exec) {
        return Reading::unavailable(id, now, "Cannot run safely: sandbox-exec not found.");
    }
    let profile = sandbox_profile(home, sonata_root);
    run_custom(id, argv, env, Some((sandbox_exec, &profile)), TIMEOUT, now)
}

/// The sandbox matches real paths (for example `/private/var`), so paths are canonicalized.
fn real(path: &Path) -> String {
    let p = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    p.display()
        .to_string()
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
}

/// Allows everything except credential stores and Sonata's data. The providers folder stays
/// readable so helper scripts can live next to their definition.
pub fn sandbox_profile(home: &Path, sonata_root: &Path) -> String {
    let home = real(home);
    let root = real(sonata_root);
    format!(
        "(version 1)\n\
         (allow default)\n\
         (deny file-read* file-write*\n  \
           (subpath \"{home}/Library/Keychains\")\n  \
           (subpath \"/Library/Keychains\")\n  \
           (literal \"{home}/.claude/.credentials.json\")\n  \
           (literal \"{home}/.codex/auth.json\")\n  \
           (subpath \"{root}\"))\n\
         (deny mach-lookup (global-name-regex #\"^com\\.apple\\.secur\") (global-name \"com.apple.SecurityServer\"))\n\
         (allow file-read* (subpath \"{root}/usage-providers\"))\n"
    )
}

/// Creates the providers folder (private) and writes the guide when it is missing. An
/// existing guide is never overwritten, so user edits stay.
pub fn ensure_folder(dir: &Path) -> io::Result<()> {
    crate::paths::ensure_private_dir(dir)?;
    let guide = dir.join("README.md");
    if !guide.exists() {
        crate::persistence::atomic::write(&guide, GUIDE.as_bytes())?;
    }
    Ok(())
}

pub const GUIDE: &str = r##"# Sonata usage providers

Sonata shows the plan limits of Claude Code and Codex on its own. To show another agent, put a
definition file in this folder. Sonata reads the folder at start and when you press
"Refresh all" in Settings → Usage.

## Definition file

Any file name ending in `.json`. One agent per file.

```json
{
  "type": "usageProvider",
  "schemaVersion": 1,
  "payload": {
    "id": "my-agent",
    "name": "My Agent",
    "command": ["/Users/me/bin/my-agent-usage", "--json"],
    "color": "#3b82f6"
  }
}
```

| Field | Rule |
|---|---|
| `id` | 1 to 32 characters: lowercase letters, digits, dashes; starts with a letter or digit. Not `claude` or `codex`. Unique in this folder. |
| `name` | 1 to 40 characters, shown in the panel. |
| `command` | The program as an absolute path, then up to 16 arguments. No shell runs it, so pipes, `~`, and `$VARS` are not expanded. |
| `color` | Optional, like `#3b82f6`. You can change it in Settings. |

Other keys are rejected, to catch typos.

## What the command prints

One JSON object on stdout, then exit with status 0:

```json
{
  "status": "ok",
  "plan": "Pro",
  "account": "work",
  "windows": [
    { "kind": "short", "label": "Session", "usedPercent": 42.5, "resetsAt": "2026-10-23T18:00:00Z" },
    { "kind": "weekly", "usedPercent": 12, "resetsAt": 1793343600 }
  ]
}
```

| Field | Rule |
|---|---|
| `status` | `ok` (default), `notLoggedIn`, `noPlanLimits`, or `unavailable`. |
| `message` | Optional, up to 200 characters. Shown for statuses other than `ok`, for example how to log in. |
| `plan`, `account` | Optional, up to 40 characters each. |
| `windows` | 1 to 4 limit windows when `status` is `ok`. |
| `windows[].kind` | `short` (a few hours), `weekly`, or `other`. |
| `windows[].label` | Optional, up to 24 characters. |
| `windows[].usedPercent` | Share of the limit used, 0 to 100. |
| `windows[].resetsAt` | Optional. An RFC 3339 time or Unix seconds. |

Never print values you do not know. Use `"status": "unavailable"` with a `message` instead.

## How Sonata runs it

- The first time, and every time the `command` changes, Sonata shows the exact command and asks
  you to allow it.
- It runs at most once a minute while usage reading is on, plus when you press refresh.
- It runs in an empty temporary folder, with only `HOME`, `USER`, `LOGNAME`, `LANG`, `TMPDIR`,
  and `PATH` set.
- It stops after 10 seconds. Output over 64 KiB is rejected.
- It runs inside a macOS sandbox that blocks your keychain, the Claude Code and Codex login
  files, and Sonata's data folder. This folder stays readable, so a helper script can live here.

## Example: wrap another tool

Most tools print their own JSON. A small script turns it into the format above. This one uses
`jq`; save it next to the definition and make it executable (`chmod +x`):

```sh
#!/bin/sh
# Turns `my-agent usage --json` into Sonata's usage format.
/usr/local/bin/my-agent usage --json | /opt/homebrew/bin/jq '{
  plan: .plan_name,
  windows: [
    { kind: "short", label: "5-hour", usedPercent: .session.percent, resetsAt: .session.reset_epoch },
    { kind: "weekly", usedPercent: .week.percent, resetsAt: .week.reset_epoch }
  ]
}'
```

Use full paths inside the script: `PATH` may be shorter than in your terminal.

## Ask an agent to write one

Give your agent this prompt:

> Write a Sonata usage provider for <agent name>. Follow the README.md in
> <path of this folder>. Find a command or file that reports the agent's plan limits, write a
> wrapper script that prints the JSON format from the README, and add the definition file.
> Never read or print credentials.
"##;

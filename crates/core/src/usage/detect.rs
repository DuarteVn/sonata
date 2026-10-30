//! Finds agent commands and the login-shell environment (research R6). One login-shell call
//! prints each command's full path plus the variables provider commands need, because an app
//! started from Finder does not inherit the user's shell setup.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

use super::Detection;
#[cfg(unix)]
use super::runner::{RunSpec, run};

/// Login-shell variables passed on to provider commands when set.
pub const SHELL_VARS: [&str; 9] = [
    "PATH",
    "CLAUDE_CONFIG_DIR",
    "CODEX_HOME",
    "HTTPS_PROXY",
    "HTTP_PROXY",
    "NO_PROXY",
    "https_proxy",
    "http_proxy",
    "no_proxy",
];

#[derive(Debug, Clone, Default, PartialEq)]
pub struct DetectResult {
    commands: BTreeMap<String, (Detection, Option<PathBuf>)>,
    /// Values of [`SHELL_VARS`] that are set in the login shell.
    pub env: Vec<(String, String)>,
    pub timed_out: bool,
}

impl DetectResult {
    pub fn command(&self, name: &str) -> (Detection, Option<PathBuf>) {
        self.commands
            .get(name)
            .cloned()
            .unwrap_or((Detection::Unknown, None))
    }

    /// A result with the given commands (`Some(path)` = found) and no environment. For tests.
    pub fn fake(commands: Vec<(String, Option<PathBuf>)>) -> Self {
        Self {
            commands: commands
                .into_iter()
                .map(|(name, path)| match path {
                    Some(p) => (name, (Detection::Found, Some(p))),
                    None => (name, (Detection::NotFound, None)),
                })
                .collect(),
            env: Vec::new(),
            timed_out: false,
        }
    }

    pub fn env_value(&self, key: &str) -> Option<&str> {
        self.env
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
    }
}

fn valid_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
}

/// Windows apps get the user's environment from the system, so commands are looked up on
/// `PATH` (with `extra_path` first) and variables are read from this process (feature 012, R11).
#[cfg(windows)]
pub fn detect(
    _shell: &Path,
    names: &[&str],
    extra_path: Option<&Path>,
    _timeout: Duration,
) -> DetectResult {
    let mut result = DetectResult::default();
    let extra = extra_path.map(Path::to_path_buf);
    for name in names {
        let found = valid_name(name)
            .then(|| {
                extra
                    .iter()
                    .flat_map(|d| {
                        crate::platform::executable_names(name)
                            .into_iter()
                            .map(move |n| d.join(n))
                    })
                    .find(|p| crate::platform::is_executable(p))
                    .or_else(|| crate::platform::find_on_path(name))
            })
            .flatten();
        let entry = match found {
            Some(p) => (Detection::Found, Some(p)),
            None => (Detection::NotFound, None),
        };
        result.commands.insert(name.to_string(), entry);
    }
    for key in SHELL_VARS {
        if let Some(value) = std::env::var_os(key).filter(|v| !v.is_empty()) {
            result
                .env
                .push((key.to_string(), value.to_string_lossy().into_owned()));
        }
    }
    if let Some(extra) = extra {
        let path = result.env.iter_mut().find(|(k, _)| k == "PATH");
        let joined = |rest: Option<&str>| {
            std::env::join_paths(
                std::iter::once(extra.clone())
                    .chain(rest.into_iter().flat_map(std::env::split_paths)),
            )
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_default()
        };
        match path {
            Some((_, p)) => *p = joined(Some(p.as_str())),
            None => result.env.push(("PATH".into(), joined(None))),
        }
    }
    result
}

/// Asks `shell -l -c` where `names` are. `extra_path` is put first on the shell's `PATH`.
#[cfg(unix)]
pub fn detect(
    shell: &Path,
    names: &[&str],
    extra_path: Option<&Path>,
    timeout: Duration,
) -> DetectResult {
    let wanted: Vec<&str> = names.iter().copied().filter(|n| valid_name(n)).collect();
    let script = format!(
        "[ -n \"$SONATA_USAGE_PATH\" ] && PATH=\"$SONATA_USAGE_PATH:$PATH\"; \
         for p in {}; do r=$(command -v \"$p\" 2>/dev/null); printf 'cmd\\t%s\\t%s\\n' \"$p\" \"$r\"; done; \
         for v in {}; do eval \"x=\\${{$v-}}\"; [ -n \"$x\" ] && printf 'env\\t%s\\t%s\\n' \"$v\" \"$x\"; done; exit 0",
        wanted.join(" "),
        SHELL_VARS.join(" ")
    );
    let mut spec = RunSpec::new(
        vec![
            shell.display().to_string(),
            "-l".into(),
            "-c".into(),
            script,
        ],
        timeout,
    );
    if let Some(extra) = extra_path {
        spec.env
            .push(("SONATA_USAGE_PATH".into(), extra.display().to_string()));
    }
    let mut result = DetectResult::default();
    let output = match run(&spec) {
        Ok(out) => String::from_utf8_lossy(&out.stdout).into_owned(),
        Err(_) => {
            result.timed_out = true;
            for name in names {
                result
                    .commands
                    .insert(name.to_string(), (Detection::Unknown, None));
            }
            return result;
        }
    };
    for name in names {
        result
            .commands
            .insert(name.to_string(), (Detection::NotFound, None));
    }
    for line in output.lines() {
        let mut parts = line.splitn(3, '\t');
        match (parts.next(), parts.next(), parts.next()) {
            (Some("cmd"), Some(name), Some(path)) if path.starts_with('/') => {
                if wanted.contains(&name) {
                    result.commands.insert(
                        name.to_string(),
                        (Detection::Found, Some(PathBuf::from(path))),
                    );
                }
            }
            (Some("env"), Some(key), Some(value)) if SHELL_VARS.contains(&key) => {
                result.env.push((key.to_string(), value.to_string()));
            }
            _ => {}
        }
    }
    if let Some(extra) = extra_path {
        // Login shells may reorder PATH (path_helper); the extra folder must still come first.
        let extra = extra.display().to_string();
        match result.env.iter_mut().find(|(k, _)| k == "PATH") {
            Some((_, path)) if !path.starts_with(&extra) => *path = format!("{extra}:{path}"),
            Some(_) => {}
            None => result.env.push(("PATH".into(), extra)),
        }
    }
    result
}

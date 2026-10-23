//! The real backend: detects commands through the login shell and runs agent commands.

use std::path::PathBuf;
use std::time::Duration;

use chrono::{DateTime, Utc};

use super::detect::{DetectResult, detect};
use super::service::{Backend, ProviderInfo};
use super::{ProviderKind, Reading};
use crate::paths::DataRoot;

pub const DETECT_TIMEOUT: Duration = Duration::from_secs(10);

pub struct SystemBackend {
    pub shell: PathBuf,
    /// Put first on `PATH` (tests point it at fake agent commands).
    pub extra_path: Option<PathBuf>,
    pub root: DataRoot,
}

impl Backend for SystemBackend {
    fn detect(&self, names: &[&str]) -> DetectResult {
        detect(
            &self.shell,
            names,
            self.extra_path.as_deref(),
            DETECT_TIMEOUT,
        )
    }

    fn read(&self, p: &ProviderInfo, env: &[(String, String)], now: DateTime<Utc>) -> Reading {
        let Some(path) = p.command_path.as_deref() else {
            return Reading::unavailable(p.id.clone(), now, "Command not found.");
        };
        match (p.kind, p.id.as_str()) {
            (ProviderKind::BuiltIn, "claude") => super::claude::read(path, env, now),
            _ => Reading::unavailable(p.id.clone(), now, "Not supported yet."),
        }
    }
}

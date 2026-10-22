//! Reading and writing Sonata's files under the data root.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::Serialize;
use serde::de::DeserializeOwned;
use uuid::Uuid;

use super::atomic;
use super::envelope::{self, EnvelopeError};
use super::migrate::Migrations;
use crate::model::app_state::{APP_STATE_SCHEMA_VERSION, APP_STATE_TYPE, AppState};
use crate::model::preferences::{PREFERENCES_SCHEMA_VERSION, PREFERENCES_TYPE, Preferences};
use crate::model::workspace::{WORKSPACE_SCHEMA_VERSION, WORKSPACE_TYPE, Workspace};
use crate::paths::{DataRoot, parse_workspace_folder};

/// Why a workspace folder could not be loaded. Shown in the sidebar (FR-025).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum LoadFailureReason {
    InvalidJson { detail: String },
    WrongType { found: String },
    NewerVersion { found: u32, supported: u32 },
    MigrationFailed { step: u32, detail: String },
    DecodeFailed { detail: String },
    IdMismatch { found: Uuid },
    Unreadable { detail: String },
}

impl From<EnvelopeError> for LoadFailureReason {
    fn from(err: EnvelopeError) -> Self {
        match err {
            EnvelopeError::InvalidJson(detail) => Self::InvalidJson { detail },
            EnvelopeError::WrongType { found, .. } => Self::WrongType { found },
            EnvelopeError::NewerVersion { found, supported } => {
                Self::NewerVersion { found, supported }
            }
            EnvelopeError::MigrationFailed { step, detail } => {
                Self::MigrationFailed { step, detail }
            }
            EnvelopeError::DecodeFailed(detail) => Self::DecodeFailed { detail },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoadFailure {
    pub folder_name: String,
    pub path: PathBuf,
    pub reason: LoadFailureReason,
}

#[derive(Debug, Default)]
pub struct Discovery {
    pub workspaces: Vec<Workspace>,
    pub failures: Vec<LoadFailure>,
}

/// Result of loading a settings file: the value plus a warning when the file was invalid.
#[derive(Debug)]
pub struct Loaded<T> {
    pub value: T,
    pub warning: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Store {
    root: DataRoot,
}

fn workspace_migrations() -> Migrations {
    Migrations::new(WORKSPACE_SCHEMA_VERSION).step(1, workspace_v1_to_v2)
}

/// Version 2 adds `links` (feature 002).
fn workspace_v1_to_v2(mut payload: serde_json::Value) -> Result<serde_json::Value, String> {
    let obj = payload.as_object_mut().ok_or("payload is not an object")?;
    obj.entry("links")
        .or_insert_with(|| serde_json::Value::Array(Vec::new()));
    Ok(payload)
}

impl Store {
    pub fn new(root: DataRoot) -> Self {
        Self { root }
    }

    pub fn root(&self) -> &DataRoot {
        &self.root
    }

    pub fn save_workspace(&self, ws: &Workspace) -> io::Result<()> {
        self.root.create_workspace_dir(ws.id)?;
        let bytes = envelope::encode(WORKSPACE_TYPE, WORKSPACE_SCHEMA_VERSION, ws)
            .map_err(io::Error::other)?;
        atomic::write(&self.root.workspace_file(ws.id), &bytes)
    }

    pub fn load_workspace(&self, id: Uuid) -> Result<Workspace, LoadFailureReason> {
        let path = self.root.workspace_file(id);
        let bytes = fs::read(&path).map_err(|e| LoadFailureReason::Unreadable {
            detail: e.to_string(),
        })?;
        let ws: Workspace = envelope::decode(&bytes, WORKSPACE_TYPE, &workspace_migrations())?;
        if ws.id != id {
            return Err(LoadFailureReason::IdMismatch { found: ws.id });
        }
        Ok(ws)
    }

    /// Lists `workspaces/` and loads every `<uuid>/workspace.json`. Folders whose name is not a
    /// UUID, or that have no `workspace.json`, are ignored.
    pub fn discover(&self) -> io::Result<Discovery> {
        let mut out = Discovery::default();
        let mut entries: Vec<_> = fs::read_dir(self.root.workspaces_dir())?
            .filter_map(Result::ok)
            .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
            .collect();
        entries.sort_by_key(|e| e.file_name());
        for entry in entries {
            let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
                continue;
            };
            let Some(id) = parse_workspace_folder(&name) else {
                continue;
            };
            let file = self.root.workspace_file(id);
            if !file.is_file() {
                continue;
            }
            match self.load_workspace(id) {
                Ok(ws) => out.workspaces.push(ws),
                Err(reason) => {
                    tracing::warn!(folder = %name, ?reason, "workspace failed to load");
                    out.failures.push(LoadFailure {
                        folder_name: name,
                        path: file,
                        reason,
                    });
                }
            }
        }
        Ok(out)
    }

    pub fn delete_workspace_dir(&self, id: Uuid) -> io::Result<()> {
        let dir = self.root.workspace_dir(id);
        self.root
            .check_deletable_workspace_dir(&dir)
            .map_err(io::Error::other)?;
        if !dir.exists() {
            return Ok(());
        }
        // Rename first so a crash mid-removal never leaves a half-deleted workspace that loads.
        let trash = self.root.workspaces_dir().join(format!(".trash-{id}"));
        fs::rename(&dir, &trash)?;
        fs::remove_dir_all(&trash)
    }

    pub fn load_app_state(&self) -> Loaded<AppState> {
        self.load_settings(
            &self.root.app_state_file(),
            APP_STATE_TYPE,
            APP_STATE_SCHEMA_VERSION,
        )
    }

    pub fn save_app_state(&self, state: &AppState) -> io::Result<()> {
        self.save_settings(
            &self.root.app_state_file(),
            APP_STATE_TYPE,
            APP_STATE_SCHEMA_VERSION,
            state,
        )
    }

    pub fn load_preferences(&self) -> Loaded<Preferences> {
        let loaded: Loaded<Preferences> = self.load_settings(
            &self.root.preferences_file(),
            PREFERENCES_TYPE,
            PREFERENCES_SCHEMA_VERSION,
        );
        Loaded {
            value: loaded.value.normalized(),
            warning: loaded.warning,
        }
    }

    pub fn save_preferences(&self, prefs: &Preferences) -> io::Result<()> {
        self.save_settings(
            &self.root.preferences_file(),
            PREFERENCES_TYPE,
            PREFERENCES_SCHEMA_VERSION,
            prefs,
        )
    }

    fn save_settings<T: Serialize>(
        &self,
        path: &Path,
        kind: &str,
        version: u32,
        v: &T,
    ) -> io::Result<()> {
        let bytes = envelope::encode(kind, version, v).map_err(io::Error::other)?;
        atomic::write(path, &bytes)
    }

    /// Missing file → defaults. Invalid file → renamed to `<name>.invalid-<timestamp>`, defaults,
    /// and a warning.
    fn load_settings<T: DeserializeOwned + Default>(
        &self,
        path: &Path,
        kind: &str,
        version: u32,
    ) -> Loaded<T> {
        let bytes = match fs::read(path) {
            Ok(b) => b,
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                return Loaded {
                    value: T::default(),
                    warning: None,
                };
            }
            Err(e) => {
                return Loaded {
                    value: T::default(),
                    warning: Some(format!(
                        "Could not read {}: {e}. Using defaults.",
                        path.display()
                    )),
                };
            }
        };
        match envelope::decode(&bytes, kind, &Migrations::new(version)) {
            Ok(value) => Loaded {
                value,
                warning: None,
            },
            Err(err) => {
                let stamp = chrono::Utc::now().format("%Y%m%dT%H%M%SZ");
                let name = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("settings");
                let aside = path.with_file_name(format!("{name}.invalid-{stamp}"));
                let moved = fs::rename(path, &aside).is_ok();
                let where_ = if moved {
                    format!(" It was kept as {}.", aside.display())
                } else {
                    String::new()
                };
                Loaded {
                    value: T::default(),
                    warning: Some(format!(
                        "{} was invalid ({err}); defaults are in use.{where_}",
                        path.display()
                    )),
                }
            }
        }
    }
}

//! Data root resolution and path safety rules (constitution principle V).

use std::ffi::OsString;
use std::fs;
use std::io;
use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
use std::path::{Component, Path, PathBuf};

use uuid::Uuid;

pub const DIR_MODE: u32 = 0o700;
pub const FILE_MODE: u32 = 0o600;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PathError {
    #[error("path contains a '..' component")]
    ParentComponent,
    #[error("path is not a Sonata workspace folder")]
    NotAWorkspaceFolder,
}

/// `$SONATA_HOME` when set and non-empty, else `<home>/.sonata`.
pub fn resolve_root(sonata_home: Option<OsString>, home: &Path) -> PathBuf {
    match sonata_home {
        Some(v) if !v.is_empty() => PathBuf::from(v),
        _ => home.join(".sonata"),
    }
}

pub fn parse_workspace_folder(name: &str) -> Option<Uuid> {
    Uuid::try_parse(name)
        .ok()
        .filter(|id| id.to_string() == name)
}

pub fn reject_parent_components(path: &Path) -> Result<(), PathError> {
    if path.components().any(|c| c == Component::ParentDir) {
        Err(PathError::ParentComponent)
    } else {
        Ok(())
    }
}

/// Creates `path` (and parents) and forces mode `0700` on it.
pub fn ensure_private_dir(path: &Path) -> io::Result<()> {
    fs::DirBuilder::new()
        .recursive(true)
        .mode(DIR_MODE)
        .create(path)?;
    set_mode(path, DIR_MODE)
}

pub fn set_mode(path: &Path, mode: u32) -> io::Result<()> {
    fs::set_permissions(path, fs::Permissions::from_mode(mode))
}

/// The folder holding all Sonata data. Every folder it creates is private.
#[derive(Debug, Clone)]
pub struct DataRoot {
    root: PathBuf,
}

impl DataRoot {
    /// Resolves the root from `SONATA_HOME` / `HOME` and creates it.
    pub fn from_env() -> io::Result<Self> {
        let home = std::env::var_os("HOME")
            .map(PathBuf::from)
            .ok_or_else(|| io::Error::other("HOME is not set"))?;
        Self::at(resolve_root(std::env::var_os("SONATA_HOME"), &home))
    }

    pub fn at(root: impl Into<PathBuf>) -> io::Result<Self> {
        let root = root.into();
        if !root.is_absolute() {
            return Err(io::Error::other("data root must be an absolute path"));
        }
        reject_parent_components(&root).map_err(io::Error::other)?;
        ensure_private_dir(&root)?;
        let this = Self { root };
        ensure_private_dir(&this.workspaces_dir())?;
        Ok(this)
    }

    pub fn path(&self) -> &Path {
        &self.root
    }

    pub fn workspaces_dir(&self) -> PathBuf {
        self.root.join("workspaces")
    }

    pub fn workspace_dir(&self, id: Uuid) -> PathBuf {
        self.workspaces_dir().join(id.to_string())
    }

    pub fn workspace_file(&self, id: Uuid) -> PathBuf {
        self.workspace_dir(id).join("workspace.json")
    }

    pub fn notes_dir(&self, id: Uuid) -> PathBuf {
        self.workspace_dir(id).join("notes")
    }

    pub fn terminals_dir(&self, id: Uuid) -> PathBuf {
        self.workspace_dir(id).join("terminals")
    }

    pub fn app_state_file(&self) -> PathBuf {
        self.root.join("app-state.json")
    }

    pub fn preferences_file(&self) -> PathBuf {
        self.root.join("preferences.json")
    }

    pub fn usage_cache_file(&self) -> PathBuf {
        self.root.join("usage-cache.json")
    }

    /// Custom usage provider definitions (feature 006). Created on demand.
    pub fn usage_providers_dir(&self) -> PathBuf {
        self.root.join("usage-providers")
    }

    pub fn logs_dir(&self) -> PathBuf {
        self.root.join("logs")
    }

    pub fn create_workspace_dir(&self, id: Uuid) -> io::Result<PathBuf> {
        let dir = self.workspace_dir(id);
        ensure_private_dir(&dir)?;
        ensure_private_dir(&self.notes_dir(id))?;
        ensure_private_dir(&self.terminals_dir(id))?;
        Ok(dir)
    }

    /// Accepts only `<root>/workspaces/<uuid>` exactly, without `..`.
    pub fn check_deletable_workspace_dir(&self, path: &Path) -> Result<(), PathError> {
        reject_parent_components(path)?;
        let parent = path.parent().ok_or(PathError::NotAWorkspaceFolder)?;
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or(PathError::NotAWorkspaceFolder)?;
        if parent != self.workspaces_dir() || parse_workspace_folder(name).is_none() {
            return Err(PathError::NotAWorkspaceFolder);
        }
        Ok(())
    }
}

/// Forces `0700` on folders and `0600` on files under `root`. Returns how many entries were
/// fixed (FR-015a). Symbolic links are left alone.
pub fn enforce_private_tree(root: &Path) -> io::Result<usize> {
    let mut fixed = 0;
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in fs::read_dir(&dir)? {
            let entry = entry?;
            let kind = entry.file_type()?;
            if kind.is_symlink() {
                continue;
            }
            let path = entry.path();
            let wanted = if kind.is_dir() { DIR_MODE } else { FILE_MODE };
            let current = entry.metadata()?.permissions().mode() & 0o777;
            if current != wanted {
                set_mode(&path, wanted)?;
                fixed += 1;
            }
            if kind.is_dir() {
                stack.push(path);
            }
        }
    }
    let current = fs::metadata(root)?.permissions().mode() & 0o777;
    if current != DIR_MODE {
        set_mode(root, DIR_MODE)?;
        fixed += 1;
    }
    Ok(fixed)
}

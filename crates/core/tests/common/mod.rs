//! Shared helpers for core integration tests.
#![allow(dead_code)]

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use sonata_core::paths::DataRoot;
use tempfile::TempDir;

/// A throwaway data root plus a throwaway "projects" folder, both removed on drop.
pub struct Sandbox {
    pub tmp: TempDir,
    pub root: DataRoot,
}

impl Sandbox {
    pub fn new() -> Self {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = DataRoot::at(tmp.path().join("sonata")).expect("data root");
        Self { tmp, root }
    }

    /// Creates `<tmp>/projects/<name>` and returns its path.
    pub fn project(&self, name: &str) -> PathBuf {
        let dir = self.tmp.path().join("projects").join(name);
        std::fs::create_dir_all(&dir).expect("project dir");
        dir
    }
}

pub fn mode(path: &Path) -> u32 {
    std::fs::metadata(path)
        .expect("metadata")
        .permissions()
        .mode()
        & 0o777
}

pub fn inode(path: &Path) -> u64 {
    use std::os::unix::fs::MetadataExt;
    std::fs::metadata(path).expect("metadata").ino()
}

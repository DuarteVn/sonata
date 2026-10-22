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

use std::sync::Arc;
use std::time::{Duration, Instant};

use sonata_core::App;
use sonata_core::app::AppOptions;
use sonata_core::events::RecordingSink;

/// Polls `check` until it returns true or `timeout` passes.
pub fn wait_until(timeout: Duration, mut check: impl FnMut() -> bool) -> bool {
    let start = Instant::now();
    while start.elapsed() < timeout {
        if check() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(25));
    }
    check()
}

pub const WAIT: Duration = Duration::from_secs(5);

/// An app with `/bin/sh` as the shell, so tests do not depend on the user's shell config.
pub fn test_app(sb: &Sandbox) -> (Arc<App>, Arc<RecordingSink>) {
    let sink = Arc::new(RecordingSink::default());
    let opts = AppOptions {
        shell: Some("/bin/sh".into()),
    };
    let app = App::start_with(sb.root.clone(), sink.clone(), opts).expect("app");
    (app, sink)
}

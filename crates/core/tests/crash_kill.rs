//! SC-005: killing the app during saves never leaves a corrupt file.
//! The child process is this test binary re-run with `SONATA_SAVE_LOOP` set.

use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::Duration;

use sonata_core::paths::DataRoot;
use sonata_core::persistence::store::Store;
use sonata_core::workspace::manager::{CreateWorkspace, WorkspaceManager};

const KILLS: usize = 100;

#[test]
fn save_loop_child() {
    let Some(root) = std::env::var_os("SONATA_SAVE_LOOP") else {
        return;
    };
    let root = DataRoot::at(PathBuf::from(root)).unwrap();
    let mut m = WorkspaceManager::load(Store::new(root)).unwrap();
    let ids = m.ids();
    let mut i = 0u64;
    loop {
        for id in &ids {
            i += 1;
            m.set_camera(*id, i as f64, -(i as f64), 1.0 + (i % 3) as f64)
                .unwrap();
        }
        m.save_dirty();
    }
}

#[test]
fn killed_during_saves_never_corrupts_files() {
    let tmp = tempfile::tempdir().unwrap();
    let root_path = tmp.path().join("sonata");
    let project = tmp.path().join("project");
    std::fs::create_dir_all(&project).unwrap();
    {
        let mut m = WorkspaceManager::load(Store::new(DataRoot::at(&root_path).unwrap())).unwrap();
        for i in 0..3 {
            m.create(CreateWorkspace {
                name: format!("W{i}"),
                working_directory: project.clone(),
            })
            .unwrap();
        }
    }
    let exe = std::env::current_exe().unwrap();
    for round in 0..KILLS {
        let mut child = Command::new(&exe)
            .args([
                "--exact",
                "save_loop_child",
                "--nocapture",
                "--test-threads=1",
            ])
            .env("SONATA_SAVE_LOOP", &root_path)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        std::thread::sleep(Duration::from_millis(20 + (round as u64 * 7) % 60));
        child.kill().unwrap(); // SIGKILL
        child.wait().unwrap();

        let m = WorkspaceManager::load(Store::new(DataRoot::at(&root_path).unwrap())).unwrap();
        assert!(m.failures().is_empty(), "round {round}: {:?}", m.failures());
        assert_eq!(m.ids().len(), 3, "round {round}");
    }
}

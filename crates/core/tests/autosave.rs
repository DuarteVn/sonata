mod common;

use common::{Sandbox, inode};
use sonata_core::autosave;
use sonata_core::model::preferences::Preferences;
use sonata_core::persistence::store::Store;
use sonata_core::workspace::manager::{CreateWorkspace, WorkspaceManager};

fn setup(sb: &Sandbox) -> (WorkspaceManager, uuid::Uuid, uuid::Uuid) {
    let mut m = WorkspaceManager::load(Store::new(sb.root.clone())).unwrap();
    let dir = sb.project("a");
    let a = m
        .create(CreateWorkspace {
            name: "A".into(),
            working_directory: dir.clone(),
        })
        .unwrap();
    let b = m
        .create(CreateWorkspace {
            name: "B".into(),
            working_directory: dir,
        })
        .unwrap();
    (m, a.id, b.id)
}

#[test]
fn saves_only_changed_workspaces() {
    let sb = Sandbox::new();
    let (mut m, a, b) = setup(&sb);
    let file_a = sb.root.workspace_dir(a).join("workspace.json");
    let file_b = sb.root.workspace_dir(b).join("workspace.json");
    let (ia, ib) = (inode(&file_a), inode(&file_b));

    m.set_camera(a, 10.0, 20.0, 1.5).unwrap();
    let report = autosave::tick(&mut m);
    assert!(report.errors.is_empty());
    assert_eq!(report.saved, vec![a]);
    assert_ne!(inode(&file_a), ia);
    assert_eq!(inode(&file_b), ib, "unchanged workspace was rewritten");

    let again = autosave::tick(&mut m);
    assert!(again.saved.is_empty());
}

#[test]
fn save_all_writes_every_dirty_workspace() {
    let sb = Sandbox::new();
    let (mut m, a, b) = setup(&sb);
    m.set_camera(a, 1.0, 1.0, 1.0).unwrap();
    m.set_camera(b, 2.0, 2.0, 2.0).unwrap();
    let mut saved = autosave::save_all(&mut m).saved;
    saved.sort();
    let mut expected = vec![a, b];
    expected.sort();
    assert_eq!(saved, expected);

    let reloaded = WorkspaceManager::load(Store::new(sb.root.clone())).unwrap();
    assert_eq!(reloaded.workspace(b).unwrap().camera.zoom, 2.0);
}

#[test]
fn interval_comes_from_preferences_and_is_clamped() {
    assert_eq!(Preferences::default().autosave_interval_seconds, 30);
    let p = Preferences {
        autosave_interval_seconds: 1,
        ..Preferences::default()
    }
    .normalized();
    assert_eq!(p.autosave_interval_seconds, 5);
    let p = Preferences {
        autosave_interval_seconds: 9999,
        ..Preferences::default()
    }
    .normalized();
    assert_eq!(p.autosave_interval_seconds, 600);
    assert_eq!(autosave::interval(&Preferences::default()).as_secs(), 30);
}

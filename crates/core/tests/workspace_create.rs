mod common;

use common::Sandbox;
use sonata_core::ErrorCode;
use sonata_core::model::workspace::Camera;
use sonata_core::persistence::store::Store;
use sonata_core::workspace::manager::{CreateWorkspace, WorkspaceManager};
use sonata_core::workspace::palette::{COLORS, ICONS};

fn manager(sb: &Sandbox) -> WorkspaceManager {
    WorkspaceManager::load(Store::new(sb.root.clone())).unwrap()
}

fn req(name: &str, dir: &std::path::Path) -> CreateWorkspace {
    CreateWorkspace {
        name: name.into(),
        working_directory: dir.to_path_buf(),
    }
}

#[test]
fn creates_and_persists_workspace() {
    let sb = Sandbox::new();
    let mut m = manager(&sb);
    let dir = sb.project("a");
    let ws = m.create(req("  Demo  ", &dir)).unwrap();
    assert_eq!(ws.name, "Demo");
    assert!(ICONS.contains(&ws.icon.as_str()));
    assert!(COLORS.contains(&ws.color.as_str()));
    assert!(
        sb.root
            .workspace_dir(ws.id)
            .join("workspace.json")
            .is_file()
    );

    let full = m.workspace(ws.id).unwrap();
    assert_eq!(
        full.camera,
        Camera {
            x: 0.0,
            y: 0.0,
            zoom: 1.0
        }
    );
    assert_eq!(full.working_directory, dir);

    // Survives a reload from disk.
    let reloaded = manager(&sb);
    assert_eq!(reloaded.summaries().len(), 1);
}

#[test]
fn empty_name_rejected() {
    let sb = Sandbox::new();
    let mut m = manager(&sb);
    let err = m.create(req("   ", &sb.project("a"))).unwrap_err();
    assert_eq!(err.code, ErrorCode::NameEmpty);
}

#[test]
fn long_name_rejected() {
    let sb = Sandbox::new();
    let mut m = manager(&sb);
    let err = m
        .create(req(&"x".repeat(81), &sb.project("a")))
        .unwrap_err();
    assert_eq!(err.code, ErrorCode::NameTooLong);
    assert!(m.create(req(&"x".repeat(80), &sb.project("a"))).is_ok());
}

#[test]
fn duplicate_name_ignoring_case_and_spaces_rejected() {
    let sb = Sandbox::new();
    let mut m = manager(&sb);
    m.create(req("Demo", &sb.project("a"))).unwrap();
    let err = m.create(req(" demo ", &sb.project("b"))).unwrap_err();
    assert_eq!(err.code, ErrorCode::NameTaken);
    assert_eq!(m.summaries().len(), 1);
}

#[test]
fn directory_rules() {
    let sb = Sandbox::new();
    let mut m = manager(&sb);
    let missing = sb.tmp.path().join("nope");
    assert_eq!(
        m.create(req("A", &missing)).unwrap_err().code,
        ErrorCode::DirectoryMissing
    );

    let file = sb.tmp.path().join("file.txt");
    std::fs::write(&file, "x").unwrap();
    assert_eq!(
        m.create(req("A", &file)).unwrap_err().code,
        ErrorCode::NotADirectory
    );

    let relative = std::path::Path::new("relative/path");
    assert_eq!(
        m.create(req("A", relative)).unwrap_err().code,
        ErrorCode::DirectoryMissing
    );
    assert!(m.summaries().is_empty());
}

#[test]
fn prefers_unused_colors() {
    let sb = Sandbox::new();
    let mut m = manager(&sb);
    let dir = sb.project("a");
    let mut colors: Vec<String> = (0..COLORS.len())
        .map(|i| m.create(req(&format!("W{i}"), &dir)).unwrap().color)
        .collect();
    colors.sort();
    colors.dedup();
    assert_eq!(colors.len(), COLORS.len());
}

#[test]
fn no_count_limit() {
    let sb = Sandbox::new();
    let mut m = manager(&sb);
    let dir = sb.project("a");
    for i in 0..60 {
        m.create(req(&format!("Workspace {i}"), &dir)).unwrap();
    }
    assert_eq!(m.summaries().len(), 60);
}

#[test]
fn open_sets_active_and_recent() {
    let sb = Sandbox::new();
    let mut m = manager(&sb);
    let dir = sb.project("a");
    let ids: Vec<_> = (0..12)
        .map(|i| m.create(req(&format!("W{i}"), &dir)).unwrap().id)
        .collect();
    for id in &ids {
        m.open(*id).unwrap();
    }
    assert_eq!(m.app_state().active_workspace_id, Some(ids[11]));
    assert_eq!(m.app_state().recent_workspace_ids.len(), 10);
    assert_eq!(m.app_state().recent_workspace_ids[0], ids[11]);
}

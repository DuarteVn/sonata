mod common;

use common::{Sandbox, test_app};
use sonata_core::ErrorCode;
use sonata_core::app::NewTerminal;
use sonata_core::persistence::store::Store;
use sonata_core::workspace::manager::{CreateWorkspace, WorkspaceManager};
use sonata_core::workspace::palette::COLORS;

fn create(app: &sonata_core::App, name: &str, dir: &std::path::Path) -> uuid::Uuid {
    app.create_workspace(CreateWorkspace {
        name: name.into(),
        working_directory: dir.to_path_buf(),
    })
    .unwrap()
    .id
}

#[test]
fn rename_follows_create_rules() {
    let sb = Sandbox::new();
    let (app, _) = test_app(&sb);
    let dir = sb.project("p");
    let a = create(&app, "Alpha", &dir);
    create(&app, "Beta", &dir);
    assert_eq!(
        app.rename_workspace(a, "  ").unwrap_err().code,
        ErrorCode::NameEmpty
    );
    assert_eq!(
        app.rename_workspace(a, "beta").unwrap_err().code,
        ErrorCode::NameTaken
    );
    assert_eq!(app.rename_workspace(a, "ALPHA").unwrap().name, "ALPHA");
    assert_eq!(app.rename_workspace(a, " Gamma ").unwrap().name, "Gamma");
}

#[test]
fn pinned_first_and_order_persists() {
    let sb = Sandbox::new();
    let (app, _) = test_app(&sb);
    let dir = sb.project("p");
    let ids: Vec<_> = ["A", "B", "C", "D"]
        .iter()
        .map(|n| create(&app, n, &dir))
        .collect();
    app.reorder_workspaces(vec![ids[3], ids[2], ids[1], ids[0]])
        .unwrap();
    app.set_pinned(ids[1], true).unwrap();
    let names: Vec<String> = app.summaries().into_iter().map(|s| s.name).collect();
    assert_eq!(names, ["B", "D", "C", "A"]);
    app.save_dirty();

    let reloaded = WorkspaceManager::load(Store::new(sb.root.clone())).unwrap();
    let names: Vec<String> = reloaded.summaries().into_iter().map(|s| s.name).collect();
    assert_eq!(names, ["B", "D", "C", "A"]);
}

#[test]
fn colors_unique_until_palette_exhausted() {
    let sb = Sandbox::new();
    let (app, _) = test_app(&sb);
    let dir = sb.project("p");
    for i in 0..=COLORS.len() {
        create(&app, &format!("W{i}"), &dir);
    }
    let mut colors: Vec<String> = app.summaries().into_iter().map(|s| s.color).collect();
    colors.sort();
    colors.dedup();
    assert_eq!(colors.len(), COLORS.len());
}

#[test]
fn delete_stops_terminals_removes_only_sonata_data() {
    let sb = Sandbox::new();
    let (app, _) = test_app(&sb);
    let dir = sb.project("keep-me");
    std::fs::write(dir.join("important.txt"), "data").unwrap();
    let id = create(&app, "Gone", &dir);
    let other = create(&app, "Stays", &dir);
    app.open_workspace(id).unwrap();
    let t = app.create_terminal(id, NewTerminal::default()).unwrap();
    assert!(app.terminals().status(t.id).is_some());

    app.delete_workspace(id).unwrap();
    assert!(
        app.terminals().status(t.id).is_none(),
        "terminal still running"
    );
    assert!(!sb.root.workspace_dir(id).exists());
    assert!(
        dir.join("important.txt").is_file(),
        "project folder touched"
    );
    assert!(sb.root.workspace_dir(other).exists());
    assert_eq!(app.summaries().len(), 1);
    assert!(!app.workspaces().app_state().workspace_order.contains(&id));
    let leftovers: Vec<_> = std::fs::read_dir(sb.root.workspaces_dir())
        .unwrap()
        .collect();
    assert_eq!(leftovers.len(), 1, "trash folder left behind");
}

#[test]
fn delete_unknown_workspace_fails() {
    let sb = Sandbox::new();
    let (app, _) = test_app(&sb);
    assert_eq!(
        app.delete_workspace(uuid::Uuid::new_v4()).unwrap_err().code,
        ErrorCode::WorkspaceNotFound
    );
}

#[test]
fn missing_working_directory_is_flagged() {
    let sb = Sandbox::new();
    let (app, _) = test_app(&sb);
    let dir = sb.project("temp");
    let id = create(&app, "W", &dir);
    assert!(!app.summaries()[0].working_directory_missing);
    std::fs::remove_dir(&dir).unwrap();
    let s = app.summaries().into_iter().find(|s| s.id == id).unwrap();
    assert!(s.working_directory_missing);
}

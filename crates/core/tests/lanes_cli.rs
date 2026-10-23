mod common;

use common::{Sandbox, git_repo, messaging_app, shells, sonata_as};
use sonata_core::App;
use sonata_core::app::TerminalSettings;
use uuid::Uuid;

fn make_orchestrator(app: &App, id: Uuid) {
    app.update_terminal_settings(
        id,
        TerminalSettings {
            orchestrator: Some(true),
            ..Default::default()
        },
    )
    .unwrap();
}

fn node_named(app: &App, ws: Uuid, name: &str) -> sonata_core::model::node::Node {
    app.workspaces()
        .workspace(ws)
        .unwrap()
        .nodes
        .iter()
        .find(|n| n.terminal().is_some_and(|t| t.name == name))
        .cloned()
        .expect("node")
}

#[test]
fn recruit_with_lane_creates_a_linked_lane_terminal() {
    let sb = Sandbox::new();
    git_repo(&sb, "p");
    let (app, _) = messaging_app(&sb);
    let (ws, ids) = shells(&app, &sb, &["Lead"]);
    make_orchestrator(&app, ids[0]);

    let out = sonata_as(
        &app,
        ids[0],
        &["recruit", "shell", "--name", "Helper", "--lane"],
    );
    assert_eq!(out.code, 0, "{}", out.stderr);
    assert!(out.stdout.contains("lane/helper"), "{}", out.stdout);
    let helper = node_named(&app, ws, "Helper");
    let lane = helper
        .terminal()
        .unwrap()
        .lane
        .as_deref()
        .cloned()
        .expect("lane");
    assert_eq!(lane.base_branch, "main");
    assert!(!app.links_of(ws, helper.id).is_empty());

    let both = sonata_as(
        &app,
        ids[0],
        &["recruit", "shell", "--lane", "--folder", "."],
    );
    assert_eq!(both.code, 2, "{}", both.stderr);
    app.quit();
}

#[test]
fn recruit_with_lane_outside_a_repository_fails() {
    let sb = Sandbox::new();
    let (app, _) = messaging_app(&sb);
    let (_, ids) = shells(&app, &sb, &["Lead"]);
    make_orchestrator(&app, ids[0]);
    let out = sonata_as(&app, ids[0], &["recruit", "shell", "--lane"]);
    assert_eq!(out.code, 1, "{}", out.stderr);
    assert!(
        out.stderr.contains("not inside a git repository"),
        "{}",
        out.stderr
    );
    app.quit();
}

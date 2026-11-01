mod common;

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use common::{Sandbox, WAIT, commit_file, git, git_repo, test_app, wait_until};
use sonata_core::App;
use sonata_core::ErrorCode;
use sonata_core::app::NewTerminal;
use sonata_core::lanes::git::Git;
use sonata_core::lanes::ops;
use sonata_core::model::node::{Frame, Lane};
use sonata_core::workspace::manager::CreateWorkspace;
use uuid::Uuid;

fn frame() -> Frame {
    Frame {
        x: 0.0,
        y: 0.0,
        width: 640.0,
        height: 400.0,
    }
}

fn open_workspace(app: &App, dir: &Path) -> Uuid {
    let ws = app
        .create_workspace(CreateWorkspace {
            name: "Repo".into(),
            working_directory: dir.to_path_buf(),
        })
        .unwrap()
        .id;
    app.open_workspace(ws).unwrap();
    ws
}

fn lane_terminal(app: &App, ws: Uuid, name: &str) -> (Uuid, Lane) {
    let node = app
        .create_terminal(
            ws,
            NewTerminal {
                frame: frame(),
                name: Some(name.into()),
                lane: true,
                ..Default::default()
            },
        )
        .unwrap();
    let lane = node
        .terminal()
        .unwrap()
        .lane
        .as_deref()
        .cloned()
        .expect("lane set");
    (node.id, lane)
}

/// The folder the terminal's shell runs in, as printed by the shell and then resolved.
fn shell_cwd(app: &Arc<App>, sb: &Sandbox, node: Uuid) -> PathBuf {
    let out = sb.tmp.path().join(format!("cwd-{node}"));
    assert!(wait_until(WAIT, || app
        .terminal_ready(node, Duration::from_millis(300))));
    app.terminals()
        .write(node, common::cwd_command(&out).as_bytes())
        .unwrap();
    assert!(wait_until(WAIT, || std::fs::read_to_string(&out)
        .is_ok_and(|s| s.ends_with('\n'))));
    common::real(Path::new(std::fs::read_to_string(&out).unwrap().trim()))
}

#[test]
fn availability_explains_why_not() {
    let sb = Sandbox::new();
    let git_bin = Git::new("git");
    let plain = sb.project("plain");
    let a = ops::availability(Ok(&git_bin), &plain);
    assert!(!a.available);
    assert!(a.reason.unwrap().contains("not inside a git repository"));

    let repo = git_repo(&sb, "repo");
    let a = ops::availability(Ok(&git_bin), &repo);
    assert!(a.available, "{:?}", a.reason);
    assert_eq!(a.base_branch.as_deref(), Some("main"));

    let a = ops::availability(Err("Git is not installed."), &repo);
    assert_eq!(a.reason.as_deref(), Some("Git is not installed."));

    std::fs::write(repo.join(".git/MERGE_HEAD"), "x\n").unwrap();
    let a = ops::availability(Ok(&git_bin), &repo);
    assert!(a.reason.unwrap().contains("in progress"));
    std::fs::remove_file(repo.join(".git/MERGE_HEAD")).unwrap();

    git(&repo, &["checkout", "-q", "--detach"]);
    let a = ops::availability(Ok(&git_bin), &repo);
    assert!(a.reason.unwrap().contains("detached"));
}

#[test]
fn lane_terminals_work_in_isolated_checkouts() {
    let sb = Sandbox::new();
    let repo = git_repo(&sb, "repo");
    let base = git(&repo, &["rev-parse", "main"]);
    let (app, _) = test_app(&sb);
    let ws = open_workspace(&app, &repo);
    let (alpha, la) = lane_terminal(&app, ws, "Alpha");
    let (beta, lb) = lane_terminal(&app, ws, "Beta");

    assert_eq!(la.branch, "lane/alpha");
    assert_eq!(la.base_branch, "main");
    assert_eq!(
        la.checkout_path,
        sb.root.lane_workspace_dir(ws).join("alpha")
    );
    assert_eq!(git(&repo, &["rev-parse", "lane/alpha"]), base);
    assert_eq!(lb.branch, "lane/beta");
    assert_eq!(shell_cwd(&app, &sb, alpha), common::real(&la.checkout_path));
    assert_eq!(shell_cwd(&app, &sb, beta), common::real(&lb.checkout_path));

    commit_file(&la.checkout_path, "a.txt", "alpha\n", "alpha");
    commit_file(&lb.checkout_path, "a.txt", "beta\n", "beta");
    let read = |p: &Path| std::fs::read_to_string(p.join("a.txt")).unwrap();
    assert_eq!(read(&repo), "one\n");
    assert_eq!(read(&la.checkout_path), "alpha\n");
    assert_eq!(read(&lb.checkout_path), "beta\n");
    app.quit();
}

#[test]
fn subfolder_workspace_runs_in_same_subfolder_of_lane() {
    let sb = Sandbox::new();
    let repo = git_repo(&sb, "repo");
    commit_file(&repo, "web/b.txt", "x\n", "web");
    let (app, _) = test_app(&sb);
    let ws = open_workspace(&app, &repo.join("web"));
    let (id, lane) = lane_terminal(&app, ws, "Web");
    assert_eq!(lane.subdirectory.as_deref(), Some(Path::new("web")));
    assert_eq!(
        shell_cwd(&app, &sb, id),
        common::real(&lane.checkout_path.join("web"))
    );
    app.quit();
}

#[test]
fn taken_names_get_a_counter() {
    let sb = Sandbox::new();
    let repo = git_repo(&sb, "repo");
    git(&repo, &["branch", "lane/alpha"]);
    let (app, _) = test_app(&sb);
    let ws = open_workspace(&app, &repo);
    let (_, lane) = lane_terminal(&app, ws, "Alpha");
    assert_eq!(lane.branch, "lane/alpha-2");
    assert!(lane.checkout_path.ends_with("alpha-2"));
    let (_, again) = lane_terminal(&app, ws, "Alpha");
    assert_eq!(again.branch, "lane/alpha-3");
    app.quit();
}

#[test]
fn not_a_repository_is_refused_without_side_effects() {
    let sb = Sandbox::new();
    let (app, _) = test_app(&sb);
    let ws = open_workspace(&app, &sb.project("plain"));
    let err = app
        .create_terminal(
            ws,
            NewTerminal {
                frame: frame(),
                lane: true,
                ..Default::default()
            },
        )
        .unwrap_err();
    assert_eq!(err.code, ErrorCode::LaneUnavailable);
    assert!(app.workspaces().workspace(ws).unwrap().nodes.is_empty());
    assert!(!sb.root.lane_workspace_dir(ws).exists());
    app.quit();
}

#[test]
fn lane_terminal_comes_back_in_its_checkout_after_restart() {
    let sb = Sandbox::new();
    let repo = git_repo(&sb, "repo");
    let (ws, id, lane) = {
        let (app, _) = test_app(&sb);
        let ws = open_workspace(&app, &repo);
        let (id, lane) = lane_terminal(&app, ws, "Alpha");
        app.save_dirty();
        app.quit();
        (ws, id, lane)
    };
    let (app, _) = test_app(&sb);
    app.open_workspace(ws).unwrap();
    assert_eq!(shell_cwd(&app, &sb, id), common::real(&lane.checkout_path));
    app.quit();
}

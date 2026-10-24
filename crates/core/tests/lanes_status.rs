mod common;

use std::time::Duration;

use common::{Sandbox, commit_file, git, git_repo, test_app, wait_until};
use sonata_core::app::NewTerminal;
use sonata_core::events::CoreEvent;
use sonata_core::lanes::git::Git;
use sonata_core::lanes::status::{LaneState, StatusCache, check};
use sonata_core::model::node::Lane;
use sonata_core::workspace::manager::CreateWorkspace;
use uuid::Uuid;

fn lane_in(sb: &Sandbox, repo: &std::path::Path, slug: &str) -> Lane {
    let path = sb.root.lane_workspace_dir(Uuid::nil()).join(slug);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let branch = format!("lane/{slug}");
    git(
        repo,
        &[
            "worktree",
            "add",
            "-q",
            "-b",
            &branch,
            path.to_str().unwrap(),
        ],
    );
    Lane {
        branch,
        base_branch: "main".into(),
        main_checkout: repo.to_path_buf(),
        checkout_path: path,
        subdirectory: None,
        created_at: chrono::Utc::now(),
        landed_at: None,
    }
}

#[test]
fn counts_changes_ahead_and_behind() {
    let sb = Sandbox::new();
    let repo = git_repo(&sb, "repo");
    let lane = lane_in(&sb, &repo, "api");
    let g = Git::new("git");
    let id = Uuid::new_v4();
    let s = check(Ok(&g), id, &lane);
    assert_eq!(s.state, LaneState::Ok);
    assert_eq!((s.changed, s.ahead, s.behind), (Some(0), Some(0), Some(0)));

    commit_file(&lane.checkout_path, "x.txt", "x\n", "x");
    commit_file(&repo, "y.txt", "y\n", "y");
    std::fs::write(lane.checkout_path.join("wip.txt"), "w\n").unwrap();
    let s = check(Ok(&g), id, &lane);
    assert_eq!((s.changed, s.ahead, s.behind), (Some(1), Some(1), Some(1)));
    assert_eq!(s.branch, "lane/api");
}

#[test]
fn reports_broken_states() {
    let sb = Sandbox::new();
    let repo = git_repo(&sb, "repo");
    let lane = lane_in(&sb, &repo, "api");
    let g = Git::new("git");
    let id = Uuid::new_v4();

    assert_eq!(
        check(Err("no git"), id, &lane).state,
        LaneState::GitUnavailable
    );

    let lane_git_dir = git(&lane.checkout_path, &["rev-parse", "--absolute-git-dir"]);
    std::fs::write(
        std::path::Path::new(&lane_git_dir).join("MERGE_HEAD"),
        "x\n",
    )
    .unwrap();
    assert_eq!(check(Ok(&g), id, &lane).state, LaneState::Busy);
    std::fs::remove_file(std::path::Path::new(&lane_git_dir).join("MERGE_HEAD")).unwrap();

    let mut other = lane.clone();
    other.branch = "lane/gone".into();
    assert_eq!(check(Ok(&g), id, &other).state, LaneState::BranchMissing);

    let mut landed = lane.clone();
    landed.landed_at = Some(chrono::Utc::now());
    assert_eq!(check(Ok(&g), id, &landed).state, LaneState::Landed);

    let mut moved = lane.clone();
    moved.main_checkout = sb.tmp.path().join("nowhere");
    assert_eq!(
        check(Ok(&g), id, &moved).state,
        LaneState::MainCheckoutMissing
    );

    std::fs::remove_dir_all(&lane.checkout_path).unwrap();
    let s = check(Ok(&g), id, &lane);
    assert_eq!(s.state, LaneState::CheckoutMissing);
    assert_eq!(s.changed, None);
}

#[test]
fn cache_sends_only_changes() {
    let sb = Sandbox::new();
    let repo = git_repo(&sb, "repo");
    let lane = lane_in(&sb, &repo, "api");
    let g = Git::new("git");
    let id = Uuid::new_v4();
    let mut cache = StatusCache::default();
    assert_eq!(cache.update(vec![check(Ok(&g), id, &lane)]).len(), 1);
    assert!(cache.update(vec![check(Ok(&g), id, &lane)]).is_empty());
    commit_file(&lane.checkout_path, "x.txt", "x\n", "x");
    let changed = cache.update(vec![check(Ok(&g), id, &lane)]);
    assert_eq!(changed[0].ahead, Some(1));
    cache.forget(&[id]);
    assert_eq!(cache.update(vec![check(Ok(&g), id, &lane)]).len(), 1);
}

#[test]
fn app_reports_a_commit_within_five_seconds() {
    let sb = Sandbox::new();
    let repo = git_repo(&sb, "repo");
    let (app, sink) = test_app(&sb);
    let ws = app
        .create_workspace(CreateWorkspace {
            name: "R".into(),
            working_directory: repo,
        })
        .unwrap()
        .id;
    app.open_workspace(ws).unwrap();
    let node = app
        .create_terminal(
            ws,
            NewTerminal {
                name: Some("Api".into()),
                lane: true,
                ..Default::default()
            },
        )
        .unwrap();
    let lane = node.terminal().unwrap().lane.as_deref().cloned().unwrap();
    let mut seen = Vec::new();
    assert!(wait_until(Duration::from_secs(5), || {
        seen.extend(sink.take());
        seen.iter()
            .any(|e| matches!(e, CoreEvent::LaneStatus(s) if s.node_id == node.id))
    }));
    commit_file(&lane.checkout_path, "x.txt", "x\n", "x");
    assert!(wait_until(Duration::from_secs(5), || {
        sink.take().iter().any(
            |e| matches!(e, CoreEvent::LaneStatus(s) if s.node_id == node.id && s.ahead == Some(1)),
        )
    }));
    app.quit();
}

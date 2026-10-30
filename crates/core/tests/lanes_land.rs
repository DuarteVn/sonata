mod common;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

use common::{Sandbox, WAIT, commit_file, git, git_repo, test_app, wait_until};
use sonata_core::App;
use sonata_core::ErrorCode;
use sonata_core::app::NewTerminal;
use sonata_core::lanes::git::Git;
use sonata_core::lanes::ops::{self, Action};
use sonata_core::model::node::Lane;
use sonata_core::workspace::manager::CreateWorkspace;
use uuid::Uuid;

/// A repository, an app with the repository's workspace open, and helpers to make lanes.
struct Setup {
    sb: Sandbox,
    repo: PathBuf,
    app: std::sync::Arc<App>,
    ws: Uuid,
}

fn setup() -> Setup {
    let sb = Sandbox::new();
    let repo = git_repo(&sb, "repo");
    let (app, _) = test_app(&sb);
    let ws = app
        .create_workspace(CreateWorkspace {
            name: "Repo".into(),
            working_directory: repo.clone(),
        })
        .unwrap()
        .id;
    app.open_workspace(ws).unwrap();
    Setup { sb, repo, app, ws }
}

impl Setup {
    fn lane(&self, name: &str) -> (Uuid, Lane) {
        let node = self
            .app
            .create_terminal(
                self.ws,
                NewTerminal {
                    name: Some(name.into()),
                    lane: true,
                    ..Default::default()
                },
            )
            .unwrap();
        let lane = node.terminal().unwrap().lane.as_deref().cloned().unwrap();
        (node.id, lane)
    }

    fn lane_of(&self, node: Uuid) -> Option<Lane> {
        self.app
            .workspaces()
            .workspace(self.ws)
            .and_then(|w| w.node(node).cloned())
            .and_then(|n| n.terminal().and_then(|t| t.lane.as_deref().cloned()))
    }
}

/// Every file under `dir` except `.git`, plus `HEAD` and the index state: what a failed land
/// must leave unchanged (SC-003).
fn snapshot(dir: &Path) -> BTreeMap<String, Vec<u8>> {
    let mut out = BTreeMap::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        for e in std::fs::read_dir(&d).unwrap() {
            let e = e.unwrap();
            let p = e.path();
            if e.file_name() == ".git" {
                continue;
            }
            if e.file_type().unwrap().is_dir() {
                stack.push(p);
            } else {
                let rel = p.strip_prefix(dir).unwrap().display().to_string();
                out.insert(rel, std::fs::read(&p).unwrap());
            }
        }
    }
    out.insert(
        "@HEAD".into(),
        git(dir, &["rev-parse", "HEAD"]).into_bytes(),
    );
    out.insert(
        "@status".into(),
        git(dir, &["status", "--porcelain=v2", "--untracked-files=all"]).into_bytes(),
    );
    out
}

#[test]
fn preview_counts_and_blockers() {
    let s = setup();
    let g = Git::new("git");
    let (_, lane) = s.lane("Api");
    commit_file(&lane.checkout_path, "x.txt", "x\n", "x");
    commit_file(&lane.checkout_path, "y.txt", "y\n", "y");
    let p = ops::preview(&g, &lane, Action::Land);
    assert_eq!(p.commits, 2);
    assert!(p.blockers.is_empty(), "{:?}", p.blockers);

    std::fs::write(lane.checkout_path.join("new.txt"), "n\n").unwrap();
    let p = ops::preview(&g, &lane, Action::Land);
    assert_eq!(p.changed_files, ["new.txt"]);
    assert_eq!(p.changed_total, 1);
    assert!(
        p.blockers.iter().any(|b| b.contains("uncommitted")),
        "{:?}",
        p.blockers
    );
    std::fs::remove_file(lane.checkout_path.join("new.txt")).unwrap();

    std::fs::write(s.repo.join("scratch.txt"), "untracked in main is fine\n").unwrap();
    assert!(ops::preview(&g, &lane, Action::Land).blockers.is_empty());
    std::fs::write(s.repo.join("a.txt"), "dirty\n").unwrap();
    let p = ops::preview(&g, &lane, Action::Land);
    assert!(
        p.blockers.iter().any(|b| b.contains("workspace folder")),
        "{:?}",
        p.blockers
    );
    git(&s.repo, &["checkout", "-q", "--", "a.txt"]);

    git(&s.repo, &["checkout", "-q", "-b", "other"]);
    let p = ops::preview(&g, &lane, Action::Land);
    assert!(
        p.blockers
            .iter()
            .any(|b| b.contains("other") && b.contains("main")),
        "{:?}",
        p.blockers
    );
    git(&s.repo, &["checkout", "-q", "main"]);

    std::fs::write(s.repo.join(".git/MERGE_HEAD"), "x\n").unwrap();
    let p = ops::preview(&g, &lane, Action::Land);
    assert!(
        p.blockers.iter().any(|b| b.contains("in progress")),
        "{:?}",
        p.blockers
    );
    std::fs::remove_file(s.repo.join(".git/MERGE_HEAD")).unwrap();

    let mut gone = lane.clone();
    gone.branch = "lane/missing".into();
    let p = ops::preview(&g, &gone, Action::Land);
    assert!(
        p.blockers.iter().any(|b| b.contains("no longer exists")),
        "{:?}",
        p.blockers
    );
    s.app.quit();
}

#[test]
fn land_makes_a_merge_commit() {
    let s = setup();
    let g = Git::new("git");
    let (_, lane) = s.lane("Api");
    commit_file(&lane.checkout_path, "a.txt", "api\n", "api");
    assert_eq!(ops::land(&g, &lane).unwrap(), 1);
    assert_eq!(
        std::fs::read_to_string(s.repo.join("a.txt")).unwrap(),
        "api\n"
    );
    assert_eq!(
        git(&s.repo, &["log", "-1", "--format=%s"]),
        "Merge lane lane/api into main"
    );
    assert_eq!(
        git(&s.repo, &["rev-list", "--parents", "-n1", "HEAD"])
            .split(' ')
            .count(),
        3
    );
    let err = ops::land(&g, &lane).unwrap_err();
    assert_eq!(err.code, ErrorCode::LaneBlocked);
    assert!(err.message.contains("Nothing to land"), "{}", err.message);
    s.app.quit();
}

#[test]
fn conflict_changes_nothing() {
    let s = setup();
    let g = Git::new("git");
    let (_, lane) = s.lane("Api");
    commit_file(&lane.checkout_path, "a.txt", "lane\n", "lane");
    commit_file(&s.repo, "a.txt", "main\n", "main");
    let before_main = snapshot(&s.repo);
    let before_lane = snapshot(&lane.checkout_path);
    let err = ops::land(&g, &lane).unwrap_err();
    assert_eq!(err.code, ErrorCode::LaneConflict);
    assert!(err.message.contains("a.txt"), "{}", err.message);
    assert_eq!(snapshot(&s.repo), before_main);
    assert_eq!(snapshot(&lane.checkout_path), before_lane);
    s.app.quit();
}

#[test]
fn untracked_file_in_the_way_fails_without_moving_head() {
    let s = setup();
    let g = Git::new("git");
    let (_, lane) = s.lane("Api");
    commit_file(&lane.checkout_path, "c.txt", "lane\n", "add c");
    std::fs::write(s.repo.join("c.txt"), "mine\n").unwrap();
    let head = git(&s.repo, &["rev-parse", "HEAD"]);
    let err = ops::land(&g, &lane).unwrap_err();
    assert_eq!(err.code, ErrorCode::LaneBlocked);
    assert_eq!(git(&s.repo, &["rev-parse", "HEAD"]), head);
    assert_eq!(
        std::fs::read_to_string(s.repo.join("c.txt")).unwrap(),
        "mine\n"
    );
    assert!(!s.repo.join(".git/MERGE_HEAD").exists());
    s.app.quit();
}

#[test]
fn concurrent_lands_run_one_after_the_other() {
    let s = setup();
    let lanes: Vec<Lane> = (0..4)
        .map(|i| {
            let (_, lane) = s.lane(&format!("L{i}"));
            commit_file(&lane.checkout_path, &format!("f{i}.txt"), "x\n", "work");
            lane
        })
        .collect();
    let handles: Vec<_> = lanes
        .into_iter()
        .map(|lane| std::thread::spawn(move || ops::land(&Git::new("git"), &lane)))
        .collect();
    for h in handles {
        h.join().unwrap().unwrap();
    }
    let merges = git(&s.repo, &["rev-list", "--merges", "--count", "HEAD"]);
    assert_eq!(merges, "4");
    s.app.quit();
}

#[test]
fn app_land_requires_confirmation_then_marks_landed() {
    let s = setup();
    let (id, lane) = s.lane("Api");
    commit_file(&lane.checkout_path, "a.txt", "api\n", "api");

    let err = s.app.lane_land(id, false).unwrap_err();
    assert_eq!(err.code, ErrorCode::ConfirmationRequired);
    assert_eq!(
        std::fs::read_to_string(s.repo.join("a.txt")).unwrap(),
        "one\n"
    );

    assert_eq!(s.app.lane_land(id, true).unwrap(), 1);
    assert_eq!(
        std::fs::read_to_string(s.repo.join("a.txt")).unwrap(),
        "api\n"
    );
    assert!(!lane.checkout_path.exists());
    assert!(!git(&s.repo, &["worktree", "list"]).contains("lanes"));
    assert_eq!(
        git(&s.repo, &["branch", "--list", "lane/api"]).trim(),
        "lane/api"
    );
    let landed = s.lane_of(id).expect("lane record kept");
    assert!(landed.landed_at.is_some());
    assert!(!s.app.terminals().contains(id), "terminal stopped");

    s.app.restart_terminal(id, true).unwrap();
    assert!(s.lane_of(id).is_none(), "restart clears a landed lane");
    let out = s.sb.tmp.path().join("cwd");
    assert!(wait_until(WAIT, || s
        .app
        .terminal_ready(id, Duration::from_millis(300))));
    s.app
        .terminals()
        .write(id, common::cwd_command(&out).as_bytes())
        .unwrap();
    assert!(wait_until(WAIT, || std::fs::read_to_string(&out)
        .is_ok_and(|t| t.ends_with('\n'))));
    assert_eq!(
        common::real(std::path::Path::new(
            std::fs::read_to_string(&out).unwrap().trim()
        )),
        common::real(&s.repo)
    );
    s.app.quit();
}

#[test]
fn landed_terminal_stays_stopped_after_reopen() {
    let s = setup();
    let (id, lane) = s.lane("Api");
    commit_file(&lane.checkout_path, "a.txt", "api\n", "api");
    s.app.lane_land(id, true).unwrap();
    s.app.save_dirty();
    s.app.quit();
    let (app, _) = test_app(&s.sb);
    app.open_workspace(s.ws).unwrap();
    std::thread::sleep(Duration::from_millis(300));
    assert!(!app.terminals().contains(id));
    app.quit();
}

mod common;

use std::path::PathBuf;
use std::time::Duration;

use common::{Sandbox, WAIT, commit_file, git, git_repo, test_app, wait_until};
use sonata_core::App;
use sonata_core::ErrorCode;
use sonata_core::app::{LaneRemoval, NewTerminal};
use sonata_core::model::node::{Lane, Node};
use sonata_core::workspace::manager::CreateWorkspace;
use uuid::Uuid;

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
        (
            node.id,
            node.terminal().unwrap().lane.as_deref().cloned().unwrap(),
        )
    }

    fn node(&self, id: Uuid) -> Option<Node> {
        self.app
            .workspaces()
            .workspace(self.ws)
            .and_then(|w| w.node(id).cloned())
    }

    fn branches(&self) -> String {
        git(&self.repo, &["branch", "--list", "lane/*"])
    }
}

const KEEP: LaneRemoval = LaneRemoval {
    discard_changes: false,
    delete_branch: false,
};

#[test]
fn clean_lane_is_removed_and_branch_kept() {
    let s = setup();
    let (id, lane) = s.lane("Api");
    let err = s.app.lane_remove(id, KEEP, false).unwrap_err();
    assert_eq!(err.code, ErrorCode::ConfirmationRequired);
    assert!(lane.checkout_path.exists());

    s.app.lane_remove(id, KEEP, true).unwrap();
    assert!(!lane.checkout_path.exists());
    assert!(!git(&s.repo, &["worktree", "list"]).contains("lanes"));
    assert!(s.branches().contains("lane/api"));
    let t = s.node(id).unwrap();
    assert!(t.terminal().unwrap().lane.is_none());
    assert!(
        !s.sb.root.lane_workspace_dir(s.ws).exists(),
        "empty folder removed"
    );

    let out = s.sb.tmp.path().join("cwd");
    assert!(wait_until(WAIT, || s
        .app
        .terminal_ready(id, Duration::from_millis(300))));
    s.app
        .terminals()
        .write(id, format!("pwd -P > '{}'\n", out.display()).as_bytes())
        .unwrap();
    assert!(wait_until(WAIT, || std::fs::read_to_string(&out)
        .is_ok_and(|t| t.ends_with('\n'))));
    assert_eq!(
        std::fs::read_to_string(&out).unwrap().trim(),
        s.repo.canonicalize().unwrap().display().to_string()
    );
    s.app.quit();
}

#[test]
fn changes_need_explicit_discard() {
    let s = setup();
    let (id, lane) = s.lane("Api");
    std::fs::write(lane.checkout_path.join("new.txt"), "n\n").unwrap();
    let err = s.app.lane_remove(id, KEEP, true).unwrap_err();
    assert_eq!(err.code, ErrorCode::LaneHasChanges);
    assert!(err.message.contains("new.txt"), "{}", err.message);
    assert!(lane.checkout_path.join("new.txt").exists());
    assert!(s.app.terminals().contains(id), "terminal untouched");

    s.app
        .lane_remove(
            id,
            LaneRemoval {
                discard_changes: true,
                delete_branch: false,
            },
            true,
        )
        .unwrap();
    assert!(!lane.checkout_path.exists());
    s.app.quit();
}

#[test]
fn branch_deletion_is_opt_in() {
    let s = setup();
    let delete = LaneRemoval {
        discard_changes: false,
        delete_branch: true,
    };
    let (merged, _) = s.lane("Merged");
    s.app.lane_remove(merged, delete, true).unwrap();
    assert!(!s.branches().contains("lane/merged"));

    let (work, lane) = s.lane("Work");
    commit_file(&lane.checkout_path, "w.txt", "w\n", "work");
    let p = s
        .app
        .lane_preview(work, sonata_core::lanes::ops::Action::Remove)
        .unwrap();
    assert_eq!(p.unmerged_commits, 1);
    s.app.lane_remove(work, delete, true).unwrap();
    assert!(!s.branches().contains("lane/work"));
    s.app.quit();
}

#[test]
fn hand_edited_paths_are_refused() {
    let s = setup();
    let (id, lane) = s.lane("Api");
    for bad in [s.repo.clone(), s.sb.tmp.path().join("projects")] {
        s.app
            .workspaces()
            .update(s.ws, |w| {
                let t = w.node_mut(id).unwrap().terminal_mut().unwrap();
                t.lane.as_mut().unwrap().checkout_path = bad.clone();
                Ok(())
            })
            .unwrap();
        let err = s.app.lane_remove(id, KEEP, true).unwrap_err();
        assert_eq!(err.code, ErrorCode::LaneUnsafePath, "{}", bad.display());
        assert!(bad.exists());
        assert!(s.repo.join("a.txt").exists());
    }
    assert!(lane.checkout_path.exists());
    s.app.quit();
}

#[test]
fn missing_folder_can_be_forgotten() {
    let s = setup();
    let (id, lane) = s.lane("Api");
    let err = s.app.lane_forget(id).unwrap_err();
    assert_eq!(err.code, ErrorCode::LaneBlocked);
    std::fs::remove_dir_all(&lane.checkout_path).unwrap();
    s.app.lane_forget(id).unwrap();
    assert!(s.node(id).unwrap().terminal().unwrap().lane.is_none());
    assert!(s.branches().contains("lane/api"));
    assert!(!git(&s.repo, &["worktree", "list"]).contains("lanes"));
    s.app.quit();
}

#[test]
fn deleting_a_lane_terminal_asks_then_removes() {
    let s = setup();
    let (id, lane) = s.lane("Api");
    let err = s.app.delete_node(s.ws, id, true).unwrap_err();
    assert_eq!(err.code, ErrorCode::LaneConfirmationNeeded);
    assert!(s.node(id).is_some());
    s.app.delete_node_with_lane(s.ws, id, true, KEEP).unwrap();
    assert!(s.node(id).is_none());
    assert!(!lane.checkout_path.exists());
    assert!(s.branches().contains("lane/api"));
    s.app.quit();
}

#[test]
fn deleting_a_workspace_blocks_on_dirty_lanes_and_removes_clean_ones() {
    let s = setup();
    let (_, clean) = s.lane("Clean");
    let (dirty_id, dirty) = s.lane("Dirty");
    std::fs::write(dirty.checkout_path.join("wip.txt"), "x\n").unwrap();
    let err = s.app.delete_workspace(s.ws).unwrap_err();
    assert_eq!(err.code, ErrorCode::LaneHasChanges);
    assert!(err.message.contains("Dirty"), "{}", err.message);
    assert!(clean.checkout_path.exists() && dirty.checkout_path.exists());
    assert!(s.app.workspaces().workspace(s.ws).is_some());

    std::fs::remove_file(dirty.checkout_path.join("wip.txt")).unwrap();
    let _ = dirty_id;
    s.app.delete_workspace(s.ws).unwrap();
    assert!(!clean.checkout_path.exists() && !dirty.checkout_path.exists());
    assert!(!s.sb.root.lane_workspace_dir(s.ws).exists());
    assert!(s.branches().contains("lane/clean"));
    s.app.quit();
}

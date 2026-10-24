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

// ---- `sonata lane …` with a human answer simulated by the event sink (US5) ----

use std::sync::{Arc, Mutex, OnceLock, Weak};
use std::time::Duration;

use common::{WAIT, cli_bin, commit_file, git, wait_until};
use sonata_core::app::{AppOptions, NewTerminal};
use sonata_core::events::{CoreEvent, EventSink, LaneConfirmEvent};
use sonata_core::lanes::confirm::Answer;

/// Answers every `lane://confirm` like a human would, with `reply` (or never, when `None`).
#[derive(Default)]
struct Human {
    app: OnceLock<Weak<App>>,
    reply: Mutex<Option<Answer>>,
    asked: Mutex<Vec<LaneConfirmEvent>>,
}

impl EventSink for Human {
    fn emit(&self, event: CoreEvent) {
        let CoreEvent::LaneConfirm(e) = event else {
            return;
        };
        self.asked.lock().unwrap().push(e.clone());
        let Some(answer) = *self.reply.lock().unwrap() else {
            return;
        };
        let app = self.app.get().cloned();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(100));
            if let Some(app) = app.and_then(|w| w.upgrade()) {
                app.answer_lane_confirmation(e.request_id, answer).unwrap();
            }
        });
    }
}

struct World {
    _sb: Sandbox,
    repo: std::path::PathBuf,
    app: Arc<App>,
    human: Arc<Human>,
    ws: Uuid,
    lead: Uuid,
    api: Uuid,
}

fn world() -> World {
    let sb = Sandbox::new();
    let repo = git_repo(&sb, "p");
    let human = Arc::new(Human::default());
    let opts = AppOptions {
        shell: Some("/bin/sh".into()),
        cli_dir: Some(cli_bin().parent().unwrap().to_path_buf()),
        socket: true,
        ..AppOptions::default()
    };
    let app = App::start_with(sb.root.clone(), human.clone(), opts).unwrap();
    human.app.set(Arc::downgrade(&app)).unwrap();
    let (ws, ids) = shells(&app, &sb, &["Lead"]);
    let api = app
        .create_terminal(
            ws,
            NewTerminal {
                name: Some("Api".into()),
                lane: true,
                ..Default::default()
            },
        )
        .unwrap()
        .id;
    app.create_link(ws, ids[0], api).unwrap();
    assert!(wait_until(WAIT, || app.terminal_ready(api, Duration::from_millis(300))));
    World {
        _sb: sb,
        repo,
        app,
        human,
        ws,
        lead: ids[0],
        api,
    }
}

impl World {
    fn reply(&self, answer: Option<Answer>) {
        *self.human.reply.lock().unwrap() = answer;
    }

    fn lane(&self, node: Uuid) -> Option<sonata_core::model::node::Lane> {
        self.app
            .workspaces()
            .workspace(self.ws)
            .and_then(|w| w.node(node).cloned())
            .and_then(|n| n.terminal().and_then(|t| t.lane.as_deref().cloned()))
    }

    fn commit_in_lane(&self) {
        let lane = self.lane(self.api).unwrap();
        commit_file(&lane.checkout_path, "a.txt", "api\n", "api");
    }
}

const YES: Answer = Answer {
    accepted: true,
    discard_changes: false,
    delete_branch: false,
};

#[test]
fn lane_status_for_self_and_linked() {
    let w = world();
    let own = sonata_as(&w.app, w.api, &["lane", "status"]);
    assert_eq!(own.code, 0, "{}", own.stderr);
    assert!(
        own.stdout.contains("lane/api") && own.stdout.contains("base main"),
        "{}",
        own.stdout
    );
    let none = sonata_as(&w.app, w.lead, &["lane", "status"]);
    assert_eq!(none.code, 4, "{}", none.stderr);
    let linked = sonata_as(&w.app, w.lead, &["lane", "status", "Api"]);
    assert_eq!(linked.code, 0, "{}", linked.stderr);
    w.app.quit();
}

#[test]
fn only_an_orchestrator_asks_for_another_lane() {
    let w = world();
    w.commit_in_lane();
    let out = sonata_as(&w.app, w.lead, &["lane", "land", "Api"]);
    assert_eq!(out.code, 3, "{}", out.stderr);
    assert!(w.human.asked.lock().unwrap().is_empty());
    w.app.quit();
}

#[test]
fn declined_land_changes_nothing() {
    let w = world();
    w.commit_in_lane();
    w.reply(Some(Answer::default()));
    let out = sonata_as(&w.app, w.api, &["lane", "land"]);
    assert_eq!(out.code, 3, "{}", out.stderr);
    assert!(out.stderr.contains("declined"), "{}", out.stderr);
    assert_eq!(
        std::fs::read_to_string(w.repo.join("a.txt")).unwrap(),
        "one\n"
    );
    let asked = w.human.asked.lock().unwrap().clone();
    assert_eq!(asked.len(), 1);
    assert_eq!(asked[0].requested_by, "Api");
    assert_eq!(asked[0].preview.commits, 1);
    w.app.quit();
}

#[test]
fn accepted_own_land_replies_before_the_terminal_stops() {
    let w = world();
    w.commit_in_lane();
    w.reply(Some(YES));
    let out = sonata_as(&w.app, w.api, &["lane", "land"]);
    assert_eq!(out.code, 0, "{}", out.stderr);
    assert!(
        out.stdout.contains("Landed lane/api into main (1 commit)"),
        "{}",
        out.stdout
    );
    assert!(
        w.app.terminals().contains(w.api),
        "still running when the reply arrives"
    );
    assert_eq!(
        std::fs::read_to_string(w.repo.join("a.txt")).unwrap(),
        "api\n"
    );
    assert!(wait_until(WAIT, || !w.app.terminals().contains(w.api)));
    assert!(wait_until(WAIT, || w
        .lane(w.api)
        .is_some_and(|l| l.landed_at.is_some())));
    w.app.quit();
}

#[test]
fn refused_before_asking_when_blocked() {
    let w = world();
    let lane = w.lane(w.api).unwrap();
    std::fs::write(lane.checkout_path.join("wip.txt"), "x\n").unwrap();
    w.reply(Some(YES));
    let out = sonata_as(&w.app, w.api, &["lane", "land"]);
    assert_eq!(out.code, 1, "{}", out.stderr);
    assert!(out.stderr.contains("uncommitted"), "{}", out.stderr);
    assert!(w.human.asked.lock().unwrap().is_empty());
    w.app.quit();
}

#[test]
fn orchestrator_removes_a_linked_lane_and_its_branch() {
    let w = world();
    make_orchestrator(&w.app, w.lead);
    w.reply(Some(Answer {
        delete_branch: true,
        ..YES
    }));
    let out = sonata_as(
        &w.app,
        w.lead,
        &["lane", "remove", "Api", "--delete-branch"],
    );
    assert_eq!(out.code, 0, "{}", out.stderr);
    assert!(w.human.asked.lock().unwrap()[0].delete_branch);
    assert!(wait_until(WAIT, || w.lane(w.api).is_none()));
    assert!(git(&w.repo, &["branch", "--list", "lane/api"]).is_empty());
    w.app.quit();
}

#[test]
fn dismissing_a_lane_terminal_asks_first() {
    let w = world();
    make_orchestrator(&w.app, w.lead);
    let r = sonata_as(
        &w.app,
        w.lead,
        &["recruit", "shell", "--name", "Helper", "--lane"],
    );
    assert_eq!(r.code, 0, "{}", r.stderr);
    let helper = node_named(&w.app, w.ws, "Helper");
    let checkout = helper
        .terminal()
        .unwrap()
        .lane
        .as_deref()
        .unwrap()
        .checkout_path
        .clone();

    w.reply(Some(Answer::default()));
    let no = sonata_as(&w.app, w.lead, &["dismiss", "Helper"]);
    assert_eq!(no.code, 3, "{}", no.stderr);
    assert!(checkout.exists());

    w.reply(Some(YES));
    let yes = sonata_as(&w.app, w.lead, &["dismiss", "Helper"]);
    assert_eq!(yes.code, 0, "{}", yes.stderr);
    assert!(!checkout.exists());
    assert!(
        w.app
            .workspaces()
            .workspace(w.ws)
            .unwrap()
            .node(helper.id)
            .is_none()
    );
    w.app.quit();
}

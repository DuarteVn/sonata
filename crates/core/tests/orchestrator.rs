mod common;

use common::{Sandbox, messaging_app, shells, sonata_as};
use sonata_core::app::TerminalSettings;
use sonata_core::events::CoreEvent;

#[test]
fn orchestrator_recruits_links_and_dismisses() {
    let sb = Sandbox::new();
    let (app, sink) = messaging_app(&sb);
    let (ws, ids) = shells(&app, &sb, &["Lead", "Plain"]);
    let (lead, plain) = (ids[0], ids[1]);

    for args in [
        vec!["recruit", "shell"],
        vec!["dismiss", "x"],
        vec!["link", "a", "b"],
        vec!["unlink", "a", "b"],
    ] {
        let out = sonata_as(&app, plain, &args);
        assert_eq!(out.code, 3, "{args:?}: {}", out.stderr);
        assert!(
            out.stderr.contains("Only the Orchestrator"),
            "{}",
            out.stderr
        );
    }

    app.update_terminal_settings(
        lead,
        TerminalSettings {
            orchestrator: Some(true),
            ..Default::default()
        },
    )
    .unwrap();
    app.role_save("Reviewer", "Review diffs carefully.")
        .unwrap();
    sink.take();

    let r = sonata_as(
        &app,
        lead,
        &["recruit", "shell", "--name", "Helper", "--role", "reviewer"],
    );
    assert_eq!(r.code, 0, "{}", r.stderr);
    let r2 = sonata_as(
        &app,
        lead,
        &[
            "recruit",
            "shell",
            "--name",
            "Helper",
            "--role-text",
            "Second helper",
            "--folder",
            ".",
        ],
    );
    assert_eq!(r2.code, 0, "{}", r2.stderr);
    assert!(r2.stdout.contains("Helper 2"), "{}", r2.stdout);

    let w = app.workspaces().workspace(ws).unwrap().clone();
    let helper = w
        .nodes
        .iter()
        .find(|n| n.terminal().is_some_and(|t| t.name == "Helper"))
        .unwrap();
    let helper2 = w
        .nodes
        .iter()
        .find(|n| n.terminal().is_some_and(|t| t.name == "Helper 2"))
        .unwrap();
    let t = helper.terminal().unwrap();
    assert_eq!(t.recruited_by, Some(lead));
    assert_eq!(t.role.as_ref().unwrap().name.as_deref(), Some("Reviewer"));
    assert_eq!(
        helper2.terminal().unwrap().role.as_ref().unwrap().text,
        "Second helper"
    );
    assert!(w.links.iter().any(|l| l.joins(lead, helper.id)));
    for (i, a) in w.nodes.iter().enumerate() {
        for b in &w.nodes[i + 1..] {
            let (fa, fb) = (a.frame, b.frame);
            let overlap = fa.x < fb.x + fb.width
                && fb.x < fa.x + fa.width
                && fa.y < fb.y + fb.height
                && fb.y < fa.y + fa.height;
            assert!(!overlap, "recruits overlap other nodes");
        }
    }
    assert!(
        sink.take()
            .iter()
            .any(|e| matches!(e, CoreEvent::NodesChanged(_)))
    );

    // Ask the recruit straight away.
    let asked = sonata_as(&app, lead, &["ask", "Helper", "echo recruited-ok"]);
    assert_eq!(asked.code, 0, "{}", asked.stderr);
    assert!(asked.stdout.contains("recruited-ok"));

    // link / unlink by name; dismiss only own recruits.
    let l = sonata_as(&app, lead, &["link", "Helper", "Plain"]);
    assert_eq!(l.code, 0, "{}", l.stderr);
    assert!(
        app.links_of(ws, plain)
            .iter()
            .any(|x| x.joins(plain, helper.id))
    );
    let u = sonata_as(&app, lead, &["unlink", "Helper", "Plain"]);
    assert_eq!(u.code, 0, "{}", u.stderr);
    app.create_link(ws, lead, plain).unwrap();
    let refused = sonata_as(&app, lead, &["dismiss", "Plain"]);
    assert_eq!(refused.code, 3, "{}", refused.stderr);
    let d = sonata_as(&app, lead, &["dismiss", "Helper"]);
    assert_eq!(d.code, 0, "{}", d.stderr);
    assert!(
        app.workspaces()
            .workspace(ws)
            .unwrap()
            .node(helper.id)
            .is_none()
    );
    assert!(app.terminals().status(helper.id).is_none());
    app.quit();
}

#[test]
fn orchestrator_creates_linked_notes_and_browsers() {
    let sb = Sandbox::new();
    let (app, _) = messaging_app(&sb);
    let (ws, ids) = shells(&app, &sb, &["Lead", "Plain"]);
    let (lead, plain) = (ids[0], ids[1]);
    assert_eq!(sonata_as(&app, plain, &["note", "new", "Plan"]).code, 3);
    assert_eq!(sonata_as(&app, plain, &["browser", "new"]).code, 3);
    app.update_terminal_settings(
        lead,
        TerminalSettings {
            orchestrator: Some(true),
            ..Default::default()
        },
    )
    .unwrap();

    let n = sonata_as(&app, lead, &["note", "new", "Plan", "- step one"]);
    assert_eq!(n.code, 0, "{}", n.stderr);
    let read = sonata_as(&app, lead, &["note", "read", "Plan"]);
    assert_eq!(read.stdout.trim(), "# Plan\n\n- step one");

    let b = sonata_as(&app, lead, &["browser", "new", "localhost:3000"]);
    assert_eq!(b.code, 0, "{}", b.stderr);
    let w = app.workspaces().workspace(ws).unwrap().clone();
    let browser = w.nodes.iter().find(|n| n.browser().is_some()).unwrap();
    assert_eq!(browser.browser().unwrap().url, "http://localhost:3000");
    assert!(w.links.iter().any(|l| l.joins(lead, browser.id)));
    assert_eq!(
        sonata_as(&app, lead, &["browser", "new", "file:///x"]).code,
        2
    );
    let list = sonata_as(&app, lead, &["list"]);
    assert!(
        list.stdout.contains("note") && list.stdout.contains("browser"),
        "{}",
        list.stdout
    );
    app.quit();
}

#[test]
fn help_and_unknown_options_never_create_anything() {
    let sb = Sandbox::new();
    let (app, _) = messaging_app(&sb);
    let (ws, ids) = shells(&app, &sb, &["Lead"]);
    app.update_terminal_settings(
        ids[0],
        TerminalSettings {
            orchestrator: Some(true),
            ..Default::default()
        },
    )
    .unwrap();
    let before = app.workspaces().workspace(ws).unwrap().nodes.len();
    let help = sonata_as(&app, ids[0], &["recruit", "--help"]);
    assert_eq!(help.code, 0);
    assert!(help.stdout.contains("sonata recruit"), "{}", help.stdout);
    for args in [
        vec!["recruit", "-h"],
        vec!["note", "new", "--help"],
        vec!["browser", "new", "--help"],
    ] {
        assert_eq!(sonata_as(&app, ids[0], &args).code, 0);
    }
    assert_eq!(sonata_as(&app, ids[0], &["recruit", "--verbose"]).code, 2);
    assert_eq!(
        app.workspaces().workspace(ws).unwrap().nodes.len(),
        before,
        "something was created"
    );
    app.quit();
}

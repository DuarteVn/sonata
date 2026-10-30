// Unix shells, modes, or sockets; Windows counterparts live in *_windows.rs (feature 012).
#![cfg(unix)]
mod common;

use std::time::{Duration, Instant};

use common::{Sandbox, messaging_app, shells, sonata_as, sonata_with, wait_until};

#[test]
fn ask_between_linked_shells() {
    let sb = Sandbox::new();
    let (app, _) = messaging_app(&sb);
    let (ws, ids) = shells(&app, &sb, &["A", "B"]);
    let (a, b) = (ids[0], ids[1]);

    let unlinked = sonata_as(&app, a, &["ask", "B", "echo hello"]);
    assert_eq!(unlinked.code, 3, "{}", unlinked.stderr);
    assert!(unlinked.stderr.contains("linked to"), "{}", unlinked.stderr);

    app.create_link(ws, a, b).unwrap();
    let out = sonata_as(&app, a, &["ask", "b", "echo hello-$((40+2))"]);
    assert_eq!(out.code, 0, "{}", out.stderr);
    assert_eq!(out.stdout.trim(), "hello-42");

    let multi = sonata_as(&app, a, &["ask", "B", "printf 'one\\n'\necho two"]);
    assert_eq!(multi.code, 0, "{}", multi.stderr);
    assert!(multi.stdout.contains("two"), "{}", multi.stdout);

    let screen = sonata_as(&app, a, &["check", "B"]);
    assert_eq!(screen.code, 0);
    assert!(screen.stdout.contains("hello-42"));

    let big = "x".repeat(101 * 1024);
    let refused = sonata_with(&app, a, &["ask", "B", "--stdin"], Some(&big));
    assert_eq!(refused.code, 2);
    app.quit();
}

#[test]
fn parallel_asks_keys_and_reply() {
    let sb = Sandbox::new();
    let (app, _) = messaging_app(&sb);
    let (ws, ids) = shells(&app, &sb, &["Lead", "B", "C"]);
    app.create_link(ws, ids[0], ids[1]).unwrap();
    app.create_link(ws, ids[0], ids[2]).unwrap();

    let start = Instant::now();
    let all = sonata_as(&app, ids[0], &["ask", "--all", "sleep 1; echo done-$0"]);
    assert_eq!(all.code, 0, "{}", all.stderr);
    assert!(
        all.stdout.contains("## B") && all.stdout.contains("## C"),
        "{}",
        all.stdout
    );
    assert!(
        start.elapsed() < Duration::from_secs(6),
        "not parallel: {:?}",
        start.elapsed()
    );

    // Keys: interrupt a running `cat`.
    app.terminals().write(ids[1], b"cat\r").unwrap();
    assert!(wait_until(common::WAIT, || app.terminals().is_busy(ids[1])));
    let keys = sonata_as(&app, ids[0], &["ask", "B", "--keys", "ctrl-c"]);
    assert_eq!(keys.code, 0, "{}", keys.stderr);
    assert!(wait_until(common::WAIT, || !app
        .terminals()
        .is_busy(ids[1])));
    app.quit();
}

#[test]
fn reply_completes_the_pending_ask() {
    let sb = Sandbox::new();
    let (app, _) = messaging_app(&sb);
    let (ws, ids) = shells(&app, &sb, &["A", "B"]);
    app.create_link(ws, ids[0], ids[1]).unwrap();
    let nobody = sonata_as(&app, ids[1], &["reply", "hi"]);
    assert_eq!(nobody.code, 4);
    let app2 = app.clone();
    let b = ids[1];
    let replier = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(1500));
        sonata_as(&app2, b, &["reply", "the full answer"])
    });
    let out = sonata_as(&app, ids[0], &["ask", "B", "sleep 30"]);
    assert_eq!(out.stdout.trim(), "the full answer", "{}", out.stderr);
    assert_eq!(replier.join().unwrap().code, 0);
    app.terminals().write(b, b"\x03").unwrap();
    app.quit();
}

#[test]
fn loops_are_refused_and_timeouts_return_partial() {
    let sb = Sandbox::new();
    let (app, _) = messaging_app(&sb);
    let (ws, ids) = shells(&app, &sb, &["A", "B"]);
    app.create_link(ws, ids[0], ids[1]).unwrap();
    let (a, b) = (ids[0], ids[1]);
    let app2 = app.clone();
    let waiting = std::thread::spawn(move || {
        sonata_as(
            &app2,
            a,
            &["ask", "B", "echo started; sleep 20", "--timeout", "6s"],
        )
    });
    std::thread::sleep(Duration::from_millis(1500));
    let looped = sonata_as(&app, b, &["ask", "A", "echo hi"]);
    assert_eq!(looped.code, 3, "{}", looped.stderr);
    assert!(looped.stderr.contains("loop"), "{}", looped.stderr);
    let timed = waiting.join().unwrap();
    assert_eq!(timed.code, 5, "{} {}", timed.stdout, timed.stderr);
    assert!(timed.stdout.contains("started"), "{}", timed.stdout);
    assert!(timed.stdout.contains("timed out"), "{}", timed.stdout);
    app.quit();
}

#[test]
fn asking_a_terminal_in_a_closed_workspace_starts_it() {
    let sb = Sandbox::new();
    let ws_b;
    let b;
    {
        let (app, _) = messaging_app(&sb);
        let (ws, ids) = shells(&app, &sb, &["A", "B"]);
        app.create_link(ws, ids[0], ids[1]).unwrap();
        ws_b = ws;
        b = ids[1];
        app.quit();
    }
    let (app, _) = messaging_app(&sb);
    // Only A's workspace is "open" via a single start: open the workspace but stop B.
    app.open_workspace(ws_b).unwrap();
    let a = app
        .workspaces()
        .workspace(ws_b)
        .unwrap()
        .nodes
        .iter()
        .find(|n| n.id != b)
        .unwrap()
        .id;
    assert!(wait_until(common::WAIT, || app
        .terminal_ready(a, Duration::from_millis(300))));
    app.terminals().write(b, b"exit\r").unwrap();
    assert!(wait_until(common::WAIT, || matches!(
        app.terminals().status(b),
        Some(sonata_core::terminal::manager::RunStatus::Exited { .. })
    )));
    let out = sonata_as(&app, a, &["ask", "B", "echo back-again"]);
    assert_eq!(out.code, 0, "{}", out.stderr);
    assert!(out.stdout.contains("back-again"));
    app.quit();
}

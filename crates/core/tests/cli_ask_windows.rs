//! `sonata ask` between Command Prompt terminals over the named pipe (feature 012, US2).
#![cfg(windows)]
mod common;

use common::{Sandbox, messaging_app, shells, sonata_as};

#[test]
fn ask_between_linked_shells() {
    let sb = Sandbox::new();
    let (app, _) = messaging_app(&sb);
    let (ws, ids) = shells(&app, &sb, &["A", "B"]);
    let (a, b) = (ids[0], ids[1]);

    let unlinked = sonata_as(&app, a, &["ask", "B", "echo hello"]);
    assert_eq!(unlinked.code, 3, "{}", unlinked.stderr);

    app.create_link(ws, a, b).unwrap();
    let out = sonata_as(&app, a, &["ask", "b", "echo hello-42"]);
    assert_eq!(out.code, 0, "{}", out.stderr);
    assert!(out.stdout.contains("hello-42"), "{}", out.stdout);

    let list = sonata_as(&app, a, &["list"]);
    assert_eq!(list.code, 0, "{}", list.stderr);
    assert!(list.stdout.contains('B'), "{}", list.stdout);
    app.quit();
}

#[test]
fn a_bad_token_is_refused() {
    let sb = Sandbox::new();
    let (app, _) = messaging_app(&sb);
    let (_, ids) = shells(&app, &sb, &["A"]);
    let out = std::process::Command::new(common::cli_bin())
        .arg("list")
        .env("SONATA_SOCKET", app.socket_path().expect("pipe"))
        .env("SONATA_TERMINAL_ID", ids[0].to_string())
        .env("SONATA_TOKEN", "not-the-token")
        .output()
        .unwrap();
    assert_ne!(out.status.code(), Some(0));
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("identity"),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    app.quit();
}

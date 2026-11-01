// Unix shells, modes, or sockets; Windows counterparts live in *_windows.rs (feature 012).
#![cfg(unix)]
mod common;

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;

use common::{Sandbox, messaging_app, mode, shells, sonata_as};

#[test]
fn socket_is_private_and_checks_identity() {
    let sb = Sandbox::new();
    let (app, _) = messaging_app(&sb);
    let sock = app.socket_path().unwrap().to_path_buf();
    assert_eq!(mode(&sock), 0o600);
    assert_eq!(mode(sock.parent().unwrap()), 0o700);
    let (_, ids) = shells(&app, &sb, &["A"]);

    let ok = sonata_as(&app, ids[0], &["list"]);
    assert_eq!(ok.code, 0, "{}", ok.stderr);
    assert!(ok.stdout.contains("You are \"A\""), "{}", ok.stdout);

    // Forged token.
    let mut s = UnixStream::connect(&sock).unwrap();
    writeln!(
        s,
        r#"{{"v":1,"terminalId":"{}","token":"bad","args":["list"]}}"#,
        ids[0]
    )
    .unwrap();
    let mut line = String::new();
    BufReader::new(s).read_line(&mut line).unwrap();
    assert!(line.contains("\"exitCode\":3"), "{line}");

    // Malformed and oversized requests.
    let mut s = UnixStream::connect(&sock).unwrap();
    writeln!(s, "not json").unwrap();
    let mut line = String::new();
    BufReader::new(s).read_line(&mut line).unwrap();
    assert!(line.contains("\"exitCode\":2"), "{line}");
    let mut s = UnixStream::connect(&sock).unwrap();
    let _ = s.write_all(&vec![b'x'; 200 * 1024]);
    let mut line = String::new();
    BufReader::new(s).read_line(&mut line).unwrap();
    assert!(line.contains("\"exitCode\":2"), "{line}");

    app.quit();
    assert!(!sock.exists(), "socket left behind");
}

#[test]
fn long_data_root_falls_back_to_temp() {
    let tmp = tempfile::tempdir().unwrap();
    let deep = tmp
        .path()
        .join("x".repeat(60))
        .join("y".repeat(60))
        .join("sonata");
    let path = sonata_core::messaging::server::socket_path(&deep).unwrap();
    assert!(path.as_os_str().len() <= 100, "{}", path.display());
    assert_eq!(mode(path.parent().unwrap()), 0o700);
}

#[test]
fn cli_outside_sonata_explains_itself() {
    let out = std::process::Command::new(common::cli_bin())
        .arg("list")
        .env_remove("SONATA_SOCKET")
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(6));
    assert!(String::from_utf8_lossy(&out.stderr).contains("only works inside a Sonata terminal"));
}

#[test]
fn terminals_get_session_environment() {
    let sb = Sandbox::new();
    let (app, _) = messaging_app(&sb);
    let (_, ids) = shells(&app, &sb, &["A"]);
    let token = app.terminals().session_token(ids[0]).unwrap();
    assert_eq!(token.len(), 64);
    app.terminals()
        .write(ids[0], b"echo \"[$SONATA_TOKEN]\" ; command -v sonata\r")
        .unwrap();
    assert!(common::wait_until(common::WAIT, || app
        .terminals()
        .screen_text(ids[0])
        .unwrap()
        .contains(&format!("[{token}]"))));
    assert!(common::wait_until(common::WAIT, || app
        .terminals()
        .screen_text(ids[0])
        .unwrap()
        .contains("/sonata")));
    app.restart_terminal(ids[0], true).unwrap();
    assert_ne!(
        app.terminals().session_token(ids[0]).unwrap(),
        token,
        "token must change on restart"
    );
    app.quit();
}

//! The Windows named pipe of the CLI channel (feature 012, contracts/ipc-pipe.md).
#![cfg(windows)]

use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;

use sonata_core::messaging::pipe::{PipeListener, connect, pipe_name, server_pid};
use sonata_core::platform::{handle_allowed_sids, user_sid};

fn unique_name() -> PathBuf {
    PathBuf::from(format!(
        r"\\.\pipe\sonata-test-{}",
        uuid::Uuid::new_v4().simple()
    ))
}

#[test]
fn a_line_goes_both_ways() {
    let name = unique_name();
    let mut listener = PipeListener::bind(&name).unwrap();
    let server = std::thread::spawn(move || {
        let conn = listener.accept().unwrap();
        let mut line = String::new();
        BufReader::new(&conn).read_line(&mut line).unwrap();
        (&conn)
            .write_all(format!("echo:{line}").as_bytes())
            .unwrap();
        sonata_core::messaging::pipe::flush(&conn);
    });
    let mut client = connect(&name).unwrap();
    assert_eq!(server_pid(&client).unwrap(), std::process::id());
    client.write_all(b"hello\n").unwrap();
    let mut reply = String::new();
    BufReader::new(&client).read_line(&mut reply).unwrap();
    assert_eq!(reply, "echo:hello\n");
    server.join().unwrap();
}

#[test]
fn the_name_is_claimed_by_the_first_server() {
    let name = unique_name();
    let _first = PipeListener::bind(&name).unwrap();
    assert!(PipeListener::bind(&name).is_err());
}

#[test]
fn a_missing_pipe_is_not_found() {
    let err = connect(&unique_name()).unwrap_err();
    assert_eq!(err.raw_os_error(), Some(2), "{err}");
}

#[test]
fn only_the_user_and_system_may_open_it() {
    let name = unique_name();
    let _listener = PipeListener::bind(&name).unwrap();
    let client = connect(&name).unwrap();
    let mut sids = handle_allowed_sids(&client).unwrap().expect("a list");
    sids.sort();
    let mut want = vec![user_sid().unwrap().to_string(), "S-1-5-18".to_string()];
    want.sort();
    assert_eq!(sids, want);
}

#[test]
fn clients_that_leave_early_do_not_stop_the_server() {
    let name = unique_name();
    let mut listener = PipeListener::bind(&name).unwrap();
    // A client connects and leaves before the server gets to it.
    drop(connect(&name).unwrap());
    let server = std::thread::spawn(move || {
        let mut served = 0;
        while served < 1 {
            if let Ok(conn) = listener.accept() {
                let mut line = String::new();
                if BufReader::new(&conn).read_line(&mut line).is_ok() && line == "ping\n" {
                    served += 1;
                }
            }
        }
    });
    std::thread::sleep(std::time::Duration::from_millis(100));
    let mut client = connect(&name).unwrap();
    client.write_all(b"ping\n").unwrap();
    server.join().unwrap();
}

#[test]
fn names_differ_per_data_root() {
    let a = pipe_name(std::path::Path::new(r"C:\a")).unwrap();
    let b = pipe_name(std::path::Path::new(r"C:\b")).unwrap();
    assert_ne!(a, b);
    assert!(a.to_string_lossy().starts_with(r"\\.\pipe\sonata-"));
}

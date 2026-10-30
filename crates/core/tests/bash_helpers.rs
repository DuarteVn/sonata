//! Typing `claude` by hand in a bash terminal starts it through `sonata run`, and the user's own
//! login startup files still load and stay unchanged (feature 011, research R8).
// Unix shells, modes, or sockets; Windows counterparts live in *_windows.rs (feature 012).
#![cfg(unix)]

mod common;

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use common::{Sandbox, cli_bin, wait_until};
use sonata_core::app::{AppOptions, NewTerminal};
use sonata_core::events::RecordingSink;
use sonata_core::workspace::manager::CreateWorkspace;
use sonata_core::{App, terminal::pty::shell_start};

#[test]
fn bash_starts_with_sonata_rcfile_and_zsh_with_zdotdir() {
    let helpers = Path::new("/data/run");
    let bash = shell_start(Path::new("/bin/bash"), Some(helpers), "/home/me".into());
    assert_eq!(bash.args, ["--rcfile", "/data/run/bash/bashrc", "-i"]);
    assert!(bash.env.is_empty());

    let zsh = shell_start(Path::new("/usr/bin/zsh"), Some(helpers), "/home/me".into());
    assert_eq!(zsh.args, ["-l"]);
    assert_eq!(
        zsh.env,
        [
            ("SONATA_USER_ZDOTDIR".to_string(), "/home/me".into()),
            ("ZDOTDIR".to_string(), "/data/run/zsh".into()),
        ]
    );

    let fish = shell_start(Path::new("/usr/bin/fish"), Some(helpers), "/home/me".into());
    assert_eq!(fish.args, ["-l"]);
    assert!(fish.env.is_empty());

    let plain = shell_start(Path::new("/bin/bash"), None, "/home/me".into());
    assert_eq!(plain.args, ["-l"]);
}

#[test]
fn hand_typed_agents_are_wrapped_and_user_profile_still_loads() {
    if !Path::new("/bin/bash").exists() {
        return;
    }
    let sb = Sandbox::new();
    let home = sb.tmp.path().join("home");
    std::fs::create_dir_all(&home).unwrap();
    let profile = "[ -r ~/.bashrc ] && . ~/.bashrc\nexport FROM_USER_PROFILE=yes\n";
    let bashrc = "export FROM_USER_RC=yes\nPS1='$ '\n";
    std::fs::write(home.join(".profile"), profile).unwrap();
    std::fs::write(home.join(".bashrc"), bashrc).unwrap();
    // SAFETY: single test touching the environment in this binary.
    unsafe {
        std::env::set_var("HOME", &home);
    }
    let opts = AppOptions {
        shell: Some("/bin/bash".into()),
        cli_dir: Some(cli_bin().parent().unwrap().to_path_buf()),
        socket: true,
        ..AppOptions::default()
    };
    let app = App::start_with(sb.root.clone(), Arc::new(RecordingSink::default()), opts).unwrap();
    let ws = app
        .create_workspace(CreateWorkspace {
            name: "B".into(),
            working_directory: sb.project("p"),
        })
        .unwrap()
        .id;
    app.open_workspace(ws).unwrap();
    let t = app.create_terminal(ws, NewTerminal::default()).unwrap().id;
    assert!(wait_until(Duration::from_secs(10), || app
        .terminal_ready(t, Duration::from_millis(500))));
    app.terminals()
        .write(
            t,
            b"echo \"[$FROM_USER_PROFILE $FROM_USER_RC]\"; type -t claude codex\r",
        )
        .unwrap();
    assert!(
        wait_until(Duration::from_secs(5), || app
            .terminals()
            .screen_text(t)
            .is_some_and(
                |s| s.contains("[yes yes]") && s.matches("function").count() >= 2
            )),
        "{:?}",
        app.terminals().screen_text(t)
    );
    assert_eq!(
        std::fs::read_to_string(home.join(".profile")).unwrap(),
        profile
    );
    assert_eq!(
        std::fs::read_to_string(home.join(".bashrc")).unwrap(),
        bashrc
    );
    app.quit();
}

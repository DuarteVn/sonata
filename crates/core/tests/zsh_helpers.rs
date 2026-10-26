//! Typing `claude` by hand in a zsh terminal starts it through `sonata run`, and the user's own
//! zsh startup files still load. Skipped where zsh is missing.

mod common;

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use common::{Sandbox, cli_bin, wait_until};
use sonata_core::App;
use sonata_core::app::{AppOptions, NewTerminal};
use sonata_core::events::RecordingSink;
use sonata_core::workspace::manager::CreateWorkspace;

#[test]
fn hand_typed_agents_are_wrapped_and_user_rc_still_loads() {
    if !Path::new("/bin/zsh").exists() {
        return;
    }
    let sb = Sandbox::new();
    let home = sb.tmp.path().join("home");
    std::fs::create_dir_all(&home).unwrap();
    std::fs::write(
        home.join(".zshrc"),
        "export FROM_USER_RC=yes\nPROMPT='%% '\n",
    )
    .unwrap();
    std::fs::write(home.join(".zshenv"), "export FROM_USER_ENV=yes\n").unwrap();
    // SAFETY: single test in this binary; nothing else reads the environment concurrently.
    unsafe {
        std::env::set_var("HOME", &home);
        std::env::remove_var("ZDOTDIR");
    }
    let opts = AppOptions {
        shell: Some("/bin/zsh".into()),
        cli_dir: Some(cli_bin().parent().unwrap().to_path_buf()),
        socket: true,
        ..AppOptions::default()
    };
    let app = App::start_with(sb.root.clone(), Arc::new(RecordingSink::default()), opts).unwrap();
    let ws = app
        .create_workspace(CreateWorkspace {
            name: "Z".into(),
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
            b"echo \"[$FROM_USER_ENV $FROM_USER_RC $ZDOTDIR]\"; whence -w claude codex kimi\r",
        )
        .unwrap();
    let expected = format!("[yes yes {}]", home.display());
    assert!(
        wait_until(Duration::from_secs(5), || app
            .terminals()
            .screen_text(t)
            .is_some_and(|s| s.contains(&expected)
                && s.contains("claude: function")
                && s.contains("codex: function")
                && s.contains("kimi: function"))),
        "{:?}",
        app.terminals().screen_text(t)
    );
    // The user's files were not touched.
    assert_eq!(
        std::fs::read_to_string(home.join(".zshrc")).unwrap(),
        "export FROM_USER_RC=yes\nPROMPT='%% '\n"
    );
    app.quit();
}

mod common;

use common::{Sandbox, WAIT, test_app, wait_until};
use sonata_core::app::NewTerminal;
use sonata_core::terminal::history;
use sonata_core::workspace::manager::CreateWorkspace;

#[test]
fn save_cap_restore_and_relaunch() {
    let sb = Sandbox::new();
    let dir = sb.project("p");
    let (ws, node, path) = {
        let (app, _) = test_app(&sb);
        let ws = app
            .create_workspace(CreateWorkspace {
                name: "W".into(),
                working_directory: dir,
            })
            .unwrap()
            .id;
        app.open_workspace(ws).unwrap();
        let node = app
            .create_terminal(
                ws,
                NewTerminal {
                    launch_command: Some("echo launched-$((40+2))".into()),
                    ..Default::default()
                },
            )
            .unwrap();
        assert!(wait_until(WAIT, || app
            .terminals()
            .screen_text(node.id)
            .unwrap()
            .contains("launched-42")));
        app.terminals()
            .write(node.id, b"echo before-restart\r")
            .unwrap();
        assert!(wait_until(WAIT, || app
            .terminals()
            .screen_text(node.id)
            .unwrap()
            .contains("\nbefore-restart")));
        app.save_dirty();
        let path = sb
            .root
            .terminals_dir(ws)
            .join(format!("{}.history", node.id));
        assert!(path.is_file(), "history not written on autosave");
        app.quit();
        (ws, node.id, path)
    };
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(text.contains("before-restart"));

    // Relaunch: history above a separator, then a fresh shell that re-runs the launch command.
    let (app, _) = test_app(&sb);
    app.open_workspace(ws).unwrap();
    assert!(wait_until(WAIT, || {
        let screen = app
            .terminals()
            .history_rows(node, 10_000)
            .unwrap()
            .join("\n");
        screen.contains("before-restart")
            && screen.contains("restored history")
            && screen.matches("launched-42").count() >= 2
    }));
    let all = app
        .terminals()
        .history_rows(node, 10_000)
        .unwrap()
        .join("\n");
    let sep = all.find("restored history").unwrap();
    assert!(all[..sep].contains("before-restart"));
    assert!(
        all[sep..].contains("launched-42"),
        "launch command did not re-run"
    );
    app.quit();
}

#[test]
fn history_file_is_capped_by_rows_and_bytes() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("t.history");
    let rows: Vec<String> = (0..20).map(|i| format!("row {i}")).collect();
    history::save(&path, &rows, 5, 1024).unwrap();
    let text = std::fs::read_to_string(&path).unwrap();
    assert_eq!(text.lines().count(), 5);
    assert!(text.starts_with("row 15"));

    let big: Vec<String> = (0..100)
        .map(|i| format!("{i:04}{}", "x".repeat(96)))
        .collect();
    history::save(&path, &big, 10_000, 1000).unwrap();
    let len = std::fs::metadata(&path).unwrap().len();
    assert!(len <= 1000, "{len}");
    assert!(
        std::fs::read_to_string(&path).unwrap().contains("0099"),
        "newest rows kept"
    );

    std::fs::write(&path, "y".repeat(5000)).unwrap();
    let restored = history::load(&path, 1000).unwrap();
    assert!(restored.len() < 1200);
}

#[test]
fn disabling_history_deletes_file_and_clear_removes_it() {
    let sb = Sandbox::new();
    let (app, _) = test_app(&sb);
    let ws = app
        .create_workspace(CreateWorkspace {
            name: "W".into(),
            working_directory: sb.project("p"),
        })
        .unwrap()
        .id;
    app.open_workspace(ws).unwrap();
    let node = app.create_terminal(ws, NewTerminal::default()).unwrap().id;
    app.terminals().write(node, b"echo secret-token\r").unwrap();
    assert!(wait_until(WAIT, || app
        .terminals()
        .screen_text(node)
        .unwrap()
        .contains("\nsecret-token")));
    let path = sb.root.terminals_dir(ws).join(format!("{node}.history"));

    app.save_dirty();
    assert!(path.is_file());
    app.clear_history(node).unwrap();
    assert!(!path.exists());

    app.set_save_history(node, false).unwrap();
    app.terminals().write(node, b"echo more\r").unwrap();
    assert!(wait_until(WAIT, || app
        .terminals()
        .screen_text(node)
        .unwrap()
        .contains("\nmore")));
    app.save_dirty();
    assert!(!path.exists(), "history written although disabled");
    app.quit();
}

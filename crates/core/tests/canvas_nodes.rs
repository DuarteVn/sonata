mod common;

use common::{Sandbox, WAIT, test_app, wait_until};
use sonata_core::ErrorCode;
use sonata_core::app::NewTerminal;
use sonata_core::model::node::Frame;
use sonata_core::workspace::manager::CreateWorkspace;

fn frame(x: f64, y: f64, w: f64, h: f64) -> Frame {
    Frame {
        x,
        y,
        width: w,
        height: h,
    }
}

fn setup(sb: &Sandbox) -> (std::sync::Arc<sonata_core::App>, uuid::Uuid) {
    let (app, _) = test_app(sb);
    let ws = app
        .create_workspace(CreateWorkspace {
            name: "W".into(),
            working_directory: sb.project("p"),
        })
        .unwrap();
    app.open_workspace(ws.id).unwrap();
    (app, ws.id)
}

#[test]
fn terminal_defaults_and_minimum_size() {
    let sb = Sandbox::new();
    let (app, ws) = setup(&sb);
    let a = app
        .create_terminal(
            ws,
            NewTerminal {
                frame: frame(0.0, 0.0, 10.0, 10.0),
                ..Default::default()
            },
        )
        .unwrap();
    let b = app
        .create_terminal(
            ws,
            NewTerminal {
                frame: frame(0.0, 0.0, 500.0, 300.0),
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(a.terminal().unwrap().name, "Terminal 1");
    assert_eq!(b.terminal().unwrap().name, "Terminal 2");
    assert_eq!((a.frame.width, a.frame.height), (240.0, 120.0));
    assert!(a.terminal().unwrap().save_history);
    app.shutdown_terminals();
}

#[test]
fn update_frame_clamps_and_bring_to_front_uses_max_z_plus_one() {
    let sb = Sandbox::new();
    let (app, ws) = setup(&sb);
    let a = app
        .create_terminal(
            ws,
            NewTerminal {
                frame: frame(0.0, 0.0, 400.0, 300.0),
                ..Default::default()
            },
        )
        .unwrap();
    let b = app
        .create_terminal(
            ws,
            NewTerminal {
                frame: frame(50.0, 50.0, 400.0, 300.0),
                ..Default::default()
            },
        )
        .unwrap();
    let f = app
        .update_frame(ws, a.id, frame(5.0, 6.0, 1.0, 1.0))
        .unwrap();
    assert_eq!(f, frame(5.0, 6.0, 240.0, 120.0));
    let z = app.bring_to_front(ws, a.id).unwrap();
    assert_eq!(z, b.z.max(a.z) + 1);
    app.shutdown_terminals();
}

#[test]
fn deleting_busy_terminal_needs_confirmation() {
    let sb = Sandbox::new();
    let (app, ws) = setup(&sb);
    let t = app
        .create_terminal(
            ws,
            NewTerminal {
                frame: frame(0.0, 0.0, 400.0, 300.0),
                ..Default::default()
            },
        )
        .unwrap();
    assert!(wait_until(WAIT, || app
        .terminals()
        .screen_text(t.id)
        .is_some_and(|s| s.contains(common::PROMPT))));
    app.terminals()
        .write(t.id, common::sleep_command(10_000).as_bytes())
        .unwrap();
    assert!(wait_until(WAIT, || app.terminals().is_busy(t.id)));
    let err = app.delete_node(ws, t.id, false).unwrap_err();
    assert_eq!(err.code, ErrorCode::TerminalBusy);
    app.delete_node(ws, t.id, true).unwrap();
    assert!(app.workspaces().workspace(ws).unwrap().node(t.id).is_none());
    assert!(app.terminals().status(t.id).is_none());
}

#[test]
fn camera_is_clamped_and_changes_mark_workspace_dirty() {
    let sb = Sandbox::new();
    let (app, ws) = setup(&sb);
    app.save_dirty();
    assert!(app.workspaces().dirty_ids().is_empty());
    let cam = app.set_camera(ws, 1.0, 2.0, 99.0).unwrap();
    assert_eq!(cam.zoom, 4.0);
    assert_eq!(app.workspaces().dirty_ids(), vec![ws]);
    app.save_dirty();
    let t = app
        .create_terminal(
            ws,
            NewTerminal {
                frame: frame(0.0, 0.0, 400.0, 300.0),
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(app.workspaces().dirty_ids(), vec![ws]);
    app.save_dirty();
    app.rename_terminal(ws, t.id, "  Agent  ").unwrap();
    assert_eq!(
        app.workspaces()
            .workspace(ws)
            .unwrap()
            .node(t.id)
            .unwrap()
            .terminal()
            .unwrap()
            .name,
        "Agent"
    );
    assert_eq!(app.workspaces().dirty_ids(), vec![ws]);
    app.shutdown_terminals();
}

#[test]
fn restart_of_busy_terminal_needs_confirmation() {
    let sb = Sandbox::new();
    let (app, ws) = setup(&sb);
    let t = app.create_terminal(ws, NewTerminal::default()).unwrap();
    assert!(wait_until(WAIT, || app
        .terminals()
        .screen_text(t.id)
        .is_some_and(|s| s.contains(common::PROMPT))));
    app.terminals()
        .write(t.id, common::sleep_command(10_000).as_bytes())
        .unwrap();
    assert!(wait_until(WAIT, || app.terminals().is_busy(t.id)));
    assert_eq!(
        app.restart_terminal(t.id, false).unwrap_err().code,
        ErrorCode::TerminalBusy
    );
    app.restart_terminal(t.id, true).unwrap();
    assert!(wait_until(WAIT, || !app.terminals().is_busy(t.id)));
    app.shutdown_terminals();
}

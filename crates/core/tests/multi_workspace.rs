mod common;

use common::{Sandbox, WAIT, test_app, wait_until};
use sonata_core::app::NewTerminal;
use sonata_core::workspace::manager::CreateWorkspace;

#[test]
fn terminals_keep_running_in_other_workspaces_and_quit_check_counts_busy() {
    let sb = Sandbox::new();
    let (app, _) = test_app(&sb);
    let dir = sb.project("p");
    let a = app
        .create_workspace(CreateWorkspace {
            name: "A".into(),
            working_directory: dir.clone(),
        })
        .unwrap()
        .id;
    let b = app
        .create_workspace(CreateWorkspace {
            name: "B".into(),
            working_directory: dir,
        })
        .unwrap()
        .id;

    app.open_workspace(a).unwrap();
    let t = app.create_terminal(a, NewTerminal::default()).unwrap();
    assert!(wait_until(WAIT, || app
        .terminals()
        .screen_text(t.id)
        .is_some_and(|s| s.contains('$'))));
    assert!(
        app.quit_check().is_empty(),
        "idle shells must not block quit"
    );

    app.terminals()
        .write(
            t.id,
            b"for i in 1 2 3 4 5 6; do echo tick-$i; sleep 0.3; done; sleep 30\r",
        )
        .unwrap();
    app.open_workspace(b).unwrap();
    assert_eq!(app.workspaces().app_state().active_workspace_id, Some(b));
    assert!(wait_until(WAIT, || app
        .terminals()
        .screen_text(t.id)
        .unwrap()
        .contains("tick-6")));

    let busy = app.quit_check();
    assert_eq!(busy.len(), 1);
    assert_eq!(busy[0].workspace_id, a);
    assert_eq!(busy[0].workspace_name, "A");
    assert_eq!(busy[0].count, 1);
    assert_eq!(
        app.summaries()
            .iter()
            .find(|s| s.id == a)
            .unwrap()
            .busy_terminal_count,
        1
    );

    app.quit();
    assert!(app.terminals().status(t.id).is_none());
    assert!(app.workspaces().app_state().clean_shutdown);
}

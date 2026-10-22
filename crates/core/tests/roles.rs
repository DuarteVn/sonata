mod common;

use common::{Sandbox, mode, test_app};
use sonata_core::ErrorCode;
use sonata_core::app::{NewTerminal, TerminalSettings};
use sonata_core::model::node::RoleRef;
use sonata_core::workspace::manager::CreateWorkspace;

#[test]
fn library_save_list_delete_persist() {
    let sb = Sandbox::new();
    let role_id = {
        let (app, _) = test_app(&sb);
        let r = app
            .role_save("Coordinator", "You coordinate the workers.")
            .unwrap();
        assert_eq!(
            app.role_save(" coordinator ", "x").unwrap_err().code,
            ErrorCode::RoleNameTaken
        );
        assert_eq!(
            app.role_save("  ", "x").unwrap_err().code,
            ErrorCode::NameEmpty
        );
        app.role_save("Reviewer", "Review diffs.").unwrap();
        assert_eq!(app.roles_list().len(), 2);
        app.quit();
        r.id
    };
    assert_eq!(mode(&sb.root.path().join("roles.json")), 0o600);

    let (app, _) = test_app(&sb);
    let names: Vec<String> = app.roles_list().into_iter().map(|r| r.name).collect();
    assert_eq!(names, ["Coordinator", "Reviewer"]);

    // A terminal keeps its copy after the library entry is deleted.
    let ws = app
        .create_workspace(CreateWorkspace {
            name: "W".into(),
            working_directory: sb.project("p"),
        })
        .unwrap()
        .id;
    app.open_workspace(ws).unwrap();
    let settings = TerminalSettings {
        role: Some(Some(RoleRef {
            name: Some("Coordinator".into()),
            text: "You coordinate the workers.".into(),
        })),
        ..Default::default()
    };
    let t = app
        .create_terminal(
            ws,
            NewTerminal {
                settings,
                ..Default::default()
            },
        )
        .unwrap();
    app.role_delete(role_id).unwrap();
    assert_eq!(app.roles_list().len(), 1);
    let node = app
        .workspaces()
        .workspace(ws)
        .unwrap()
        .node(t.id)
        .unwrap()
        .clone();
    assert_eq!(
        node.terminal().unwrap().role.as_ref().unwrap().text,
        "You coordinate the workers."
    );
    app.quit();
}

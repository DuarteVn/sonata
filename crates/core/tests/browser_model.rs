mod common;

use common::{Sandbox, test_app};
use sonata_core::browser::normalize_url;
use sonata_core::model::node::Frame;
use sonata_core::persistence::store::Store;
use sonata_core::workspace::manager::{CreateWorkspace, WorkspaceManager};

#[test]
fn url_normalization() {
    let ok = |i: &str, o: &str| assert_eq!(normalize_url(i).unwrap(), o, "{i}");
    ok("localhost:3000", "http://localhost:3000");
    ok("localhost", "http://localhost");
    ok("127.0.0.1:8080/x?y=1", "http://127.0.0.1:8080/x?y=1");
    ok("[::1]:5173", "http://[::1]:5173");
    ok("example.com", "https://example.com");
    ok("example.com:8443/a", "https://example.com:8443/a");
    ok("  https://a.b/c ", "https://a.b/c");
    ok("http://x.test", "http://x.test");
    ok("about:blank", "about:blank");
    for bad in [
        "",
        "two words",
        "file:///etc/passwd",
        "javascript:alert(1)",
        "mailto:a@b.c",
        "ftp://x",
    ] {
        assert!(normalize_url(bad).is_err(), "{bad} accepted");
    }
}

#[test]
fn browser_nodes_persist_and_link() {
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
    let b = app
        .create_browser(
            ws,
            Frame {
                x: 0.0,
                y: 0.0,
                width: 10.0,
                height: 10.0,
            },
            Some("localhost:3000"),
        )
        .unwrap();
    assert_eq!((b.frame.width, b.frame.height), (320.0, 240.0));
    assert_eq!(b.browser().unwrap().url, "http://localhost:3000");
    let blank = app
        .create_browser(
            ws,
            Frame {
                x: 0.0,
                y: 400.0,
                width: 1024.0,
                height: 720.0,
            },
            None,
        )
        .unwrap();
    assert_eq!(blank.browser().unwrap().url, "about:blank");
    assert!(
        app.create_browser(
            ws,
            Frame {
                x: 0.0,
                y: 0.0,
                width: 500.0,
                height: 500.0
            },
            Some("file:///x")
        )
        .is_err()
    );

    app.update_browser_page(b.id, "http://localhost:3000/login", Some("Login"))
        .unwrap();
    app.create_link(ws, b.id, blank.id).unwrap();
    app.save_dirty();
    let m = WorkspaceManager::load(Store::new(sb.root.clone())).unwrap();
    let saved = m
        .workspace(ws)
        .unwrap()
        .node(b.id)
        .unwrap()
        .browser()
        .unwrap()
        .clone();
    assert_eq!(saved.url, "http://localhost:3000/login");
    assert_eq!(saved.title.as_deref(), Some("Login"));
    app.delete_node(ws, b.id, false).unwrap();
    assert!(app.workspaces().workspace(ws).unwrap().links.is_empty());
    app.quit();
}

#[test]
fn windows_paths_are_not_addresses() {
    // Feature 012: a drive path or a file URL never opens in a browser node.
    for input in [
        r"C:\Users\a\page.html",
        "C:/Users/a/page.html",
        "file:///C:/x.html",
    ] {
        assert!(
            sonata_core::browser::normalize_url(input).is_err(),
            "{input} was accepted"
        );
    }
}

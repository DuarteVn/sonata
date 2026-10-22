mod common;

use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use common::{Sandbox, messaging_app, shells, sonata_as};
use sonata_core::app::TerminalSettings;
use sonata_core::browser::{BrowserHost, PageInfo};
use sonata_core::model::node::Frame;
use uuid::Uuid;

/// Records every call and answers like a tiny page.
#[derive(Default)]
struct FakeHost {
    calls: Mutex<Vec<String>>,
}

impl FakeHost {
    fn log(&self, s: String) {
        self.calls.lock().unwrap().push(s);
    }
    fn page(&self) -> Result<PageInfo, String> {
        Ok(PageInfo {
            url: "http://localhost:3000/".into(),
            title: "Test App".into(),
        })
    }
}

impl BrowserHost for FakeHost {
    fn open(&self, _: Uuid, url: &str, t: Duration) -> Result<PageInfo, String> {
        self.log(format!("open {url} {}", t.as_secs()));
        if url.contains("down") {
            Err("Could not connect to the server.".into())
        } else {
            self.page()
        }
    }
    fn back(&self, _: Uuid, _: Duration) -> Result<PageInfo, String> {
        self.log("back".into());
        self.page()
    }
    fn forward(&self, _: Uuid, _: Duration) -> Result<PageInfo, String> {
        self.log("forward".into());
        self.page()
    }
    fn reload(&self, _: Uuid, _: Duration) -> Result<PageInfo, String> {
        self.log("reload".into());
        self.page()
    }
    fn read(&self, _: Uuid) -> Result<String, String> {
        self.log("read".into());
        Ok("Title: Test App\n  [1] button \"Go\"".into())
    }
    fn click(&self, _: Uuid, target: &str, t: Duration) -> Result<PageInfo, String> {
        self.log(format!("click {target} {}", t.as_secs()));
        self.page()
    }
    fn fill(&self, _: Uuid, target: &str, text: &str) -> Result<(), String> {
        self.log(format!("fill {target} {text}"));
        Ok(())
    }
    fn press(&self, _: Uuid, key: &str) -> Result<(), String> {
        self.log(format!("press {key}"));
        Ok(())
    }
    fn screenshot(&self, _: Uuid, out: &Path) -> Result<(), String> {
        self.log("screenshot".into());
        std::fs::write(out, b"\x89PNG").map_err(|e| e.to_string())
    }
    fn eval(&self, _: Uuid, js: &str) -> Result<String, String> {
        self.log(format!("eval {js}"));
        Ok("\"Test App\"".into())
    }
}

#[test]
fn linked_terminals_drive_browsers() {
    let sb = Sandbox::new();
    let (app, _) = messaging_app(&sb);
    let host = Arc::new(FakeHost::default());
    app.set_browser_host(host.clone());
    let (ws, ids) = shells(&app, &sb, &["A", "B"]);
    let (a, b) = (ids[0], ids[1]);
    let browser = app
        .create_browser(
            ws,
            Frame {
                x: 0.0,
                y: 900.0,
                width: 1024.0,
                height: 720.0,
            },
            Some("localhost:3000"),
        )
        .unwrap()
        .id;
    app.update_browser_page(browser, "http://localhost:3000/", Some("Test App"))
        .unwrap();

    let refused = sonata_as(&app, a, &["browser", "read", "Test App"]);
    assert_eq!(refused.code, 3, "{}", refused.stderr);
    app.create_link(ws, a, browser).unwrap();

    let list = sonata_as(&app, a, &["list"]);
    assert!(
        list.stdout.contains("browser") && list.stdout.contains("http://localhost:3000/"),
        "{}",
        list.stdout
    );

    let open = sonata_as(
        &app,
        a,
        &[
            "browser",
            "open",
            "test app",
            "localhost:3000",
            "--timeout",
            "45s",
        ],
    );
    assert_eq!(open.code, 0, "{}", open.stderr);
    assert!(open.stdout.contains("Test App"));
    let down = sonata_as(&app, a, &["browser", "open", "Test App", "down.test"]);
    assert_eq!(down.code, 1);
    assert!(down.stderr.contains("Could not connect"));
    let bad = sonata_as(
        &app,
        a,
        &["browser", "open", "Test App", "file:///etc/passwd"],
    );
    assert_eq!(bad.code, 2, "{}", bad.stderr);

    assert!(
        sonata_as(&app, a, &["browser", "read", "Test App"])
            .stdout
            .contains("[1] button")
    );
    assert_eq!(
        sonata_as(&app, a, &["browser", "click", "Test App", "1"]).code,
        0
    );
    assert_eq!(
        sonata_as(&app, a, &["browser", "fill", "Test App", "#email", "a@b.c"]).code,
        0
    );
    assert_eq!(
        sonata_as(&app, a, &["browser", "press", "Test App", "Enter"]).code,
        0
    );
    for nav in ["back", "forward", "reload"] {
        assert_eq!(
            sonata_as(&app, a, &["browser", nav, "Test App"]).code,
            0,
            "{nav}"
        );
    }
    let shot = sonata_as(&app, a, &["browser", "screenshot", "Test App"]);
    assert_eq!(shot.code, 0, "{}", shot.stderr);
    assert!(Path::new(shot.stdout.trim()).is_file());
    let out = sb.tmp.path().join("shot.png");
    let shot2 = sonata_as(
        &app,
        a,
        &[
            "browser",
            "screenshot",
            "Test App",
            "--out",
            out.to_str().unwrap(),
        ],
    );
    assert_eq!(shot2.code, 0);
    assert!(out.is_file());

    // eval is for the Orchestrator only.
    assert_eq!(
        sonata_as(&app, a, &["browser", "eval", "Test App", "document.title"]).code,
        3
    );
    app.update_terminal_settings(
        a,
        TerminalSettings {
            orchestrator: Some(true),
            ..Default::default()
        },
    )
    .unwrap();
    let ev = sonata_as(&app, a, &["browser", "eval", "Test App", "document.title"]);
    assert_eq!(ev.stdout.trim(), "\"Test App\"");

    // An unlinked terminal is refused for every command.
    for sub in [
        "open",
        "read",
        "click",
        "fill",
        "press",
        "screenshot",
        "back",
        "forward",
        "reload",
        "eval",
    ] {
        let r = sonata_as(&app, b, &["browser", sub, "Test App", "x", "y"]);
        assert_eq!(r.code, 3, "{sub}: {}", r.stderr);
    }
    let calls = host.calls.lock().unwrap().clone();
    assert!(
        calls.contains(&"open http://localhost:3000 45".to_string()),
        "{calls:?}"
    );
    assert!(calls.contains(&"click 1 10".to_string()));
    assert!(calls.contains(&"fill #email a@b.c".to_string()));
    app.quit();
}

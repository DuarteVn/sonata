mod common;

use std::os::unix::fs::PermissionsExt;

use common::{Sandbox, mode, test_app};
use sonata_core::events::CoreEvent;
use sonata_core::workspace::manager::CreateWorkspace;

fn walk(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
    for e in std::fs::read_dir(dir).unwrap().flatten() {
        let p = e.path();
        out.push(p.clone());
        if p.is_dir() {
            walk(&p, out);
        }
    }
}

#[test]
fn everything_is_private_and_startup_fixes_loose_modes() {
    let sb = Sandbox::new();
    {
        let (app, _) = test_app(&sb);
        let ws = app
            .create_workspace(CreateWorkspace {
                name: "W".into(),
                working_directory: sb.project("p"),
            })
            .unwrap()
            .id;
        app.open_workspace(ws).unwrap();
        app.quit();
    }
    let _log = sonata_core::logging::init(&sb.root).unwrap();
    tracing::info!("hello");
    drop(_log);

    let mut all = Vec::new();
    walk(sb.root.path(), &mut all);
    for p in &all {
        let expected = if p.is_dir() { 0o700 } else { 0o600 };
        assert_eq!(mode(p), expected, "{}", p.display());
    }

    let loose = sb.root.app_state_file();
    std::fs::set_permissions(&loose, std::fs::Permissions::from_mode(0o644)).unwrap();
    let (app, sink) = test_app(&sb);
    assert_eq!(mode(&loose), 0o600);
    let warned =
        app.bootstrap()
            .warnings
            .iter()
            .any(|w| w.contains("permissions"))
            || sink.take().iter().any(
                |e| matches!(e, CoreEvent::PersistenceWarning(m) if m.contains("permissions")),
            );
    assert!(warned);
}

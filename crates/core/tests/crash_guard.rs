mod common;

use common::{Sandbox, test_app};
use sonata_core::crash_guard;
use sonata_core::model::app_state::AppState;
use sonata_core::workspace::manager::CreateWorkspace;

fn launched(mut s: AppState) -> (AppState, bool) {
    let tripped = crash_guard::on_launch(&mut s);
    (s, tripped)
}

#[test]
fn transitions() {
    // Clean shutdown: counter resets.
    let (s, tripped) = launched(AppState {
        consecutive_early_crashes: 1,
        ..AppState::default()
    });
    assert_eq!(s.consecutive_early_crashes, 0);
    assert!(!tripped);
    assert!(!s.clean_shutdown && !s.outlived_launch_window);

    // Early crash (unclean, inside the window): counter increments.
    let (s, tripped) = launched(s);
    assert_eq!(s.consecutive_early_crashes, 1);
    assert!(!tripped);
    let (s, tripped) = launched(s);
    assert_eq!(s.consecutive_early_crashes, 2);
    assert!(tripped);

    // Late crash (outlived the window): counter resets.
    let mut late = s.clone();
    crash_guard::on_outlived_launch_window(&mut late);
    assert_eq!(late.consecutive_early_crashes, 0);
    let (late, tripped) = launched(late);
    assert_eq!(late.consecutive_early_crashes, 0);
    assert!(!tripped);
    assert_eq!(crash_guard::EARLY_CRASH_WINDOW.as_secs(), 30);
}

#[test]
fn tripped_guard_skips_restoring_active_workspace() {
    let sb = Sandbox::new();
    let id = {
        let (app, _) = test_app(&sb);
        let id = app
            .create_workspace(CreateWorkspace {
                name: "W".into(),
                working_directory: sb.project("p"),
            })
            .unwrap()
            .id;
        app.open_workspace(id).unwrap();
        app.quit();
        id
    };
    // Normal relaunch restores.
    {
        let (app, _) = test_app(&sb);
        let boot = app.bootstrap();
        assert_eq!(boot.active_workspace_id, Some(id));
        assert!(!boot.crash_guard_tripped);
        // Simulate two early crashes: never quit, never outlive the window.
    }
    {
        let (app, _) = test_app(&sb);
        assert!(!app.bootstrap().crash_guard_tripped);
    }
    let (app, _) = test_app(&sb);
    let boot = app.bootstrap();
    assert!(boot.crash_guard_tripped);
    assert_eq!(boot.active_workspace_id, None);
    assert!(
        boot.warnings.iter().any(|w| w.contains("crashed")),
        "{:?}",
        boot.warnings
    );
}

mod common;

use common::{Sandbox, test_app};
use sonata_core::persistence::store::Store;
use sonata_core::workspace::manager::CreateWorkspace;

#[test]
fn invalid_settings_files_are_set_aside_with_a_warning() {
    let sb = Sandbox::new();
    std::fs::write(sb.root.app_state_file(), "not json").unwrap();
    std::fs::write(sb.root.preferences_file(), "{}").unwrap();
    let (app, _) = test_app(&sb);
    let boot = app.bootstrap();
    assert_eq!(
        boot.warnings
            .iter()
            .filter(|w| w.contains("invalid"))
            .count(),
        2,
        "{:?}",
        boot.warnings
    );
    assert_eq!(boot.preferences.autosave_interval_seconds, 30);
    let aside: Vec<String> = std::fs::read_dir(sb.root.path())
        .unwrap()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.contains(".invalid-"))
        .collect();
    assert_eq!(aside.len(), 2, "{aside:?}");
}

#[test]
fn missing_settings_mean_defaults_without_warning() {
    let sb = Sandbox::new();
    let store = Store::new(sb.root.clone());
    let loaded = store.load_preferences();
    assert!(loaded.warning.is_none());
    assert_eq!(loaded.value.terminal_font_size, 13);
}

#[test]
fn active_workspace_restored_and_recent_capped() {
    let sb = Sandbox::new();
    let last = {
        let (app, _) = test_app(&sb);
        let dir = sb.project("p");
        let mut last = None;
        for i in 0..12 {
            let id = app
                .create_workspace(CreateWorkspace {
                    name: format!("W{i}"),
                    working_directory: dir.clone(),
                })
                .unwrap()
                .id;
            app.open_workspace(id).unwrap();
            last = Some(id);
        }
        app.quit();
        last.unwrap()
    };
    let (app, _) = test_app(&sb);
    let boot = app.bootstrap();
    assert_eq!(boot.active_workspace_id, Some(last));
    assert_eq!(boot.recent_workspace_ids.len(), 10);
    assert_eq!(boot.recent_workspace_ids[0], last);
}

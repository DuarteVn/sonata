mod common;

use common::{Sandbox, test_app};
use sonata_core::model::node::{DeviceContent, Frame, Platform};
use sonata_core::model::preferences::Preferences;
use sonata_core::persistence::store::Store;
use sonata_core::workspace::manager::{CreateWorkspace, WorkspaceManager};

fn frame(x: f64, y: f64, width: f64, height: f64) -> Frame {
    Frame {
        x,
        y,
        width,
        height,
    }
}

#[test]
fn device_content_round_trips_and_omits_empty_fields() {
    let blank = DeviceContent {
        platform: Platform::Android,
        device_id: String::new(),
        name: None,
        os_version: None,
    };
    assert_eq!(
        serde_json::to_value(&blank).unwrap(),
        serde_json::json!({"deviceId": "", "platform": "android"})
    );
    let full: DeviceContent = serde_json::from_str(
        r#"{"platform":"ios","deviceId":"ABC-123","name":"iPhone 16","osVersion":"iOS 18.2"}"#,
    )
    .unwrap();
    assert_eq!(full.platform, Platform::Ios);
    assert_eq!(full.name.as_deref(), Some("iPhone 16"));
    assert!(serde_json::from_str::<DeviceContent>(r#"{"platform":"web","deviceId":""}"#).is_err());
}

#[test]
fn device_nodes_have_a_phone_frame_persist_and_link() {
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
    let small = app.create_device(ws, frame(0.0, 0.0, 10.0, 10.0)).unwrap();
    assert_eq!((small.frame.width, small.frame.height), (200.0, 300.0));
    let d = app
        .create_device(ws, frame(0.0, 400.0, 380.0, 820.0))
        .unwrap();
    let content = d.device().unwrap();
    assert_eq!(content.platform, Platform::Android);
    assert!(content.device_id.is_empty());

    app.select_device(
        d.id,
        Platform::Android,
        "Pixel_8_API_34",
        Some("Pixel 8"),
        Some("Android 14"),
    )
    .unwrap();
    let twice = app
        .select_device(small.id, Platform::Android, "Pixel_8_API_34", None, None)
        .unwrap_err();
    assert_eq!(twice.code, sonata_core::ErrorCode::DeviceInUse);
    app.create_link(ws, d.id, small.id).unwrap();
    app.save_dirty();
    let m = WorkspaceManager::load(Store::new(sb.root.clone())).unwrap();
    let saved = m
        .workspace(ws)
        .unwrap()
        .node(d.id)
        .unwrap()
        .device()
        .unwrap()
        .clone();
    assert_eq!(saved.device_id, "Pixel_8_API_34");
    assert_eq!(saved.name.as_deref(), Some("Pixel 8"));
    assert_eq!(saved.os_version.as_deref(), Some("Android 14"));
    assert_eq!(app.node_name(d.id).as_deref(), Some("Pixel 8"));

    app.delete_node(ws, d.id, false).unwrap();
    assert!(app.workspaces().workspace(ws).unwrap().links.is_empty());
    app.quit();
}

#[test]
fn chosen_sdk_folder_is_saved_only_when_set() {
    let sb = Sandbox::new();
    let store = Store::new(sb.root.clone());
    store.save_preferences(&Preferences::default()).unwrap();
    let raw = std::fs::read_to_string(sb.root.preferences_file()).unwrap();
    assert!(!raw.contains("androidSdkPath"), "{raw}");
    let prefs = Preferences {
        android_sdk_path: Some("/opt/android-sdk".into()),
        ..Preferences::default()
    };
    store.save_preferences(&prefs).unwrap();
    assert_eq!(
        store.load_preferences().value.android_sdk_path.as_deref(),
        Some(std::path::Path::new("/opt/android-sdk"))
    );
}

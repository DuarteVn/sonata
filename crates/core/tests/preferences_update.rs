mod common;

use common::Sandbox;
use sonata_core::model::preferences::Preferences;
use sonata_core::persistence::store::Store;

fn write_prefs(sb: &Sandbox, payload: &str) {
    let json = format!(r#"{{"type":"preferences","schemaVersion":1,"payload":{payload}}}"#);
    std::fs::write(sb.root.preferences_file(), json).unwrap();
}

#[test]
fn old_preferences_without_the_field_check_for_updates() {
    let sb = Sandbox::new();
    write_prefs(&sb, r#"{"autosaveIntervalSeconds":30}"#);
    let loaded = Store::new(sb.root.clone()).load_preferences();
    assert!(loaded.warning.is_none(), "{:?}", loaded.warning);
    assert!(loaded.value.auto_update_check);
}

#[test]
fn turning_update_checks_off_round_trips() {
    let sb = Sandbox::new();
    let store = Store::new(sb.root.clone());
    let prefs = Preferences {
        auto_update_check: false,
        ..Preferences::default()
    };
    store.save_preferences(&prefs).unwrap();
    let raw = std::fs::read_to_string(sb.root.preferences_file()).unwrap();
    let json: serde_json::Value = serde_json::from_str(&raw).unwrap();
    assert_eq!(json["payload"]["autoUpdateCheck"], false, "{raw}");
    assert!(!store.load_preferences().value.auto_update_check);
}

#[test]
fn the_default_is_not_written() {
    let sb = Sandbox::new();
    let store = Store::new(sb.root.clone());
    store.save_preferences(&Preferences::default()).unwrap();
    let raw = std::fs::read_to_string(sb.root.preferences_file()).unwrap();
    assert!(!raw.contains("autoUpdateCheck"), "{raw}");
}

// ---- Kimi notice dismissal (feature 010) ----

#[test]
fn kimi_notice_dismissal_round_trips_and_is_omitted_when_unset() {
    use sonata_core::model::preferences::KimiNoticeDismissed;
    let sb = Sandbox::new();
    let store = Store::new(sb.root.clone());
    store.save_preferences(&Preferences::default()).unwrap();
    let raw = std::fs::read_to_string(sb.root.preferences_file()).unwrap();
    assert!(!raw.contains("kimiNoticeDismissed"), "{raw}");
    for (value, text) in [
        (KimiNoticeDismissed::Off, "off"),
        (KimiNoticeDismissed::Outdated, "outdated"),
    ] {
        let prefs = Preferences {
            kimi_notice_dismissed: Some(value),
            ..Preferences::default()
        };
        store.save_preferences(&prefs).unwrap();
        let raw = std::fs::read_to_string(sb.root.preferences_file()).unwrap();
        let json: serde_json::Value = serde_json::from_str(&raw).unwrap();
        assert_eq!(json["payload"]["kimiNoticeDismissed"], text, "{raw}");
        assert_eq!(
            store.load_preferences().value.kimi_notice_dismissed,
            Some(value)
        );
    }
}

#[test]
fn an_unknown_kimi_notice_value_is_a_load_warning() {
    let sb = Sandbox::new();
    write_prefs(&sb, r#"{"kimiNoticeDismissed":"bogus"}"#);
    let loaded = Store::new(sb.root.clone()).load_preferences();
    assert!(loaded.warning.is_some());
}

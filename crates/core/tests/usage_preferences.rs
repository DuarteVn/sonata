mod common;

use common::Sandbox;
use sonata_core::model::preferences::{Preferences, ProviderPrefs};
use sonata_core::persistence::store::Store;
use sonata_core::usage::consent::Consent;

fn write_prefs(sb: &Sandbox, payload: &str) {
    let json = format!(r#"{{"type":"preferences","schemaVersion":1,"payload":{payload}}}"#);
    std::fs::write(sb.root.preferences_file(), json).unwrap();
}

fn raw(sb: &Sandbox) -> serde_json::Value {
    serde_json::from_str(&std::fs::read_to_string(sb.root.preferences_file()).unwrap()).unwrap()
}

#[test]
fn old_preferences_load_with_usage_on() {
    let sb = Sandbox::new();
    write_prefs(&sb, r#"{"autosaveIntervalSeconds":30}"#);
    let loaded = Store::new(sb.root.clone()).load_preferences();
    assert!(loaded.warning.is_none(), "{:?}", loaded.warning);
    assert!(loaded.value.usage.enabled);
    assert!(loaded.value.usage.warnings);
    assert!(loaded.value.usage.providers.is_empty());
}

#[test]
fn defaults_are_not_written() {
    let sb = Sandbox::new();
    let store = Store::new(sb.root.clone());
    store.save_preferences(&Preferences::default()).unwrap();
    assert!(raw(&sb)["payload"].get("usage").is_none());
}

#[test]
fn switches_and_provider_entries_round_trip() {
    let sb = Sandbox::new();
    let store = Store::new(sb.root.clone());
    let mut prefs = Preferences::default();
    prefs.usage.enabled = false;
    prefs.usage.warnings = false;
    prefs.usage.providers.insert(
        "claude".into(),
        ProviderPrefs {
            enabled: Some(true),
            color: Some("#112233".into()),
            consent: Some(Consent::Granted {
                approved_command: vec!["claude".into(), "-p".into()],
            }),
        },
    );
    prefs
        .usage
        .providers
        .insert("codex".into(), ProviderPrefs::default());
    store.save_preferences(&prefs).unwrap();
    let v = raw(&sb);
    let usage = &v["payload"]["usage"];
    assert_eq!(usage["enabled"], false);
    assert_eq!(usage["warnings"], false);
    assert_eq!(usage["providers"]["claude"]["color"], "#112233");
    assert_eq!(usage["providers"]["claude"]["consent"]["state"], "granted");
    assert!(
        usage["providers"].get("codex").is_none(),
        "empty entries are omitted"
    );
    let back = store.load_preferences().value;
    assert_eq!(
        back.usage.providers["claude"],
        prefs.usage.providers["claude"]
    );
    assert!(!back.usage.enabled);
    assert_eq!(v["schemaVersion"], 1);
}

mod common;

use common::Sandbox;
use sonata_core::events::CoreEvent;
use sonata_core::model::preferences::UsagePreferences;
use sonata_core::usage::service::{ConsentRequest, UsageWarning};

#[test]
fn usage_events_have_contract_names_and_camel_case_payloads() {
    let sb = Sandbox::new();
    let (app, _) = common::test_app(&sb);
    let state = app.usage_state();
    assert!(state.enabled);
    let e = CoreEvent::UsageState(state);
    assert_eq!(e.name(), "usage://state");
    assert!(e.payload()["providersFolder"].is_string());

    let w = CoreEvent::UsageWarning(UsageWarning {
        provider_id: "claude".into(),
        provider_name: "Claude Code".into(),
        window_label: "5-hour".into(),
        level: 80,
        resets_at: None,
    });
    assert_eq!(w.name(), "usage://warning");
    assert_eq!(w.payload()["windowLabel"], "5-hour");

    let c = CoreEvent::UsageConsent(ConsentRequest {
        provider_id: "claude".into(),
        provider_name: "Claude Code".into(),
        kind: sonata_core::usage::ProviderKind::BuiltIn,
        command: vec!["claude".into()],
        explanation: String::new(),
    });
    assert_eq!(c.name(), "usage://consent");
    assert_eq!(c.payload()["kind"], "builtIn");
    let _ = UsagePreferences::default();
}

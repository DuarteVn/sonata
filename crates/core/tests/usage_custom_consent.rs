mod common;

use std::sync::Arc;

use common::Sandbox;
use common::fake_bin::fake_bin;
use sonata_core::events::{CoreEvent, RecordingSink};
use sonata_core::model::preferences::UsagePreferences;
use sonata_core::usage::ReadingStatus;
use sonata_core::usage::schedule::RefreshReason;
use sonata_core::usage::service::UsageService;
use sonata_core::usage::system::SystemBackend;

fn write_def(sb: &Sandbox, command: &[String]) {
    let dir = sb.root.usage_providers_dir();
    std::fs::create_dir_all(&dir).unwrap();
    let payload = serde_json::json!({"id": "fake", "name": "Fake", "command": command});
    let doc = serde_json::json!({"type": "usageProvider", "schemaVersion": 1, "payload": payload});
    std::fs::write(dir.join("fake.json"), doc.to_string()).unwrap();
}

fn consent_commands(sink: &RecordingSink) -> Vec<(String, Vec<String>)> {
    sink.take()
        .into_iter()
        .filter_map(|e| match e {
            CoreEvent::UsageConsent(c) => Some((c.provider_id, c.command)),
            _ => None,
        })
        .collect()
}

#[test]
fn custom_command_needs_consent_again_when_it_changes() {
    let sb = Sandbox::new();
    let bin_dir = sb.tmp.path().join("bin");
    let tool = fake_bin(
        &bin_dir,
        "fake-usage",
        r#"echo '{"plan":"Test","windows":[{"kind":"short","usedPercent":42}]}'"#,
    );
    let shell = fake_bin(&bin_dir, "loginsh", r#"shift; exec /bin/sh "$@""#);
    let command = vec![tool.display().to_string()];
    write_def(&sb, &command);
    let sink = Arc::new(RecordingSink::default());
    let svc = UsageService::new(
        sb.root.clone(),
        UsagePreferences::default(),
        Arc::new(SystemBackend {
            shell,
            extra_path: Some(sb.tmp.path().join("empty")),
            root: sb.root.clone(),
        }),
        sink.clone(),
    );
    svc.refresh(RefreshReason::Manual, None);
    let asked = consent_commands(&sink);
    assert!(
        asked.contains(&("fake".to_string(), command.clone())),
        "{asked:?}"
    );

    svc.answer_consent("fake", true, command.clone()).unwrap();
    let st = svc.refresh(RefreshReason::Manual, Some("fake"));
    let fake = st.providers.iter().find(|p| p.id == "fake").unwrap();
    assert_eq!(fake.consent, "granted");
    if !std::path::Path::new(sonata_core::usage::custom::SANDBOX_EXEC).exists() {
        // No sandbox (Linux CI): custom commands fail closed and never run.
        assert!(fake.reading.is_none());
        assert!(
            fake.last_error
                .as_deref()
                .unwrap_or_default()
                .contains("sandbox-exec"),
            "{:?}",
            fake.last_error
        );
        return;
    }
    let reading = fake.reading.as_ref().unwrap();
    assert_eq!(
        reading.status,
        ReadingStatus::Ok,
        "{:?} {:?}",
        reading.message,
        fake.last_error
    );
    assert_eq!(reading.windows[0].used_percent, 42.0);

    let changed = vec![tool.display().to_string(), "--more".into()];
    write_def(&sb, &changed);
    sink.take();
    let st = svc.refresh(RefreshReason::Manual, None);
    let fake = st.providers.iter().find(|p| p.id == "fake").unwrap();
    assert_eq!(fake.consent, "unasked");
    assert_eq!(consent_commands(&sink), [("fake".to_string(), changed)]);
}

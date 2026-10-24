mod common;

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use chrono::{DateTime, Utc};
use common::Sandbox;
use sonata_core::events::{CoreEvent, RecordingSink};
use sonata_core::model::preferences::UsagePreferences;
use sonata_core::usage::detect::DetectResult;
use sonata_core::usage::schedule::RefreshReason;
use sonata_core::usage::service::{Backend, ProviderInfo, UsageService};
use sonata_core::usage::{Detection, Reading, ReadingStatus, Window, WindowKind};

/// Finds every requested command and answers reads with a fixed percentage.
#[derive(Default)]
struct FakeBackend {
    detects: AtomicUsize,
    reads: AtomicUsize,
    percent: Mutex<f64>,
    delay: Mutex<Duration>,
    missing: Mutex<Vec<String>>,
}

impl Backend for FakeBackend {
    fn detect(&self, names: &[&str]) -> DetectResult {
        self.detects.fetch_add(1, Ordering::SeqCst);
        let missing = self.missing.lock().unwrap().clone();
        DetectResult::fake(
            names
                .iter()
                .map(|n| {
                    let found = !missing.iter().any(|m| m == n);
                    (
                        n.to_string(),
                        found.then(|| format!("/opt/fake/{n}").into()),
                    )
                })
                .collect(),
        )
    }

    fn read(&self, p: &ProviderInfo, _env: &[(String, String)], now: DateTime<Utc>) -> Reading {
        self.reads.fetch_add(1, Ordering::SeqCst);
        std::thread::sleep(*self.delay.lock().unwrap());
        let mut r = Reading::new(p.id.clone(), ReadingStatus::Ok, now);
        r.plan = Some("Team".into());
        r.account_email = Some("user@example.com".into());
        r.windows = vec![Window {
            kind: WindowKind::Short,
            label: "5-hour".into(),
            used_percent: *self.percent.lock().unwrap(),
            resets_at: Some(now + chrono::Duration::hours(2)),
        }];
        r
    }
}

fn service(sb: &Sandbox) -> (Arc<UsageService>, Arc<FakeBackend>, Arc<RecordingSink>) {
    let backend = Arc::new(FakeBackend::default());
    *backend.percent.lock().unwrap() = 25.0;
    let sink = Arc::new(RecordingSink::default());
    let svc = UsageService::new(
        sb.root.clone(),
        UsagePreferences::default(),
        backend.clone(),
        sink.clone(),
    );
    (Arc::new(svc), backend, sink)
}

fn consent_requests(sink: &RecordingSink) -> Vec<String> {
    sink.take()
        .into_iter()
        .filter_map(|e| match e {
            CoreEvent::UsageConsent(r) => Some(r.provider_id),
            _ => None,
        })
        .collect()
}

fn allow(svc: &UsageService, id: &str) {
    let command = svc
        .state()
        .providers
        .iter()
        .find(|p| p.id == id)
        .unwrap()
        .command
        .clone();
    svc.answer_consent(id, true, command).unwrap();
}

#[test]
fn lists_built_in_providers_unasked() {
    let sb = Sandbox::new();
    let (svc, _, _) = service(&sb);
    let st = svc.state();
    assert!(st.enabled && st.warnings);
    let ids: Vec<&str> = st.providers.iter().map(|p| p.id.as_str()).collect();
    assert_eq!(ids, ["claude", "codex"]);
    let claude = &st.providers[0];
    assert_eq!(claude.name, "Claude Code");
    assert_eq!(claude.color, "#d97757");
    assert_eq!(claude.consent, "unasked");
    assert_eq!(claude.command[0], "claude");
    assert!(claude.enabled);
    assert!(claude.reading.is_none());
}

#[test]
fn no_read_happens_before_consent_and_consent_is_asked_once() {
    let sb = Sandbox::new();
    let (svc, backend, sink) = service(&sb);
    svc.refresh(RefreshReason::Auto, None);
    assert_eq!(backend.detects.load(Ordering::SeqCst), 1);
    assert_eq!(backend.reads.load(Ordering::SeqCst), 0);
    assert_eq!(consent_requests(&sink), ["claude", "codex"]);
    svc.refresh(RefreshReason::Auto, None);
    assert!(consent_requests(&sink).is_empty(), "asked once per run");
    assert_eq!(
        backend.detects.load(Ordering::SeqCst),
        1,
        "detection is cached"
    );
}

#[test]
fn consent_with_a_different_command_is_rejected() {
    let sb = Sandbox::new();
    let (svc, _, _) = service(&sb);
    let err = svc
        .answer_consent("claude", true, vec!["/bin/evil".into()])
        .unwrap_err();
    assert_eq!(err.code, sonata_core::ErrorCode::ConsentMismatch);
}

#[test]
fn allowed_provider_is_read_cached_and_saved_without_email() {
    let sb = Sandbox::new();
    let (svc, backend, _) = service(&sb);
    svc.refresh(RefreshReason::Auto, None);
    allow(&svc, "claude");
    let st = svc.refresh(RefreshReason::Manual, Some("claude"));
    assert_eq!(backend.reads.load(Ordering::SeqCst), 1);
    let claude = &st.providers[0];
    assert_eq!(claude.consent, "granted");
    let reading = claude.reading.as_ref().unwrap();
    assert_eq!(reading.status, ReadingStatus::Ok);
    assert_eq!(reading.account_email.as_deref(), Some("user@example.com"));
    assert_eq!(st.summary.percent, Some(25.0));
    let text = std::fs::read_to_string(sb.root.usage_cache_file()).unwrap();
    assert!(text.contains("\"plan\": \"Team\""), "{text}");
    assert!(!text.contains('@'));
    let prefs = std::fs::read_to_string(sb.root.preferences_file()).unwrap();
    assert!(prefs.contains("\"granted\""), "{prefs}");
}

#[test]
fn cached_reading_survives_a_restart() {
    let sb = Sandbox::new();
    let (svc, _, _) = service(&sb);
    allow(&svc, "claude");
    svc.refresh(RefreshReason::Manual, Some("claude"));
    let prefs = sonata_core::persistence::store::Store::new(sb.root.clone())
        .load_preferences()
        .value
        .usage;
    let again = UsageService::new(
        sb.root.clone(),
        prefs,
        Arc::new(FakeBackend::default()),
        Arc::new(RecordingSink::default()),
    );
    let claude = &again.state().providers[0];
    assert_eq!(claude.consent, "granted");
    assert_eq!(
        claude.reading.as_ref().unwrap().plan.as_deref(),
        Some("Team")
    );
    assert_eq!(claude.reading.as_ref().unwrap().account_email, None);
}

#[test]
fn a_provider_is_never_read_twice_at_once() {
    let sb = Sandbox::new();
    let (svc, backend, _) = service(&sb);
    allow(&svc, "claude");
    *backend.delay.lock().unwrap() = Duration::from_millis(400);
    let a = {
        let svc = svc.clone();
        std::thread::spawn(move || svc.refresh(RefreshReason::Manual, Some("claude")))
    };
    std::thread::sleep(Duration::from_millis(100));
    let during = svc.refresh(RefreshReason::Manual, Some("claude"));
    assert!(during.providers[0].refreshing);
    a.join().unwrap();
    assert_eq!(backend.reads.load(Ordering::SeqCst), 1);
}

#[test]
fn colors_are_validated() {
    let sb = Sandbox::new();
    let (svc, _, _) = service(&sb);
    let err = svc.set_provider_color("claude", "red").unwrap_err();
    assert_eq!(err.code, sonata_core::ErrorCode::InvalidColor);
    let st = svc.set_provider_color("claude", "#00aaff").unwrap();
    assert_eq!(st.providers[0].color, "#00aaff");
}

#[test]
fn denying_turns_the_switch_off_and_turning_it_on_asks_again() {
    let sb = Sandbox::new();
    let (svc, backend, sink) = service(&sb);
    svc.refresh(RefreshReason::Auto, None);
    sink.take();
    let command = svc.state().providers[0].command.clone();
    let st = svc.answer_consent("claude", false, command).unwrap();
    assert_eq!(st.providers[0].consent, "denied");
    assert!(!st.providers[0].enabled);
    svc.refresh(RefreshReason::Manual, Some("claude"));
    assert_eq!(backend.reads.load(Ordering::SeqCst), 0);
    let st = svc.set_provider_enabled("claude", true).unwrap();
    assert_eq!(st.providers[0].consent, "unasked");
    svc.refresh(RefreshReason::Auto, Some("claude"));
    assert_eq!(consent_requests(&sink), ["claude"]);
}

#[test]
fn missing_command_shows_not_installed() {
    let sb = Sandbox::new();
    let (svc, backend, _) = service(&sb);
    backend.missing.lock().unwrap().push("codex".into());
    let st = svc.refresh(RefreshReason::Auto, None);
    let codex = &st.providers[1];
    assert_eq!(codex.detection, Detection::NotFound);
    assert_eq!(codex.command_path, None);
    assert_eq!(
        codex.reading.as_ref().unwrap().status,
        ReadingStatus::NotInstalled
    );
}

fn warnings(sink: &RecordingSink) -> Vec<(String, u8)> {
    sink.take()
        .into_iter()
        .filter_map(|e| match e {
            CoreEvent::UsageWarning(w) => Some((w.window_label, w.level)),
            _ => None,
        })
        .collect()
}

#[test]
fn crossing_80_sends_one_warning() {
    let sb = Sandbox::new();
    let (svc, backend, sink) = service(&sb);
    allow(&svc, "claude");
    *backend.percent.lock().unwrap() = 85.0;
    sink.take();
    svc.refresh(RefreshReason::Manual, Some("claude"));
    assert_eq!(warnings(&sink), [("5-hour".to_string(), 80)]);
    svc.refresh(RefreshReason::Manual, Some("claude"));
    assert!(warnings(&sink).is_empty());
    assert_eq!(
        svc.state().summary.state,
        sonata_core::usage::window::SummaryState::Warning
    );
}

#[test]
fn warnings_off_sends_nothing_but_the_meter_still_changes() {
    let sb = Sandbox::new();
    let (svc, backend, sink) = service(&sb);
    allow(&svc, "claude");
    svc.set_warnings(false).unwrap();
    *backend.percent.lock().unwrap() = 100.0;
    sink.take();
    let st = svc.refresh(RefreshReason::Manual, Some("claude"));
    assert!(warnings(&sink).is_empty());
    assert_eq!(
        st.summary.state,
        sonata_core::usage::window::SummaryState::Full
    );
    assert!(!st.warnings);
}

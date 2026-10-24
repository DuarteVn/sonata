mod common;

use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

use chrono::{DateTime, Utc};
use common::Sandbox;
use sonata_core::events::NullSink;
use sonata_core::model::preferences::UsagePreferences;
use sonata_core::usage::detect::DetectResult;
use sonata_core::usage::schedule::RefreshReason;
use sonata_core::usage::service::{Backend, ProviderInfo, UsageService};
use sonata_core::usage::window::SummaryState;
use sonata_core::usage::{Reading, ReadingStatus, Window, WindowKind};

#[derive(Default)]
struct Counting {
    detects: AtomicUsize,
    reads: Mutex<Vec<String>>,
    missing: Mutex<Vec<String>>,
}

impl Backend for Counting {
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
    fn read(&self, p: &ProviderInfo, _: &[(String, String)], now: DateTime<Utc>) -> Reading {
        self.reads.lock().unwrap().push(p.id.clone());
        let mut r = Reading::new(p.id.clone(), ReadingStatus::Ok, now);
        r.windows = vec![Window {
            kind: WindowKind::Short,
            label: "5-hour".into(),
            used_percent: 30.0,
            resets_at: None,
        }];
        r
    }
}

fn setup() -> (Sandbox, UsageService, Arc<Counting>) {
    let sb = Sandbox::new();
    let backend = Arc::new(Counting::default());
    let svc = UsageService::new(
        sb.root.clone(),
        UsagePreferences::default(),
        backend.clone(),
        Arc::new(NullSink),
    );
    svc.refresh(RefreshReason::Auto, None);
    for id in ["claude", "codex"] {
        let st = svc.state();
        let p = st.providers.iter().find(|p| p.id == id).unwrap();
        svc.answer_consent(id, true, p.command.clone()).unwrap();
    }
    (sb, svc, backend)
}

fn reads(b: &Counting) -> Vec<String> {
    std::mem::take(&mut b.reads.lock().unwrap())
}

#[test]
fn master_switch_off_reads_nothing() {
    let (_sb, svc, backend) = setup();
    let st = svc.set_enabled(false).unwrap();
    assert!(!st.enabled);
    assert_eq!(st.summary.state, SummaryState::Off);
    svc.refresh(RefreshReason::Manual, None);
    svc.refresh(RefreshReason::Auto, None);
    assert!(reads(&backend).is_empty());
    svc.set_enabled(true).unwrap();
    svc.refresh(RefreshReason::Manual, Some("claude"));
    assert_eq!(reads(&backend), ["claude"]);
}

#[test]
fn a_switched_off_agent_is_skipped() {
    let (_sb, svc, backend) = setup();
    let st = svc.set_provider_enabled("codex", false).unwrap();
    assert!(!st.providers[1].enabled);
    assert_eq!(st.providers[1].consent, "granted", "consent is kept");
    svc.refresh(RefreshReason::Manual, None);
    assert_eq!(reads(&backend), ["claude"]);
}

#[test]
fn a_missing_command_is_never_read() {
    let (_sb, svc, backend) = setup();
    backend.missing.lock().unwrap().push("codex".into());
    svc.refresh(RefreshReason::Manual, None);
    svc.set_provider_enabled("codex", true).unwrap();
    svc.refresh(RefreshReason::Manual, Some("codex"));
    assert_eq!(reads(&backend), ["claude"]);
}

#[test]
fn refresh_all_detects_again_and_reads_each_agent_once() {
    let (_sb, svc, backend) = setup();
    let before = backend.detects.load(Ordering::SeqCst);
    svc.refresh(RefreshReason::Manual, None);
    assert_eq!(backend.detects.load(Ordering::SeqCst), before + 1);
    let mut got = reads(&backend);
    got.sort();
    assert_eq!(got, ["claude", "codex"]);
}

#[test]
fn settings_persist_in_preferences() {
    let (sb, svc, _) = setup();
    svc.set_provider_color("claude", "#ABCDEF").unwrap();
    svc.set_warnings(false).unwrap();
    let prefs = sonata_core::persistence::store::Store::new(sb.root.clone())
        .load_preferences()
        .value
        .usage;
    assert!(!prefs.warnings);
    assert_eq!(prefs.providers["claude"].color.as_deref(), Some("#abcdef"));
}

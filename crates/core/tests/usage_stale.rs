mod common;

use std::sync::{Arc, Mutex};

use chrono::{DateTime, Utc};
use common::Sandbox;
use sonata_core::events::NullSink;
use sonata_core::model::preferences::UsagePreferences;
use sonata_core::usage::detect::DetectResult;
use sonata_core::usage::schedule::RefreshReason;
use sonata_core::usage::service::{Backend, ProviderInfo, UsageService};
use sonata_core::usage::{Reading, ReadingStatus, Window, WindowKind};

type Step = Box<dyn Fn(&str, DateTime<Utc>) -> Reading + Send>;

/// Answers with the next queued reading builder.
struct Scripted(Mutex<Vec<Step>>);

impl Backend for Scripted {
    fn detect(&self, names: &[&str]) -> DetectResult {
        DetectResult::fake(
            names
                .iter()
                .map(|n| (n.to_string(), Some(format!("/opt/fake/{n}").into())))
                .collect(),
        )
    }
    fn read(&self, p: &ProviderInfo, _: &[(String, String)], now: DateTime<Utc>) -> Reading {
        let f = self.0.lock().unwrap().remove(0);
        f(&p.id, now)
    }
}

fn ok_minutes_ago(minutes: i64) -> Step {
    Box::new(move |id, now| {
        let mut r = Reading::new(
            id,
            ReadingStatus::Ok,
            now - chrono::Duration::minutes(minutes),
        );
        r.windows = vec![Window {
            kind: WindowKind::Weekly,
            label: "Weekly".into(),
            used_percent: 10.0,
            resets_at: Some(now + chrono::Duration::days(3)),
        }];
        r
    })
}

fn granted(svc: &UsageService) {
    let cmd = svc.state().providers[0].command.clone();
    svc.answer_consent("claude", true, cmd).unwrap();
}

fn service(sb: &Sandbox, script: Vec<Step>) -> UsageService {
    UsageService::new(
        sb.root.clone(),
        UsagePreferences::default(),
        Arc::new(Scripted(Mutex::new(script))),
        Arc::new(NullSink),
    )
}

#[test]
fn readings_older_than_15_minutes_are_stale() {
    let sb = Sandbox::new();
    let svc = service(&sb, vec![ok_minutes_ago(16)]);
    granted(&svc);
    let st = svc.refresh(RefreshReason::Manual, Some("claude"));
    assert!(st.providers[0].stale);

    let sb = Sandbox::new();
    let svc = service(&sb, vec![ok_minutes_ago(14)]);
    granted(&svc);
    assert!(!svc.refresh(RefreshReason::Manual, Some("claude")).providers[0].stale);
}

#[test]
fn a_failed_refresh_keeps_the_last_good_reading() {
    let sb = Sandbox::new();
    let svc = service(
        &sb,
        vec![
            ok_minutes_ago(0),
            Box::new(|id, now| Reading::unavailable(id, now, "timed out after 20 s")),
        ],
    );
    granted(&svc);
    svc.refresh(RefreshReason::Manual, Some("claude"));
    let st = svc.refresh(RefreshReason::Manual, Some("claude"));
    let claude = &st.providers[0];
    assert_eq!(claude.reading.as_ref().unwrap().status, ReadingStatus::Ok);
    assert_eq!(claude.last_error.as_deref(), Some("timed out after 20 s"));
    assert_eq!(st.summary.percent, Some(10.0));
}

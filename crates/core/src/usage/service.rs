//! The usage service: provider registry, refreshes, state, and events. No Tauri dependency.
//!
//! Reads run outside the lock, one thread per provider, so a slow agent command never blocks
//! the state, other providers, or terminals (SC-004).

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use chrono::{DateTime, Utc};
use serde::Serialize;

use super::cache::{self, UsageCache};
use super::consent::Consent;
use super::detect::DetectResult;
use super::schedule::{RefreshReason, Schedule};
use super::window::{self, UsageSummary};
use super::{Detection, ProviderKind, Reading, ReadingStatus, current_windows};
use crate::error::{AppError, AppResult, ErrorCode};
use crate::events::{CoreEvent, EventSink};
use crate::model::preferences::UsagePreferences;
use crate::paths::DataRoot;
use crate::persistence::store::Store;

/// A reading older than this is shown as stale (FR-004).
pub const STALE_AFTER: Duration = Duration::from_secs(15 * 60);

/// Built-in providers: id, name, default color.
pub const BUILT_INS: [(&str, &str, &str); 2] = [
    ("claude", "Claude Code", "#d97757"),
    ("codex", "Codex", "#10a37f"),
];

/// Everything a backend needs to read one provider.
#[derive(Debug, Clone, PartialEq)]
pub struct ProviderInfo {
    pub id: String,
    pub name: String,
    pub kind: ProviderKind,
    pub default_color: String,
    pub detection: Detection,
    pub command_path: Option<PathBuf>,
    /// What consent approves; stable across command path changes for built-ins.
    pub consent_key: Vec<String>,
    /// The full command shown in the consent dialog.
    pub command: Vec<String>,
    pub explanation: String,
    pub definition_error: Option<String>,
}

/// Detects commands and reads providers. The real one runs agent commands; tests fake it.
pub trait Backend: Send + Sync {
    fn detect(&self, names: &[&str]) -> DetectResult;
    fn read(
        &self,
        provider: &ProviderInfo,
        env: &[(String, String)],
        now: DateTime<Utc>,
    ) -> Reading;
    /// Custom providers from definition files.
    fn custom_providers(&self) -> Vec<ProviderInfo> {
        Vec::new()
    }
}

/// What the frontend renders (contracts/ipc.md `UsageState`).
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageState {
    pub enabled: bool,
    pub warnings: bool,
    pub summary: UsageSummary,
    pub providers: Vec<ProviderState>,
    pub providers_folder: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderState {
    pub id: String,
    pub name: String,
    pub kind: ProviderKind,
    pub color: String,
    pub enabled: bool,
    pub detection: Detection,
    pub command_path: Option<String>,
    pub consent: &'static str,
    pub command: Vec<String>,
    pub definition_error: Option<String>,
    pub reading: Option<Reading>,
    pub stale: bool,
    pub last_error: Option<String>,
    pub refreshing: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageWarning {
    pub provider_id: String,
    pub provider_name: String,
    pub window_label: String,
    pub level: u8,
    pub resets_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConsentRequest {
    pub provider_id: String,
    pub provider_name: String,
    pub kind: ProviderKind,
    pub command: Vec<String>,
    pub explanation: String,
}

#[derive(Default)]
struct Inner {
    prefs: UsagePreferences,
    cache: UsageCache,
    schedule: Schedule,
    detection: Option<DetectResult>,
    customs: Vec<ProviderInfo>,
    in_flight: HashSet<String>,
    last_error: HashMap<String, String>,
    emails: HashMap<String, String>,
    /// Command last shown in a consent request per provider, once per app run.
    consent_asked: HashMap<String, Vec<String>>,
}

pub struct UsageService {
    root: DataRoot,
    backend: Arc<dyn Backend>,
    events: Arc<dyn EventSink>,
    inner: Mutex<Inner>,
    warnings: Mutex<Vec<String>>,
}

impl UsageService {
    pub fn new(
        root: DataRoot,
        prefs: UsagePreferences,
        backend: Arc<dyn Backend>,
        events: Arc<dyn EventSink>,
    ) -> Self {
        let loaded = cache::load(&root);
        let customs = backend.custom_providers();
        Self {
            root,
            backend,
            events,
            inner: Mutex::new(Inner {
                prefs,
                cache: loaded.value,
                customs,
                ..Inner::default()
            }),
            warnings: Mutex::new(loaded.warning.into_iter().collect()),
        }
    }

    /// Load problems found at start (an unreadable cache), for the startup warnings.
    pub fn take_warnings(&self) -> Vec<String> {
        std::mem::take(&mut self.warnings.lock().unwrap_or_else(|e| e.into_inner()))
    }

    fn inner(&self) -> MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn state(&self) -> UsageState {
        build_state(&self.inner(), &self.root, Utc::now())
    }

    /// Runs the automatic readings that are due. Called by the shell's background loop.
    pub fn tick(&self) {
        self.refresh(RefreshReason::Auto, None);
    }

    /// Reads `provider` (or every provider) when the schedule allows it for `reason`.
    pub fn refresh(&self, reason: RefreshReason, provider: Option<&str>) -> UsageState {
        let all_manual = reason == RefreshReason::Manual && provider.is_none();
        let need_detect = {
            let inner = self.inner();
            if !inner.prefs.enabled {
                return build_state(&inner, &self.root, Utc::now());
            }
            inner.detection.is_none() || all_manual
        };
        if need_detect {
            let names: Vec<&str> = BUILT_INS.iter().map(|(id, _, _)| *id).collect();
            let found = self.backend.detect(&names);
            let customs = all_manual.then(|| self.backend.custom_providers());
            let mut inner = self.inner();
            inner.detection = Some(found);
            if let Some(customs) = customs {
                inner.customs = customs;
            }
        }

        let now = Instant::now();
        let wall = Utc::now();
        let (todo, env, asks) = {
            let mut inner = self.inner();
            let mut todo = Vec::new();
            let mut asks = Vec::new();
            for info in providers(&inner) {
                if provider.is_some_and(|p| p != info.id) || !readable_except_consent(&inner, &info)
                {
                    continue;
                }
                let consent = consent_of(&inner, &info.id);
                match consent.state_for(&info.consent_key) {
                    "granted" => {}
                    "unasked" => {
                        if inner.consent_asked.get(&info.id) != Some(&info.command) {
                            inner
                                .consent_asked
                                .insert(info.id.clone(), info.command.clone());
                            asks.push(consent_request(&info));
                        }
                        continue;
                    }
                    _ => continue,
                }
                if inner.in_flight.contains(&info.id) {
                    continue;
                }
                let cached = inner.cache.readings.get(&info.id);
                let cached_age = cached.map(|r| age(wall, r.read_at));
                let reset_passed = cached.is_some_and(|r| {
                    r.windows
                        .iter()
                        .any(|w| w.resets_at.is_some_and(|t| t <= wall))
                });
                if !inner
                    .schedule
                    .due(&info.id, reason, now, cached_age, reset_passed)
                {
                    continue;
                }
                inner.schedule.mark(&info.id, now);
                inner.in_flight.insert(info.id.clone());
                todo.push(info);
            }
            let env = inner
                .detection
                .as_ref()
                .map(|d| d.env.clone())
                .unwrap_or_default();
            (todo, env, asks)
        };
        for ask in asks {
            self.events.emit(CoreEvent::UsageConsent(ask));
        }
        if todo.is_empty() {
            return self.state();
        }
        self.emit_state();

        let readings: Vec<(ProviderInfo, Reading)> = std::thread::scope(|scope| {
            let handles: Vec<_> = todo
                .into_iter()
                .map(|info| {
                    let env = &env;
                    scope.spawn(move || {
                        let reading = self.backend.read(&info, env, Utc::now());
                        (info, reading)
                    })
                })
                .collect();
            handles.into_iter().filter_map(|h| h.join().ok()).collect()
        });

        let mut warnings = Vec::new();
        {
            let mut inner = self.inner();
            for (info, reading) in readings {
                warnings.extend(apply(&mut inner, &info, reading));
            }
            if let Err(e) = cache::save(&self.root, &inner.cache) {
                tracing::warn!("could not save usage cache: {e}");
            }
            if !inner.prefs.warnings {
                warnings.clear();
            }
        }
        for warning in warnings {
            self.events.emit(CoreEvent::UsageWarning(warning));
        }
        self.emit_state()
    }

    fn emit_state(&self) -> UsageState {
        let state = self.state();
        self.events.emit(CoreEvent::UsageState(state.clone()));
        state
    }

    fn change(&self, f: impl FnOnce(&mut Inner) -> AppResult<()>) -> AppResult<UsageState> {
        {
            let mut inner = self.inner();
            f(&mut inner)?;
            let store = Store::new(self.root.clone());
            let mut prefs = store.load_preferences().value;
            prefs.usage = inner.prefs.clone();
            store.save_preferences(&prefs)?;
        }
        Ok(self.emit_state())
    }

    /// Master switch (FR-016).
    pub fn set_enabled(&self, enabled: bool) -> AppResult<UsageState> {
        self.change(|inner| {
            inner.prefs.enabled = enabled;
            Ok(())
        })
    }

    /// Warning notifications on or off (FR-011).
    pub fn set_warnings(&self, enabled: bool) -> AppResult<UsageState> {
        self.change(|inner| {
            inner.prefs.warnings = enabled;
            Ok(())
        })
    }

    /// Per-agent switch. Turning on a provider the user denied asks for consent again.
    pub fn set_provider_enabled(&self, id: &str, enabled: bool) -> AppResult<UsageState> {
        self.change(|inner| {
            find(inner, id)?;
            let entry = inner.prefs.providers.entry(id.to_string()).or_default();
            entry.enabled = Some(enabled);
            if enabled && entry.consent == Some(Consent::Denied) {
                entry.consent = None;
            }
            if enabled {
                inner.consent_asked.remove(id);
            }
            Ok(())
        })
    }

    pub fn set_provider_color(&self, id: &str, color: &str) -> AppResult<UsageState> {
        if !valid_color(color) {
            return Err(AppError::new(
                ErrorCode::InvalidColor,
                format!("'{color}' is not a color like #3b82f6."),
            ));
        }
        self.change(|inner| {
            find(inner, id)?;
            inner
                .prefs
                .providers
                .entry(id.to_string())
                .or_default()
                .color = Some(color.to_ascii_lowercase());
            Ok(())
        })
    }

    /// Records the answer to a consent request. `command` must be the command the user saw.
    pub fn answer_consent(
        &self,
        id: &str,
        allow: bool,
        command: Vec<String>,
    ) -> AppResult<UsageState> {
        self.change(|inner| {
            let info = find(inner, id)?;
            if command != info.command && command != info.consent_key {
                return Err(AppError::new(
                    ErrorCode::ConsentMismatch,
                    "The command changed since you were asked. Review it again.",
                ));
            }
            let entry = inner.prefs.providers.entry(id.to_string()).or_default();
            if allow {
                entry.consent = Some(Consent::granted(info.consent_key));
                entry.enabled = Some(true);
            } else {
                entry.consent = Some(Consent::Denied);
                entry.enabled = Some(false);
            }
            Ok(())
        })
    }
}

fn find(inner: &Inner, id: &str) -> AppResult<ProviderInfo> {
    providers(inner)
        .into_iter()
        .find(|p| p.id == id)
        .ok_or_else(|| {
            AppError::new(
                ErrorCode::UsageProviderNotFound,
                format!("No usage provider '{id}'."),
            )
        })
}

pub fn valid_color(color: &str) -> bool {
    color.len() == 7 && color.starts_with('#') && color[1..].chars().all(|c| c.is_ascii_hexdigit())
}

fn age(now: DateTime<Utc>, then: DateTime<Utc>) -> Duration {
    (now - then).to_std().unwrap_or(Duration::ZERO)
}

/// Built-ins (with detection results) followed by custom providers sorted by name.
fn providers(inner: &Inner) -> Vec<ProviderInfo> {
    let mut list: Vec<ProviderInfo> = BUILT_INS
        .iter()
        .map(|(id, name, color)| {
            let (detection, path) = inner
                .detection
                .as_ref()
                .map(|d| d.command(id))
                .unwrap_or((Detection::Unknown, None));
            built_in(id, name, color, detection, path)
        })
        .collect();
    let mut customs = inner.customs.clone();
    customs.sort_by_key(|c| c.name.to_lowercase());
    list.extend(customs);
    list
}

fn built_in(
    id: &str,
    name: &str,
    color: &str,
    detection: Detection,
    path: Option<PathBuf>,
) -> ProviderInfo {
    let (args, explanation): (Vec<&str>, &str) = match id {
        "claude" => (
            super::claude::PROBE_ARGS.to_vec(),
            "Sonata runs `claude auth status --json` and then this command, which sends one \
             tiny request (about 600 tokens on Haiku) so Claude Code reports your plan limits. \
             Claude Code uses its own login; Sonata never reads it. Automatic reads run every \
             5 minutes while usage reading is on.",
        ),
        _ => (
            vec!["app-server"],
            "Sonata starts Codex's app server and asks it for your rate limits. No model \
             request is made. Codex uses its own login; Sonata never reads it. When the app \
             server is not available, Sonata reads the limits Codex recorded in its newest \
             session file.",
        ),
    };
    let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    let shown = path
        .as_ref()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| id.to_string());
    ProviderInfo {
        id: id.into(),
        name: name.into(),
        kind: ProviderKind::BuiltIn,
        default_color: color.into(),
        detection,
        command_path: path,
        consent_key: std::iter::once(id.to_string())
            .chain(args.iter().cloned())
            .collect(),
        command: std::iter::once(shown).chain(args).collect(),
        explanation: explanation.into(),
        definition_error: None,
    }
}

fn consent_of(inner: &Inner, id: &str) -> Consent {
    inner
        .prefs
        .providers
        .get(id)
        .and_then(|p| p.consent.clone())
        .unwrap_or_default()
}

fn switched_on(inner: &Inner, id: &str) -> bool {
    let prefs = inner.prefs.providers.get(id);
    prefs.and_then(|p| p.enabled).unwrap_or(true)
        && prefs.and_then(|p| p.consent.as_ref()) != Some(&Consent::Denied)
}

fn readable_except_consent(inner: &Inner, info: &ProviderInfo) -> bool {
    inner.prefs.enabled
        && switched_on(inner, &info.id)
        && info.definition_error.is_none()
        && info.detection == Detection::Found
}

fn consent_request(info: &ProviderInfo) -> ConsentRequest {
    ConsentRequest {
        provider_id: info.id.clone(),
        provider_name: info.name.clone(),
        kind: info.kind,
        command: info.command.clone(),
        explanation: info.explanation.clone(),
    }
}

/// Stores one reading; returns the warnings it triggers.
fn apply(inner: &mut Inner, info: &ProviderInfo, mut reading: Reading) -> Vec<UsageWarning> {
    inner.in_flight.remove(&info.id);
    let now = reading.read_at;
    reading.windows = current_windows(std::mem::take(&mut reading.windows), now);
    if !cache::is_cacheable(reading.status) {
        let reason = reading
            .message
            .clone()
            .unwrap_or_else(|| "Usage not available.".into());
        tracing::info!(provider = %info.id, "usage read failed: {reason}");
        inner.last_error.insert(info.id.clone(), reason);
        if reading.status == ReadingStatus::NotInstalled {
            inner.detection = None;
        }
        return Vec::new();
    }
    inner.last_error.remove(&info.id);
    match reading.account_email.clone() {
        Some(email) => inner.emails.insert(info.id.clone(), email),
        None => inner.emails.remove(&info.id),
    };
    let crossings = if reading.status == ReadingStatus::Ok {
        window::observe(&mut inner.cache.warnings, &info.id, &reading.windows, now)
    } else {
        Vec::new()
    };
    inner.cache.readings.insert(info.id.clone(), reading);
    crossings
        .into_iter()
        .map(|(w, level)| UsageWarning {
            provider_id: info.id.clone(),
            provider_name: info.name.clone(),
            window_label: w.label,
            level: level.percent(),
            resets_at: w.resets_at,
        })
        .collect()
}

fn build_state(inner: &Inner, root: &DataRoot, now: DateTime<Utc>) -> UsageState {
    let list: Vec<ProviderState> = providers(inner)
        .into_iter()
        .map(|info| {
            let enabled = switched_on(inner, &info.id);
            let mut reading = inner.cache.readings.get(&info.id).cloned();
            if let Some(r) = reading.as_mut() {
                r.account_email = inner.emails.get(&info.id).cloned();
                r.windows = current_windows(std::mem::take(&mut r.windows), now);
            }
            if info.detection == Detection::NotFound && info.definition_error.is_none() {
                reading = Some(
                    Reading::new(info.id.clone(), ReadingStatus::NotInstalled, now)
                        .with_message("Command not found."),
                );
            }
            let stale = reading.as_ref().is_some_and(|r| {
                r.status == ReadingStatus::Ok && age(now, r.read_at) > STALE_AFTER
            });
            let consent = consent_of(inner, &info.id);
            ProviderState {
                color: inner
                    .prefs
                    .providers
                    .get(&info.id)
                    .and_then(|p| p.color.clone())
                    .unwrap_or_else(|| info.default_color.clone()),
                enabled,
                detection: info.detection,
                command_path: info.command_path.as_ref().map(|p| p.display().to_string()),
                consent: consent.state_for(&info.consent_key),
                command: info.consent_key.clone(),
                definition_error: info.definition_error.clone(),
                reading,
                stale,
                last_error: inner.last_error.get(&info.id).cloned(),
                refreshing: inner.in_flight.contains(&info.id),
                id: info.id,
                name: info.name,
                kind: info.kind,
            }
        })
        .collect();
    let summary = window::summary(
        inner.prefs.enabled,
        list.iter()
            .filter(|p| p.enabled && p.consent == "granted")
            .filter_map(|p| p.reading.as_ref())
            .filter(|r| r.status == ReadingStatus::Ok)
            .map(|r| (r.provider_id.as_str(), r.windows.as_slice())),
    );
    UsageState {
        enabled: inner.prefs.enabled,
        warnings: inner.prefs.warnings,
        summary,
        providers: list,
        providers_folder: root.usage_providers_dir().display().to_string(),
    }
}

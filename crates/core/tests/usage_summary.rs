use sonata_core::usage::window::{SummaryState, summary};
use sonata_core::usage::{Window, WindowKind};

fn w(kind: WindowKind, percent: f64) -> Window {
    Window {
        kind,
        label: String::new(),
        used_percent: percent,
        resets_at: None,
    }
}

#[test]
fn highest_window_across_providers_wins() {
    let a = [w(WindowKind::Short, 40.0), w(WindowKind::Weekly, 55.0)];
    let b = [w(WindowKind::Short, 70.0)];
    let s = summary(true, [("claude", &a[..]), ("codex", &b[..])]);
    assert_eq!(s.state, SummaryState::Normal);
    assert_eq!(s.percent, Some(70.0));
    assert_eq!(s.provider_id.as_deref(), Some("codex"));
    assert_eq!(s.window_kind, Some(WindowKind::Short));
}

#[test]
fn warning_and_full_states() {
    let a = [w(WindowKind::Short, 80.0)];
    assert_eq!(
        summary(true, [("claude", &a[..])]).state,
        SummaryState::Warning
    );
    let b = [w(WindowKind::Weekly, 100.0)];
    assert_eq!(
        summary(true, [("claude", &b[..])]).state,
        SummaryState::Full
    );
}

#[test]
fn off_and_empty_states() {
    let a = [w(WindowKind::Short, 90.0)];
    assert_eq!(
        summary(false, [("claude", &a[..])]).state,
        SummaryState::Off
    );
    assert_eq!(summary(false, [("claude", &a[..])]).percent, None);
    assert_eq!(summary(true, std::iter::empty()).state, SummaryState::Empty);
}

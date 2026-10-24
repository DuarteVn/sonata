mod common;

use std::sync::Arc;

use common::Sandbox;
use common::fake_bin::{fake_bin, usage_fixture};
use sonata_core::events::NullSink;
use sonata_core::model::preferences::UsagePreferences;
use sonata_core::usage::schedule::RefreshReason;
use sonata_core::usage::service::UsageService;
use sonata_core::usage::system::SystemBackend;

const TOKEN: &str = "sk-ant-oat01-FAKEFAKEFAKEFAKEFAKEFAKEFAKEFAKE";

fn all_text(dir: &std::path::Path, out: &mut String) {
    for entry in std::fs::read_dir(dir).unwrap().flatten() {
        let path = entry.path();
        if path.is_dir() {
            all_text(&path, out);
        } else {
            out.push_str(&String::from_utf8_lossy(&std::fs::read(&path).unwrap()));
        }
    }
}

/// SC-005: after a full cycle (good reading, then a failing probe that prints a token), no
/// file in the data folder, logs included, holds the token or the login email.
#[test]
fn no_credential_or_email_reaches_sonata_files_or_logs() {
    let sb = Sandbox::new();
    let guard = sonata_core::logging::init(&sb.root).unwrap();
    let bin_dir = sb.tmp.path().join("bin");
    let flag = sb.tmp.path().join("fail");
    fake_bin(
        &bin_dir,
        "claude",
        &format!(
            r#"if [ "$1" = auth ]; then cat '{auth}'; exit 0; fi
if [ -e '{flag}' ]; then echo "error: bad token {TOKEN}" >&2; echo garbage; exit 1; fi
cat '{stream}'"#,
            auth = usage_fixture("claude-auth-team.json").display(),
            stream = usage_fixture("claude-stream-ok.jsonl").display(),
            flag = flag.display(),
        ),
    );
    let shell = fake_bin(&bin_dir, "loginsh", r#"shift; exec /bin/sh "$@""#);
    let svc = UsageService::new(
        sb.root.clone(),
        UsagePreferences::default(),
        Arc::new(SystemBackend {
            shell,
            extra_path: Some(bin_dir.clone()),
            root: sb.root.clone(),
        }),
        Arc::new(NullSink),
    );
    svc.refresh(RefreshReason::Auto, None);
    let cmd = svc.state().providers[0].command.clone();
    svc.answer_consent("claude", true, cmd).unwrap();
    let st = svc.refresh(RefreshReason::Manual, Some("claude"));
    assert_eq!(
        st.providers[0]
            .reading
            .as_ref()
            .unwrap()
            .account_email
            .as_deref(),
        Some("user@example.com"),
        "{:?}",
        st.providers[0].last_error
    );
    std::fs::write(&flag, "").unwrap();
    let st = svc.refresh(RefreshReason::Manual, Some("claude"));
    assert!(st.providers[0].last_error.is_some());
    drop(guard);

    let mut text = String::new();
    all_text(sb.root.path(), &mut text);
    assert!(text.contains("Example Org"), "cache was written");
    assert!(
        text.contains("claude probe stderr"),
        "stderr was logged, redacted"
    );
    assert!(!text.contains("FAKEFAKE"), "token leaked");
    assert!(!text.contains("user@example.com"), "email leaked");
}

mod common;

use chrono::Utc;
use common::fake_bin::fake_bin;
use sonata_core::usage::ReadingStatus;
use sonata_core::usage::custom::{SANDBOX_EXEC, read_custom, sandbox_profile};

#[test]
fn profile_denies_credential_stores_and_sonata_data() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home \"q\"");
    let root = tmp.path().join("sonata");
    std::fs::create_dir_all(&home).unwrap();
    std::fs::create_dir_all(&root).unwrap();
    let p = sandbox_profile(&home, &root);
    let real_home = home
        .canonicalize()
        .unwrap()
        .display()
        .to_string()
        .replace('"', "\\\"");
    let real_root = root.canonicalize().unwrap().display().to_string();
    assert!(p.starts_with("(version 1)\n(allow default)"), "{p}");
    assert!(
        p.contains(&format!("(subpath \"{real_home}/Library/Keychains\")")),
        "{p}"
    );
    assert!(p.contains("(subpath \"/Library/Keychains\")"));
    assert!(p.contains(&format!(
        "(literal \"{real_home}/.claude/.credentials.json\")"
    )));
    assert!(p.contains(&format!("(literal \"{real_home}/.codex/auth.json\")")));
    assert!(p.contains(&format!("(subpath \"{real_root}\")")));
    assert!(p.contains("com\\.apple\\.secur"));
    assert!(p.trim_end().ends_with(&format!(
        "(allow file-read* (subpath \"{real_root}/usage-providers\"))"
    )));
}

#[test]
fn sandboxed_command_cannot_read_sonata_data_but_can_read_its_folder() {
    if !std::path::Path::new(SANDBOX_EXEC).exists() {
        return;
    }
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("sonata");
    let providers = root.join("usage-providers");
    std::fs::create_dir_all(&providers).unwrap();
    std::fs::write(root.join("secret.json"), "secret").unwrap();
    std::fs::write(providers.join("ok.txt"), "ok").unwrap();
    let ok_json = r#"{"windows":[{"kind":"short","usedPercent":5}]}"#;
    let reads_folder = fake_bin(
        &providers,
        "reads-folder",
        &format!(
            "cat '{}/ok.txt' >/dev/null && echo '{ok_json}'",
            providers.display()
        ),
    );
    let r = read_custom(
        "m",
        &[reads_folder.display().to_string()],
        &[],
        tmp.path(),
        &root,
        Utc::now(),
    );
    assert_eq!(r.status, ReadingStatus::Ok, "{:?}", r.message);
    let reads_secret = fake_bin(
        tmp.path(),
        "reads-secret",
        &format!("cat '{}/secret.json' && echo '{ok_json}'", root.display()),
    );
    let r = read_custom(
        "m",
        &[reads_secret.display().to_string()],
        &[],
        tmp.path(),
        &root,
        Utc::now(),
    );
    assert_eq!(r.status, ReadingStatus::Unavailable);
    assert_eq!(r.message.as_deref(), Some("exited with status 1"));
}

#[test]
fn without_sandbox_exec_nothing_runs() {
    let tmp = tempfile::tempdir().unwrap();
    let marker = tmp.path().join("ran");
    let bin = fake_bin(tmp.path(), "tool", &format!("touch '{}'", marker.display()));
    let r = sonata_core::usage::custom::read_custom_with(
        std::path::Path::new("/nonexistent/sandbox-exec"),
        "m",
        &[bin.display().to_string()],
        &[],
        tmp.path(),
        tmp.path(),
        Utc::now(),
    );
    assert_eq!(r.status, ReadingStatus::Unavailable);
    assert_eq!(
        r.message.as_deref(),
        Some("Cannot run safely: sandbox-exec not found.")
    );
    assert!(!marker.exists());
}

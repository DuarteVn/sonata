//! The usage runner on Windows: Job Object kill and the Windows environment allowlist
//! (feature 012, research R11).
#![cfg(windows)]

use std::time::Duration;

use sonata_core::usage::runner::{ENV_ALLOWLIST, RunError, RunSpec, run};

fn cmd(args: &str) -> Vec<String> {
    vec![
        std::env::var("ComSpec").unwrap(),
        "/C".to_string(),
        args.to_string(),
    ]
}

#[test]
fn output_is_captured_and_the_environment_is_scrubbed() {
    let mut spec = RunSpec::new(cmd("set"), Duration::from_secs(10));
    spec.env.push(("SONATA_EXTRA".into(), "yes".into()));
    let out = run(&spec).unwrap();
    assert_eq!(out.code, Some(0));
    let text = String::from_utf8_lossy(&out.stdout).to_ascii_uppercase();
    assert!(text.contains("SYSTEMROOT="), "{text}");
    assert!(text.contains("SONATA_EXTRA=YES"), "{text}");
    for line in text.lines() {
        let key = line.split('=').next().unwrap_or_default();
        let allowed = ENV_ALLOWLIST
            .iter()
            .any(|k| k.eq_ignore_ascii_case(key))
            || key == "SONATA_EXTRA"
            // cmd.exe adds these itself.
            || ["PROMPT", "COMSPEC", "PATHEXT"].contains(&key);
        assert!(allowed, "unexpected variable {key}");
    }
}

#[test]
fn a_timeout_kills_the_whole_job() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("grandchild.txt");
    // `cmd` (the child) starts `ping` (the grandchild) with its output in `out`; Windows refuses
    // to delete a file a running process holds open. The temp path has no spaces, so it needs no
    // quotes (Windows argument quoting and `cmd /C` disagree about them).
    let script = format!("ping -n 60 127.0.0.1 > {}", out.display());
    let err = run(&RunSpec::new(cmd(&script), Duration::from_secs(5))).unwrap_err();
    assert!(matches!(err, RunError::TimedOut(_)), "{err}");
    assert!(out.exists(), "the grandchild did not start");
    let deleted = (0..20).any(|_| {
        std::thread::sleep(Duration::from_millis(100));
        std::fs::remove_file(&out).is_ok()
    });
    assert!(deleted, "the grandchild outlived the run");
}

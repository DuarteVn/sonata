mod common;

use std::time::{Duration, Instant};

use common::fake_bin::fake_bin;
use sonata_core::usage::runner::{RunError, RunSpec, redact, run};

fn spec(argv: Vec<String>, timeout_ms: u64) -> RunSpec {
    RunSpec::new(argv, Duration::from_millis(timeout_ms))
}

fn alive(pid: i32) -> bool {
    unsafe { libc::kill(pid, 0) == 0 }
}

fn read_pid(path: &std::path::Path) -> i32 {
    common::wait_until(Duration::from_secs(2), || path.exists());
    std::fs::read_to_string(path)
        .unwrap()
        .trim()
        .parse()
        .unwrap()
}

#[test]
fn arguments_arrive_literally_without_a_shell() {
    let dir = tempfile::tempdir().unwrap();
    let bin = fake_bin(dir.path(), "args", r#"printf '%s\n' "$@""#);
    let out = run(&spec(
        vec![
            bin.display().to_string(),
            "a b".into(),
            "$HOME".into(),
            ";ls".into(),
        ],
        5000,
    ))
    .unwrap();
    assert_eq!(String::from_utf8(out.stdout).unwrap(), "a b\n$HOME\n;ls\n");
    assert_eq!(out.code, Some(0));
}

#[test]
fn runs_in_a_fresh_empty_directory() {
    let dir = tempfile::tempdir().unwrap();
    let bin = fake_bin(dir.path(), "where", "pwd; ls -A | wc -l");
    let out = run(&spec(vec![bin.display().to_string()], 5000)).unwrap();
    let text = String::from_utf8(out.stdout).unwrap();
    let mut lines = text.lines();
    let cwd = lines.next().unwrap();
    assert_ne!(cwd, std::env::current_dir().unwrap().display().to_string());
    assert_eq!(lines.next().unwrap().trim(), "0");
}

#[test]
fn environment_holds_only_the_allowlist() {
    let dir = tempfile::tempdir().unwrap();
    let bin = fake_bin(dir.path(), "env", "env");
    let mut s = spec(vec![bin.display().to_string()], 5000);
    s.env.push(("CODEX_HOME".into(), "/tmp/codex".into()));
    let out = run(&s).unwrap();
    let text = String::from_utf8(out.stdout).unwrap();
    let allowed = [
        "HOME",
        "USER",
        "LOGNAME",
        "LANG",
        "TMPDIR",
        "PATH",
        "CODEX_HOME",
        "PWD",
        "SHLVL",
        "_",
        "OLDPWD",
    ];
    for line in text.lines() {
        let key = line.split('=').next().unwrap();
        assert!(allowed.contains(&key), "unexpected variable {key}");
    }
    assert!(text.contains("CODEX_HOME=/tmp/codex"));
    assert!(text.lines().any(|l| l.starts_with("PATH=")));
}

#[test]
fn timeout_kills_the_whole_process_group() {
    let dir = tempfile::tempdir().unwrap();
    let pid_file = dir.path().join("pid");
    let bin = fake_bin(dir.path(), "slow", r#"sleep 30 & echo $! > "$1"; wait"#);
    let start = Instant::now();
    let err = run(&spec(
        vec![bin.display().to_string(), pid_file.display().to_string()],
        500,
    ))
    .unwrap_err();
    assert!(matches!(err, RunError::TimedOut(_)), "{err:?}");
    assert_eq!(err.to_string(), "timed out after 0.5 s");
    assert!(start.elapsed() < Duration::from_secs(3));
    let pid = read_pid(&pid_file);
    assert!(common::wait_until(Duration::from_secs(2), || !alive(pid)));
}

#[test]
fn background_children_die_when_the_command_exits() {
    let dir = tempfile::tempdir().unwrap();
    let pid_file = dir.path().join("pid");
    let bin = fake_bin(dir.path(), "bg", r#"sleep 30 & echo $! > "$1"; echo done"#);
    let start = Instant::now();
    let out = run(&spec(
        vec![bin.display().to_string(), pid_file.display().to_string()],
        10_000,
    ))
    .unwrap();
    assert_eq!(String::from_utf8(out.stdout).unwrap(), "done\n");
    assert!(start.elapsed() < Duration::from_secs(5));
    let pid = read_pid(&pid_file);
    assert!(common::wait_until(Duration::from_secs(2), || !alive(pid)));
}

#[test]
fn stdout_over_the_cap_is_an_error() {
    let dir = tempfile::tempdir().unwrap();
    let bin = fake_bin(dir.path(), "big", "head -c 70000 /dev/zero | tr '\\0' 'x'");
    let err = run(&spec(vec![bin.display().to_string()], 5000)).unwrap_err();
    assert!(matches!(err, RunError::OutputTooLarge(65536)), "{err:?}");
    assert_eq!(err.to_string(), "printed more than 64 KiB");
}

#[test]
fn non_zero_exit_keeps_output_and_capped_stderr() {
    let dir = tempfile::tempdir().unwrap();
    let bin = fake_bin(
        dir.path(),
        "fail",
        "head -c 204800 /dev/zero | tr '\\0' 'e' >&2; echo out; exit 3",
    );
    let out = run(&spec(vec![bin.display().to_string()], 5000)).unwrap();
    assert_eq!(out.code, Some(3));
    assert_eq!(String::from_utf8(out.stdout).unwrap(), "out\n");
    assert!(out.stderr.len() <= 4096);
}

#[test]
fn stdin_is_written_and_stop_condition_ends_the_run() {
    let dir = tempfile::tempdir().unwrap();
    let bin = fake_bin(
        dir.path(),
        "echo",
        r#"read line; echo "got $line"; sleep 30"#,
    );
    let mut s = spec(vec![bin.display().to_string()], 10_000);
    s.stdin = Some(b"hello\n".to_vec());
    s.stop_when = Some(|out: &[u8]| out.ends_with(b"\n") && out.starts_with(b"got"));
    let start = Instant::now();
    let out = run(&s).unwrap();
    assert!(out.stopped);
    assert_eq!(String::from_utf8(out.stdout).unwrap(), "got hello\n");
    assert!(start.elapsed() < Duration::from_secs(5));
}

#[test]
fn missing_program_is_not_found() {
    let err = run(&spec(vec!["/nonexistent/sonata-test-tool".into()], 1000)).unwrap_err();
    assert!(matches!(err, RunError::NotFound), "{err:?}");
}

#[test]
fn redact_hides_token_like_strings() {
    let text = redact("error: token sk-ant-oat01-ABCDEFGHIJKLMNOPQRSTUVWXYZ012345 rejected");
    assert!(!text.contains("ABCDEFGHIJ"), "{text}");
    assert!(text.contains("[redacted]"));
    assert!(text.contains("rejected"));
    assert_eq!(redact("short words stay"), "short words stay");
}

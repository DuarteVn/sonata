//! Custom usage providers on Linux run inside bubblewrap, which hides credential stores and
//! Sonata's data but keeps the providers folder readable (feature 011, research R9).
// Unix shells, modes, or sockets; Windows counterparts live in *_windows.rs (feature 012).
#![cfg(unix)]

mod common;

use std::path::{Path, PathBuf};

use sonata_core::platform::bwrap;

#[test]
fn arguments_hide_secrets_and_sonata_data() {
    let home = Path::new("/home/me");
    let root = Path::new("/home/me/.sonata");
    let present: Vec<PathBuf> = [
        "/home/me/.sonata/usage-providers",
        "/home/me/.claude/.credentials.json",
        "/home/me/.local/share/keyrings",
        "/run/user/1000",
    ]
    .iter()
    .map(PathBuf::from)
    .collect();
    let args = bwrap::args(home, root, Some(Path::new("/run/user/1000")), |p| {
        present.iter().any(|q| q == p)
    });
    let expected: Vec<String> = [
        "--die-with-parent",
        "--new-session",
        "--unshare-pid",
        "--unshare-ipc",
        "--dev-bind",
        "/",
        "/",
        "--proc",
        "/proc",
        "--tmpfs",
        "/home/me/.sonata",
        "--ro-bind",
        "/home/me/.sonata/usage-providers",
        "/home/me/.sonata/usage-providers",
        "--ro-bind",
        "/dev/null",
        "/home/me/.claude/.credentials.json",
        "--tmpfs",
        "/home/me/.local/share/keyrings",
        "--tmpfs",
        "/run/user/1000",
        "--",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();
    assert_eq!(args, expected);
}

#[test]
fn missing_paths_are_left_out() {
    let args = bwrap::args(Path::new("/h"), Path::new("/h/.sonata"), None, |_| false);
    assert_eq!(
        args.join(" "),
        "--die-with-parent --new-session --unshare-pid --unshare-ipc --dev-bind / / \
         --proc /proc --tmpfs /h/.sonata --"
    );
}

#[test]
fn without_bubblewrap_nothing_runs() {
    let tmp = tempfile::tempdir().unwrap();
    let marker = tmp.path().join("ran");
    let bin =
        common::fake_bin::fake_bin(tmp.path(), "tool", &format!("touch '{}'", marker.display()));
    let r = sonata_core::usage::custom::read_custom_bwrap(
        Path::new("/nonexistent/bwrap"),
        "m",
        &[bin.display().to_string()],
        &[],
        tmp.path(),
        tmp.path(),
        chrono::Utc::now(),
    );
    assert_eq!(r.status, sonata_core::usage::ReadingStatus::Unavailable);
    assert_eq!(r.message.as_deref(), Some(bwrap::MISSING));
    assert!(!marker.exists());
}

#[cfg(target_os = "linux")]
mod linux {
    use chrono::Utc;
    use sonata_core::usage::ReadingStatus;
    use sonata_core::usage::custom::read_custom;

    use crate::common::fake_bin::fake_bin;

    const OK_JSON: &str = r#"{"windows":[{"kind":"short","usedPercent":5}]}"#;

    #[test]
    fn sandboxed_provider_reads_its_folder_but_not_secrets() {
        if !std::path::Path::new(sonata_core::platform::bwrap::BWRAP).exists() {
            eprintln!("bubblewrap missing; skipped");
            return;
        }
        let tmp = tempfile::tempdir().unwrap();
        let home = tmp.path().join("home");
        let root = home.join(".sonata");
        let providers = root.join("usage-providers");
        std::fs::create_dir_all(&providers).unwrap();
        std::fs::create_dir_all(home.join(".codex")).unwrap();
        std::fs::write(root.join("secret.json"), "secret").unwrap();
        std::fs::write(home.join(".codex/auth.json"), "token").unwrap();
        std::fs::write(providers.join("ok.txt"), "ok").unwrap();

        let reads_folder = fake_bin(
            &providers,
            "reads-folder",
            &format!(
                "cat '{}/ok.txt' >/dev/null && echo '{OK_JSON}'",
                providers.display()
            ),
        );
        let r = read_custom(
            "m",
            &[reads_folder.display().to_string()],
            &[],
            &home,
            &root,
            Utc::now(),
        );
        assert_eq!(r.status, ReadingStatus::Ok, "{:?}", r.message);

        for secret in [root.join("secret.json"), home.join(".codex/auth.json")] {
            let reads = fake_bin(
                tmp.path(),
                "reads-secret",
                &format!("grep -q . '{}' && echo '{OK_JSON}'", secret.display()),
            );
            let r = read_custom(
                "m",
                &[reads.display().to_string()],
                &[],
                &home,
                &root,
                Utc::now(),
            );
            assert_eq!(
                r.status,
                ReadingStatus::Unavailable,
                "{} was readable",
                secret.display()
            );
        }
    }
}

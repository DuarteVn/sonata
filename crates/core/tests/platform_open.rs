//! Opening links and revealing files uses the system tool of each OS (feature 011, R13).

use std::ffi::OsString;
use std::path::Path;
#[cfg(unix)]
use std::path::PathBuf;

use sonata_core::platform::{open_url_command, reveal_command};

fn args(list: &[&str]) -> Vec<OsString> {
    list.iter().map(OsString::from).collect()
}

#[cfg(target_os = "macos")]
#[test]
fn macos_uses_open() {
    let open = open_url_command("https://example.com");
    assert_eq!(open.program, PathBuf::from("/usr/bin/open"));
    assert_eq!(open.args, args(&["https://example.com"]));
    let reveal = reveal_command(Path::new("/Users/me/.sonata/ws/a.json"));
    assert_eq!(reveal.program, PathBuf::from("/usr/bin/open"));
    assert_eq!(reveal.args, args(&["-R", "/Users/me/.sonata/ws/a.json"]));
}

#[cfg(target_os = "linux")]
#[test]
fn linux_uses_xdg_open_and_opens_the_folder() {
    let open = open_url_command("https://example.com");
    assert_eq!(open.program, PathBuf::from("xdg-open"));
    assert_eq!(open.args, args(&["https://example.com"]));
    let reveal = reveal_command(Path::new("/home/me/.sonata/ws/a.json"));
    assert_eq!(reveal.program, PathBuf::from("xdg-open"));
    assert_eq!(reveal.args, args(&["/home/me/.sonata/ws"]));
}

#[cfg(windows)]
#[test]
fn windows_uses_the_url_handler_and_explorer() {
    let open = open_url_command("https://example.com");
    assert!(
        open.program.ends_with(r"System32\rundll32.exe"),
        "{:?}",
        open.program
    );
    assert_eq!(
        open.args,
        args(&["url.dll,FileProtocolHandler", "https://example.com"])
    );
    let reveal = reveal_command(Path::new(r"C:\Users\me\.sonata\ws\a.json"));
    assert!(
        reveal.program.ends_with("explorer.exe"),
        "{:?}",
        reveal.program
    );
    assert_eq!(
        reveal.args,
        args(&[r"/select,C:\Users\me\.sonata\ws\a.json"])
    );
}

#[cfg(unix)]
mod spawn_checked {
    use std::time::{Duration, Instant};

    use sonata_core::platform::Launch;

    fn shell(script: &str) -> Launch {
        Launch {
            program: "/bin/sh".into(),
            args: vec!["-c".into(), script.into()],
        }
    }

    #[test]
    fn a_clean_exit_is_success() {
        assert!(
            shell("exit 0")
                .spawn_checked(Duration::from_secs(5))
                .unwrap()
        );
    }

    #[test]
    fn an_error_exit_is_failure() {
        assert!(
            !shell("exit 3")
                .spawn_checked(Duration::from_secs(5))
                .unwrap()
        );
    }

    #[test]
    fn a_program_still_running_counts_as_success() {
        let started = Instant::now();
        assert!(
            shell("sleep 5")
                .spawn_checked(Duration::from_millis(100))
                .unwrap()
        );
        assert!(started.elapsed() < Duration::from_secs(4));
    }

    #[test]
    fn a_missing_program_is_an_error() {
        let launch = Launch {
            program: "/nonexistent/opener".into(),
            args: vec![],
        };
        assert!(launch.spawn_checked(Duration::from_secs(1)).is_err());
    }
}

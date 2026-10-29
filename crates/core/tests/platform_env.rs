//! Programs started from an AppImage must not inherit the AppImage's own library and data paths
//! (feature 011, research R6).

use sonata_core::platform::{EnvFix, host_env};

fn vars(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
    pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

fn sorted(mut fix: EnvFix) -> EnvFix {
    fix.remove.sort();
    fix.set.sort();
    fix
}

#[test]
fn nothing_changes_outside_an_appimage() {
    let fix = host_env(vars(&[
        ("PATH", "/usr/bin:/bin"),
        ("LD_LIBRARY_PATH", "/opt/lib"),
    ]));
    assert!(fix.is_empty());
}

#[test]
fn appimage_variables_and_paths_are_removed() {
    let fix = sorted(host_env(vars(&[
        ("APPIMAGE", "/home/me/Sonata.AppImage"),
        ("APPDIR", "/tmp/.mount_Sonata"),
        ("ARGV0", "./Sonata.AppImage"),
        ("OWD", "/home/me"),
        ("PATH", "/tmp/.mount_Sonata/usr/bin:/usr/bin:/bin"),
        ("LD_LIBRARY_PATH", "/tmp/.mount_Sonata/usr/lib"),
        (
            "XDG_DATA_DIRS",
            "/tmp/.mount_Sonata/usr/share:/usr/local/share:/usr/share",
        ),
        ("GTK_PATH", "/tmp/.mount_Sonata/usr/lib/gtk-3.0"),
        ("HOME", "/home/me"),
        ("EDITOR", "vim"),
    ])));
    assert_eq!(
        fix.remove,
        [
            "APPDIR",
            "APPIMAGE",
            "ARGV0",
            "GTK_PATH",
            "LD_LIBRARY_PATH",
            "OWD"
        ]
    );
    assert_eq!(
        fix.set,
        [
            ("PATH".to_string(), "/usr/bin:/bin".to_string()),
            (
                "XDG_DATA_DIRS".to_string(),
                "/usr/local/share:/usr/share".to_string()
            ),
        ]
    );
}

#[test]
fn a_folder_that_only_shares_the_prefix_is_kept() {
    let fix = host_env(vars(&[
        ("APPIMAGE", "/a/Sonata.AppImage"),
        ("APPDIR", "/tmp/.mount_S"),
        ("PATH", "/tmp/.mount_Sonata/bin:/tmp/.mount_S/usr/bin"),
    ]));
    assert_eq!(
        fix.set,
        [("PATH".to_string(), "/tmp/.mount_Sonata/bin".to_string())]
    );
}

#[test]
fn applying_the_fix_to_a_command() {
    let fix = EnvFix {
        remove: vec!["APPDIR".into()],
        set: vec![("PATH".into(), "/usr/bin".into())],
    };
    let mut cmd = std::process::Command::new("env");
    fix.apply_std(&mut cmd);
    let envs: Vec<_> = cmd.get_envs().collect();
    assert!(envs.contains(&(std::ffi::OsStr::new("APPDIR"), None)));
    assert!(envs.contains(&(
        std::ffi::OsStr::new("PATH"),
        Some(std::ffi::OsStr::new("/usr/bin"))
    )));
}

//! Update commands (feature 005). The updater plugin does the network, signature, and bundle
//! work; `sonata_core::update` decides the state the banner shows.

use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use sonata_core::App;
use sonata_core::update::{Release, UpdateState};
use tauri::{AppHandle, Manager, State};
use tauri_plugin_updater::{Update, UpdaterExt};

use crate::commands::app::Quitting;
use crate::menu;

#[cfg(target_os = "macos")]
const WRONG_LOCATION: &str = "Move Sonata to Applications to update";
#[cfg(any(target_os = "linux", test))]
const PACKAGE_INSTALL: &str = "Sonata was installed with a package. Download the new version from https://github.com/DuarteVn/sonata/releases/latest";
#[cfg(any(target_os = "linux", test))]
const READ_ONLY_APPIMAGE: &str =
    "Move the Sonata AppImage to a folder you can write to, such as ~/Applications, to update";
/// How often the background task asks core whether an automatic check is due.
const TICK: Duration = Duration::from_secs(10 * 60);

/// The release found by the last check and, once downloaded and verified, its archive.
#[derive(Default)]
pub struct PendingUpdate {
    found: Mutex<Option<Update>>,
    bytes: Mutex<Option<Vec<u8>>>,
}

#[tauri::command]
pub fn update_state(app: State<'_, Arc<App>>) -> UpdateState {
    app.update_state()
}

/// Checks now. A manual check reports errors; a background one only logs them (FR-014).
#[tauri::command]
pub async fn update_check(
    handle: AppHandle,
    app: State<'_, Arc<App>>,
    pending: State<'_, PendingUpdate>,
    manual: bool,
) -> Result<UpdateState, String> {
    check(&handle, &app, &pending, manual).await
}

#[tauri::command]
pub fn update_dismiss(app: State<'_, Arc<App>>) -> UpdateState {
    app.update(|s| s.dismiss())
}

/// Downloads and verifies the update. Nothing is replaced and no terminal is stopped yet.
#[tauri::command]
pub async fn update_download(
    app: State<'_, Arc<App>>,
    pending: State<'_, PendingUpdate>,
) -> Result<UpdateState, String> {
    if let Some(reason) = install_blocker() {
        return Ok(app.update(|s| s.block(reason.into())));
    }
    let Some(update) = pending.found.lock().map_err(|e| e.to_string())?.clone() else {
        return Err("No update to download. Check for updates first.".into());
    };
    let mut started = false;
    app.update(|s| started = s.start_download());
    if !started {
        return Ok(app.update_state());
    }
    let mut received = 0u64;
    let mut shown = 0u64;
    let result = update
        .download(
            |chunk, total| {
                received += chunk as u64;
                // About one event per percent (or per 512 KB without a known size).
                let step = total.map_or(512 * 1024, |t| (t / 100).max(1));
                if received - shown >= step {
                    shown = received;
                    app.update(|s| s.progress(received, total));
                }
            },
            || {},
        )
        .await;
    match result {
        Ok(bytes) => {
            *pending.bytes.lock().map_err(|e| e.to_string())? = Some(bytes);
            Ok(app.update(|s| s.downloaded()))
        }
        Err(err) => {
            tracing::warn!(error = %err, "update download failed");
            Ok(app.update(|s| s.fail(err.to_string())))
        }
    }
}

/// Installs the downloaded update, saves and stops everything, and restarts. The frontend
/// calls this only after the quit check passed or the user confirmed (FR-012).
#[tauri::command]
pub fn update_install(
    handle: AppHandle,
    app: State<'_, Arc<App>>,
    pending: State<'_, PendingUpdate>,
    quitting: State<'_, Quitting>,
) -> Result<(), String> {
    let update = pending.found.lock().map_err(|e| e.to_string())?.clone();
    let bytes = pending.bytes.lock().map_err(|e| e.to_string())?.take();
    let (Some(update), Some(bytes)) = (update, bytes) else {
        return Err("The update is not downloaded yet.".into());
    };
    app.update(|s| s.installing());
    if let Err(err) = update.install(&bytes) {
        tracing::warn!(error = %err, "update install failed");
        app.update(|s| s.fail(err.to_string()));
        return Err(err.to_string());
    }
    tracing::info!(version = %update.version, "update installed; restarting");
    quitting.0.store(true, Ordering::SeqCst);
    app.quit();
    handle.restart()
}

#[tauri::command]
pub fn update_set_auto_check(
    handle: AppHandle,
    app: State<'_, Arc<App>>,
    enabled: bool,
) -> Result<(), String> {
    app.set_auto_update_check(enabled)
        .map_err(|e| e.to_string())?;
    menu::set_auto_check(&handle, enabled);
    Ok(())
}

async fn check(
    handle: &AppHandle,
    app: &App,
    pending: &PendingUpdate,
    manual: bool,
) -> Result<UpdateState, String> {
    let result = match handle.updater() {
        Ok(updater) => updater.check().await,
        Err(err) => Err(err),
    };
    match result {
        Ok(Some(update)) => {
            let release = Release {
                version: update.version.clone(),
                notes: update.body.clone().unwrap_or_default(),
                date: update.date.map(|d| d.to_string()),
            };
            if let Ok(mut found) = pending.found.lock() {
                *found = Some(update);
            }
            Ok(app.update(|s| s.found(release)))
        }
        Ok(None) => Ok(app.update_state()),
        Err(err) => {
            tracing::warn!(error = %err, manual, "update check failed");
            if manual {
                Err(err.to_string())
            } else {
                Ok(app.update_state())
            }
        }
    }
}

/// Why this install cannot replace itself, if it cannot.
///
/// macOS: the updater cannot replace a bundle that runs from the disk image or from the
/// read-only copy macOS makes of a quarantined app (005 research R9).
#[cfg(target_os = "macos")]
fn install_blocker() -> Option<&'static str> {
    std::env::current_exe()
        .is_ok_and(|exe| {
            let path = exe.to_string_lossy();
            path.starts_with("/Volumes/") || path.contains("/AppTranslocation/")
        })
        .then_some(WRONG_LOCATION)
}

/// Linux: only an AppImage in a folder the user can write to updates itself; a package belongs
/// to the package manager (011 research R3).
#[cfg(target_os = "linux")]
fn install_blocker() -> Option<&'static str> {
    use tauri::utils::config::BundleType;
    let install = match tauri::utils::platform::bundle_type() {
        Some(BundleType::AppImage) => LinuxInstall::AppImage {
            writable: std::env::var_os("APPIMAGE")
                .map(std::path::PathBuf::from)
                .and_then(|p| p.parent().map(writable))
                .unwrap_or(false),
        },
        _ => LinuxInstall::Package,
    };
    linux_blocker(install)
}

#[cfg(any(target_os = "linux", test))]
#[derive(Debug, Clone, Copy)]
enum LinuxInstall {
    AppImage {
        writable: bool,
    },
    /// A Debian package, or a build without a bundle.
    Package,
}

#[cfg(any(target_os = "linux", test))]
fn linux_blocker(install: LinuxInstall) -> Option<&'static str> {
    match install {
        LinuxInstall::AppImage { writable: true } => None,
        LinuxInstall::AppImage { writable: false } => Some(READ_ONLY_APPIMAGE),
        LinuxInstall::Package => Some(PACKAGE_INSTALL),
    }
}

/// Whether a new file can be created in `dir` (the updater writes the new AppImage there).
#[cfg(target_os = "linux")]
fn writable(dir: &std::path::Path) -> bool {
    let probe = dir.join(format!(".sonata-update-check-{}", std::process::id()));
    let ok = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&probe)
        .is_ok();
    let _ = std::fs::remove_file(&probe);
    ok
}

/// Checks at launch and then whenever core says an automatic check is due (FR-008).
pub fn spawn_background_checks(handle: AppHandle, core: Arc<App>) {
    tauri::async_runtime::spawn(async move {
        // Let startup finish first.
        tokio::time::sleep(Duration::from_secs(5)).await;
        loop {
            let now = Instant::now();
            if core.update_check_due(now) {
                core.mark_update_checked(now);
                let pending = handle.state::<PendingUpdate>();
                let _ = check(&handle, &core, &pending, false).await;
            }
            tokio::time::sleep(TICK).await;
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_writable_appimage_updates_itself_on_linux() {
        assert_eq!(
            linux_blocker(LinuxInstall::AppImage { writable: true }),
            None
        );
        assert_eq!(
            linux_blocker(LinuxInstall::AppImage { writable: false }),
            Some(READ_ONLY_APPIMAGE)
        );
        assert_eq!(linux_blocker(LinuxInstall::Package), Some(PACKAGE_INSTALL));
        assert!(PACKAGE_INSTALL.contains("https://github.com/DuarteVn/sonata/releases/latest"));
    }
}

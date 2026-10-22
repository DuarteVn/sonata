//! WebKit calls Tauri does not expose: page snapshots (FR-008, FR-017) and removal of a
//! workspace's browsing data (FR-010).

use std::path::PathBuf;

use tauri::{AppHandle, Webview};
use uuid::Uuid;

/// Captures the visible page as PNG into `path`, then calls `done(success)`.
pub fn capture(webview: &Webview, path: PathBuf, done: impl Fn(bool) + Send + Sync + 'static) {
    let done = std::sync::Arc::new(done);
    let fallback = done.clone();
    tracing::debug!(path = %path.display(), "snapshot requested");
    let result = webview.with_webview(move |pw| {
        tracing::debug!("snapshot: on main thread");
        #[cfg(target_os = "macos")]
        {
            use block2::RcBlock;
            use objc2_app_kit::{NSBitmapImageFileType, NSBitmapImageRep, NSImage};
            use objc2_foundation::{NSDictionary, NSError};
            use objc2_web_kit::WKWebView;

            // SAFETY: on macOS `inner()` is the WKWebView of this webview, alive during the call.
            let wk: &WKWebView = unsafe { &*(pw.inner() as *const WKWebView) };
            let done = done.clone();
            let block = RcBlock::new(move |image: *mut NSImage, err: *mut NSError| {
                tracing::debug!(
                    null_image = image.is_null(),
                    error = !err.is_null(),
                    "snapshot: completion"
                );
                let ok = (|| {
                    // SAFETY: WebKit passes a valid NSImage or null.
                    let image = unsafe { image.as_ref() }?;
                    let tiff = image.TIFFRepresentation()?;
                    let rep = NSBitmapImageRep::imageRepWithData(&tiff)?;
                    // SAFETY: an empty properties dictionary is valid for PNG output.
                    let png = unsafe {
                        rep.representationUsingType_properties(
                            NSBitmapImageFileType::PNG,
                            &NSDictionary::new(),
                        )
                    }?;
                    sonata_core::persistence::atomic::write(&path, &png.to_vec()).ok()
                })()
                .is_some();
                done(ok);
            });
            // SAFETY: a null configuration captures the visible bounds; the block lives until called.
            unsafe { wk.takeSnapshotWithConfiguration_completionHandler(None, &block) };
            tracing::debug!("snapshot: requested from WebKit");
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = (pw, path);
            done(false);
        }
    });
    if result.is_err() {
        fallback(false);
    }
}

/// Deletes the persistent data store used by a workspace's browsers (macOS 14+).
pub fn remove_data_store(app: &AppHandle, ws: Uuid) {
    #[cfg(target_os = "macos")]
    {
        let _ = app.run_on_main_thread(move || {
            use block2::RcBlock;
            use objc2::MainThreadMarker;
            use objc2_foundation::{NSError, NSUUID};
            use objc2_web_kit::WKWebsiteDataStore;

            let Some(mtm) = MainThreadMarker::new() else {
                return;
            };
            let id = NSUUID::from_bytes(*ws.as_bytes());
            let block = RcBlock::new(|_err: *mut NSError| {});
            // SAFETY: called on the main thread after the workspace's webviews were closed.
            unsafe {
                WKWebsiteDataStore::removeDataStoreForIdentifier_completionHandler(&id, &block, mtm)
            };
        });
    }
    #[cfg(not(target_os = "macos"))]
    let _ = (app, ws);
}

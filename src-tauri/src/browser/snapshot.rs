//! WebKit calls Tauri does not expose: page snapshots (FR-008, FR-017) and, on macOS, removal of
//! a workspace's browsing data (FR-010). Linux uses WebKitGTK's snapshot (011 research R10);
//! Windows uses WebView2's page capture (012 research R13).

use std::path::PathBuf;

#[cfg(target_os = "macos")]
use tauri::AppHandle;
use tauri::Webview;
#[cfg(target_os = "macos")]
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
        #[cfg(target_os = "linux")]
        {
            use webkit2gtk::{SnapshotOptions, SnapshotRegion, WebViewExt};

            let done = done.clone();
            pw.inner().snapshot(
                SnapshotRegion::Visible,
                SnapshotOptions::NONE,
                None::<&gtk::gio::Cancellable>,
                move |result| {
                    let ok = result
                        .ok()
                        .and_then(|surface| cairo::ImageSurface::try_from(surface).ok())
                        .and_then(|image| {
                            let mut png = Vec::new();
                            image.write_to_png(&mut png).ok()?;
                            Some(png)
                        })
                        .and_then(|png| sonata_core::persistence::atomic::write(&path, &png).ok())
                        .is_some();
                    tracing::debug!(ok, "snapshot: completion");
                    done(ok);
                },
            );
        }
                #[cfg(windows)]
        {
            use webview2_com::CapturePreviewCompletedHandler;
            use webview2_com::Microsoft::Web::WebView2::Win32::COREWEBVIEW2_CAPTURE_PREVIEW_IMAGE_FORMAT_PNG;
            use windows::Win32::Foundation::HGLOBAL;
            use windows::Win32::System::Com::StructuredStorage::{
                CreateStreamOnHGlobal, GetHGlobalFromStream,
            };
            use windows::Win32::System::Com::{STATFLAG_NONAME, STATSTG};
            use windows::Win32::System::Memory::{GlobalLock, GlobalSize, GlobalUnlock};

            let callback = done.clone();
            // SAFETY: the controller and its WebView2 are alive during this main-thread call;
            // the stream is owned by the handler until it runs.
            let started = unsafe {
                (|| -> windows::core::Result<()> {
                    let core = pw.controller().CoreWebView2()?;
                    let stream = CreateStreamOnHGlobal(HGLOBAL::default(), true)?;
                    let reader = stream.clone();
                    let handler = CapturePreviewCompletedHandler::create(Box::new(move |result| {
                        let png = result.ok().and_then(|_| {
                            let mut stat = STATSTG::default();
                            reader.Stat(&mut stat, STATFLAG_NONAME).ok()?;
                            let hglobal = GetHGlobalFromStream(&reader).ok()?;
                            let len = (stat.cbSize as usize).min(GlobalSize(hglobal));
                            let ptr = GlobalLock(hglobal) as *const u8;
                            if ptr.is_null() || len == 0 {
                                return None;
                            }
                            let bytes = std::slice::from_raw_parts(ptr, len).to_vec();
                            let _ = GlobalUnlock(hglobal);
                            Some(bytes)
                        });
                        let ok = png.is_some_and(|png| {
                            sonata_core::persistence::atomic::write(&path, &png).is_ok()
                        });
                        callback(ok);
                        Ok(())
                    }));
                    core.CapturePreview(
                        COREWEBVIEW2_CAPTURE_PREVIEW_IMAGE_FORMAT_PNG,
                        &stream,
                        &handler,
                    )
                })()
            };
            if let Err(err) = started {
                tracing::debug!(error = %err, "snapshot: capture failed to start");
                done(false);
            }
        }
        #[cfg(not(any(target_os = "macos", target_os = "linux", windows)))]
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
#[cfg(target_os = "macos")]
pub fn remove_data_store(app: &AppHandle, ws: Uuid) {
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

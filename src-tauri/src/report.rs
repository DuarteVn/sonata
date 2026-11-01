//! Report Bug commands (feature 013, contracts/app-commands.md). Thin wrappers over
//! `sonata_core::report`; the service URL and the fallback alias are fixed at build time.

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use sonata_core::App;
use sonata_core::platform;
use sonata_core::report::{self, BugReport, ReportError, ReportImage, diagnostics, mailto};
use tauri::{AppHandle, State};

/// Build-time configuration (research R5). An empty value counts as absent.
pub struct ReportConfig {
    pub service_url: Option<&'static str>,
    pub fallback_email: Option<&'static str>,
}

fn non_empty(value: Option<&'static str>) -> Option<&'static str> {
    value.map(str::trim).filter(|v| !v.is_empty())
}

pub fn config() -> ReportConfig {
    ReportConfig {
        service_url: non_empty(option_env!("SONATA_REPORT_URL")),
        fallback_email: non_empty(option_env!("SONATA_REPORT_FALLBACK_EMAIL")),
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportConfigView {
    service_configured: bool,
    fallback_email: Option<&'static str>,
}

/// An image as the UI holds it. The declared type is ignored; core reads the magic number.
#[derive(Debug, Serialize, Deserialize)]
pub struct ImageView {
    bytes: Vec<u8>,
    mime: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportInput {
    name: String,
    description: String,
    image: Option<ImageView>,
    #[serde(default)]
    include_logs: bool,
}

impl ReportInput {
    fn into_report(self) -> Result<BugReport, ReportError> {
        let image = match self.image {
            Some(image) => Some(ReportImage::from_bytes(image.bytes)?),
            None => None,
        };
        Ok(BugReport {
            name: self.name,
            description: self.description,
            image,
            include_logs: self.include_logs,
        })
    }
}

fn app_version(handle: &AppHandle) -> String {
    handle.package_info().version.to_string()
}

#[tauri::command]
pub fn report_config() -> ReportConfigView {
    let config = config();
    ReportConfigView {
        service_configured: config.service_url.is_some(),
        fallback_email: config.fallback_email,
    }
}

/// Reads a dropped or chosen image; core checks its size and type.
#[tauri::command]
pub async fn report_read_image(path: std::path::PathBuf) -> Result<ImageView, ReportError> {
    let image = tauri::async_runtime::spawn_blocking(move || report::read_image(&path))
        .await
        .map_err(|_| ReportError::Invalid {
            field: "image",
            reason: "unreadable",
        })??;
    Ok(ImageView {
        mime: image.mime.as_str().to_owned(),
        bytes: image.bytes,
    })
}

#[tauri::command]
pub async fn report_send(
    handle: AppHandle,
    core: State<'_, Arc<App>>,
    input: ReportInput,
) -> Result<(), ReportError> {
    let report = input.into_report()?.validate()?;
    let diagnostics = diagnostics::collect_now(&app_version(&handle));
    // Logs are read only on the user's explicit opt-in (FR-008).
    let log = if report.include_logs {
        let dir = core.logs_dir();
        tauri::async_runtime::spawn_blocking(move || report::log_tail::read(&dir))
            .await
            .ok()
            .flatten()
    } else {
        None
    };
    report::send(
        config().service_url,
        report,
        &diagnostics,
        log,
        report::SEND_TIMEOUT,
    )
    .await
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FallbackResult {
    opened: bool,
    text: String,
    truncated: bool,
}

/// Opens a `mailto:` to the public alias with the report filled in, and always returns the
/// full text so the UI can copy it when the link was not opened or was cut.
#[tauri::command]
pub fn report_fallback(
    handle: AppHandle,
    input: ReportInput,
) -> Result<FallbackResult, ReportError> {
    let report = input.into_report()?.validate()?;
    let diagnostics = diagnostics::collect_now(&app_version(&handle));
    // A mail link cannot carry files, so ticked logs are reported as unavailable.
    let logs = if report.include_logs {
        mailto::LogsState::RequestedButUnavailable
    } else {
        mailto::LogsState::NotRequested
    };
    let Some(alias) = config().fallback_email else {
        return Ok(FallbackResult {
            opened: false,
            text: mailto::plain_text(&report, &diagnostics, logs),
            truncated: false,
        });
    };
    let link = mailto::build(alias, &report, &diagnostics, logs);
    let opened = match platform::open_url_command(&link.url).spawn() {
        Ok(()) => true,
        Err(err) => {
            tracing::warn!(error = %err, "could not open the mail app");
            false
        }
    };
    Ok(FallbackResult {
        opened,
        text: link.text,
        truncated: link.truncated,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_build_values_count_as_absent() {
        assert_eq!(non_empty(None), None);
        assert_eq!(non_empty(Some("")), None);
        assert_eq!(non_empty(Some("  ")), None);
        assert_eq!(
            non_empty(Some("https://r.example/report")),
            Some("https://r.example/report")
        );
    }

    #[test]
    fn input_image_type_comes_from_the_bytes() {
        let input: ReportInput = serde_json::from_value(serde_json::json!({
            "name": "Ana",
            "description": "x",
            "image": { "bytes": [255, 216, 255, 0], "mime": "image/png" },
            "includeLogs": false
        }))
        .unwrap();
        let report = input.into_report().unwrap();
        assert_eq!(
            report.image.unwrap().mime,
            sonata_core::report::ImageMime::Jpeg
        );
    }
}

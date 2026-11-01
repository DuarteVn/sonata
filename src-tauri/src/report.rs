//! Report Bug commands (feature 013, contracts/app-commands.md). Thin wrappers over
//! `sonata_core::report`; the service URL and the fallback alias are fixed at build time.

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use sonata_core::App;
use sonata_core::report::{self, BugReport, ReportError, ReportImage, diagnostics};
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

#[tauri::command]
pub async fn report_send(
    handle: AppHandle,
    _core: State<'_, Arc<App>>,
    input: ReportInput,
) -> Result<(), ReportError> {
    let report = input.into_report()?.validate()?;
    let diagnostics = diagnostics::collect_now(&app_version(&handle));
    report::send(
        config().service_url,
        report,
        &diagnostics,
        None,
        report::SEND_TIMEOUT,
    )
    .await
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

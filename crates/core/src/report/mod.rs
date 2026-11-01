//! Bug reports (feature 013): validation, diagnostics, the log excerpt, and sending to the
//! report service. The UI never talks to the network; the shell passes user input here.

pub mod diagnostics;

use std::time::Duration;

use reqwest::multipart::{Form, Part};
use serde::{Deserialize, Serialize};

use diagnostics::Diagnostics;

/// Longest name, in Unicode scalar values, after trimming.
pub const MAX_NAME: usize = 100;
/// Longest description, in Unicode scalar values, after trimming.
pub const MAX_DESCRIPTION: usize = 10_000;
/// Largest image, in bytes.
pub const MAX_IMAGE: usize = 5 * 1024 * 1024;
/// Largest log excerpt, in bytes.
pub const MAX_LOG: usize = 1024 * 1024;
/// Total time allowed for one send.
pub const SEND_TIMEOUT: Duration = Duration::from_secs(30);

/// The image types the service accepts, decided from the first bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImageMime {
    #[serde(rename = "image/png")]
    Png,
    #[serde(rename = "image/jpeg")]
    Jpeg,
}

impl ImageMime {
    pub fn as_str(self) -> &'static str {
        match self {
            ImageMime::Png => "image/png",
            ImageMime::Jpeg => "image/jpeg",
        }
    }

    /// The file name used on the wire.
    pub fn file_name(self) -> &'static str {
        match self {
            ImageMime::Png => "screenshot.png",
            ImageMime::Jpeg => "screenshot.jpg",
        }
    }
}

/// One attached image. Its type always comes from [`sniff_image`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReportImage {
    pub bytes: Vec<u8>,
    pub mime: ImageMime,
}

impl ReportImage {
    /// Checks size and magic number; the declared type of the source is never trusted.
    pub fn from_bytes(bytes: Vec<u8>) -> Result<Self, ReportError> {
        if bytes.len() > MAX_IMAGE {
            return Err(ReportError::invalid("image", "too_large"));
        }
        let mime = sniff_image(&bytes)?;
        Ok(ReportImage { bytes, mime })
    }
}

/// Reads a dropped or chosen image file: size from metadata first, then at most
/// [`MAX_IMAGE`] + 1 bytes, then the magic number.
pub fn read_image(path: &std::path::Path) -> Result<ReportImage, ReportError> {
    use std::io::Read;

    let unreadable = |_| ReportError::invalid("image", "unreadable");
    let file = std::fs::File::open(path).map_err(unreadable)?;
    let meta = file.metadata().map_err(unreadable)?;
    if !meta.is_file() {
        return Err(ReportError::invalid("image", "unreadable"));
    }
    if meta.len() > MAX_IMAGE as u64 {
        return Err(ReportError::invalid("image", "too_large"));
    }
    // The file may grow after the metadata check; never read more than one byte past the limit.
    let mut bytes = Vec::with_capacity(meta.len() as usize);
    file.take(MAX_IMAGE as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(unreadable)?;
    ReportImage::from_bytes(bytes)
}

/// What the user typed and attached.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct BugReport {
    pub name: String,
    pub description: String,
    pub image: Option<ReportImage>,
    pub include_logs: bool,
}

/// The end of the newest log file, sent only when the user ticks "Include logs".
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogExcerpt {
    pub text: String,
    /// The file it came from, for example `sonata.log.2026-11-01`.
    pub source: String,
}

/// Why a report was not sent. Serialized for the UI as `{"kind": …}` per
/// `contracts/app-commands.md`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, thiserror::Error)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub enum ReportError {
    #[error("invalid {field}: {reason}")]
    Invalid {
        field: &'static str,
        reason: &'static str,
    },
    #[error("this build has no report service")]
    NotConfigured,
    #[error("could not reach the report service")]
    Unreachable,
    #[error("too many reports; retry in {retry_after_secs} s")]
    RateLimited { retry_after_secs: u64 },
    #[error("the report service refused the report: {code}")]
    Refused { code: String },
}

impl ReportError {
    fn invalid(field: &'static str, reason: &'static str) -> Self {
        ReportError::Invalid { field, reason }
    }
}

/// Decides the image type from its first bytes: PNG `89 50 4E 47 0D 0A 1A 0A`, JPEG `FF D8 FF`.
pub fn sniff_image(bytes: &[u8]) -> Result<ImageMime, ReportError> {
    const PNG: &[u8] = &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
    const JPEG: &[u8] = &[0xFF, 0xD8, 0xFF];
    if bytes.starts_with(PNG) {
        Ok(ImageMime::Png)
    } else if bytes.starts_with(JPEG) {
        Ok(ImageMime::Jpeg)
    } else {
        Err(ReportError::invalid("image", "unsupported_type"))
    }
}

/// Trims `value` and checks it holds 1 to `max` Unicode scalar values.
fn text(value: &str, field: &'static str, max: usize) -> Result<String, ReportError> {
    let trimmed = value.trim();
    match trimmed.chars().count() {
        0 => Err(ReportError::invalid(field, "empty")),
        n if n > max => Err(ReportError::invalid(field, "too_long")),
        _ => Ok(trimmed.to_owned()),
    }
}

impl BugReport {
    /// Returns the report with name and description trimmed, or the first rule it breaks.
    pub fn validate(self) -> Result<BugReport, ReportError> {
        let name = text(&self.name, "name", MAX_NAME)?;
        let description = text(&self.description, "description", MAX_DESCRIPTION)?;
        let image = match self.image {
            Some(image) => Some(ReportImage::from_bytes(image.bytes)?),
            None => None,
        };
        Ok(BugReport {
            name,
            description,
            image,
            include_logs: self.include_logs,
        })
    }
}

/// When a 429 carries no usable `Retry-After`.
const DEFAULT_RETRY_AFTER_SECS: u64 = 3600;

/// Validates `report` and posts it with `diagnostics` (and `log`, only when the user asked for
/// logs) to the report service at `url` as `multipart/form-data`, per
/// `contracts/report-service.md`. Nothing connects when the report is invalid or `url` is
/// `None`.
pub async fn send(
    url: Option<&str>,
    report: BugReport,
    diagnostics: &Diagnostics,
    log: Option<LogExcerpt>,
    timeout: Duration,
) -> Result<(), ReportError> {
    let report = report.validate()?;
    let url = url
        .filter(|u| !u.trim().is_empty())
        .ok_or(ReportError::NotConfigured)?;
    let log = log.filter(|_| report.include_logs);
    let form = form(report, diagnostics, log)?;

    install_crypto_provider();
    let client = reqwest::Client::builder()
        .user_agent(format!("Sonata/{}", diagnostics.app_version))
        .timeout(timeout)
        .build()
        .map_err(|err| {
            tracing::warn!(error = %err, "could not build the report client");
            ReportError::Unreachable
        })?;
    let response = client
        .post(url)
        .multipart(form)
        .send()
        .await
        .map_err(|err| {
            tracing::info!(error = %err, "report service unreachable");
            ReportError::Unreachable
        })?;
    let status = response.status();
    tracing::info!(status = status.as_u16(), "report service answered");
    if status.is_success() {
        return Ok(());
    }
    if status.as_u16() == 429 {
        let retry_after_secs = response
            .headers()
            .get(reqwest::header::RETRY_AFTER)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.trim().parse().ok())
            .unwrap_or(DEFAULT_RETRY_AFTER_SECS);
        return Err(ReportError::RateLimited { retry_after_secs });
    }
    if status.is_client_error() {
        #[derive(Deserialize)]
        struct Body {
            error: Option<String>,
        }
        let code = response
            .bytes()
            .await
            .ok()
            .and_then(|b| serde_json::from_slice::<Body>(&b).ok())
            .and_then(|b| b.error)
            .unwrap_or_else(|| format!("http_{}", status.as_u16()));
        return Err(ReportError::Refused { code });
    }
    Err(ReportError::Unreachable)
}

fn form(report: BugReport, d: &Diagnostics, log: Option<LogExcerpt>) -> Result<Form, ReportError> {
    let mut form = Form::new()
        .text("name", report.name)
        .text("description", report.description)
        .text("appVersion", d.app_version.clone())
        .text("os", d.os.clone())
        .text("osVersion", d.os_version.clone())
        .text("arch", d.arch.clone())
        .text("sentAt", d.sent_at.clone())
        .text("logsRequested", report.include_logs.to_string());
    let part = |bytes: Vec<u8>, file: &'static str, mime: &str| {
        Part::bytes(bytes)
            .file_name(file)
            .mime_str(mime)
            .map_err(|_| ReportError::invalid("image", "unsupported_type"))
    };
    if let Some(image) = report.image {
        let mime = image.mime;
        form = form.part("image", part(image.bytes, mime.file_name(), mime.as_str())?);
    }
    if let Some(log) = log {
        let mut text = log.text.into_bytes();
        text.truncate(MAX_LOG);
        form = form.part("log", part(text, "sonata-log.txt", "text/plain")?);
    }
    Ok(form)
}

/// `reqwest` is built without a bundled crypto provider; use `ring`, as the updater does.
fn install_crypto_provider() {
    if rustls::crypto::CryptoProvider::get_default().is_none() {
        let _ = rustls::crypto::ring::default_provider().install_default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PNG: &[u8] = &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0, 0];
    const JPEG: &[u8] = &[0xFF, 0xD8, 0xFF, 0xE0, 0, 0];
    const GIF: &[u8] = b"GIF89a\0\0";

    fn report(name: &str, description: &str) -> BugReport {
        BugReport {
            name: name.into(),
            description: description.into(),
            ..Default::default()
        }
    }

    fn invalid(field: &'static str, reason: &'static str) -> Result<BugReport, ReportError> {
        Err(ReportError::Invalid { field, reason })
    }

    #[test]
    fn trims_name_and_description() {
        let ok = report("  Ana ", "\n broke \t").validate().unwrap();
        assert_eq!(ok.name, "Ana");
        assert_eq!(ok.description, "broke");
    }

    #[test]
    fn name_is_required_and_limited() {
        assert_eq!(report("   ", "x").validate(), invalid("name", "empty"));
        assert!(report(&"a".repeat(MAX_NAME), "x").validate().is_ok());
        assert_eq!(
            report(&"a".repeat(MAX_NAME + 1), "x").validate(),
            invalid("name", "too_long")
        );
    }

    #[test]
    fn limits_count_unicode_scalar_values() {
        // 100 emoji are 400 bytes but 100 characters.
        let emoji = "😀".repeat(MAX_NAME);
        assert_eq!(report(&emoji, "x").validate().unwrap().name, emoji);
        assert_eq!(
            report("José 山田 🐛", "x").validate().unwrap().name,
            "José 山田 🐛"
        );
        let long = "é".repeat(MAX_DESCRIPTION);
        assert!(report("a", &long).validate().is_ok());
    }

    #[test]
    fn description_is_required_and_limited() {
        assert_eq!(
            report("a", " \n ").validate(),
            invalid("description", "empty")
        );
        assert_eq!(
            report("a", &"d".repeat(MAX_DESCRIPTION + 1)).validate(),
            invalid("description", "too_long")
        );
    }

    #[test]
    fn sniffs_png_and_jpeg_and_rejects_others() {
        assert_eq!(sniff_image(PNG), Ok(ImageMime::Png));
        assert_eq!(sniff_image(JPEG), Ok(ImageMime::Jpeg));
        let unsupported = Err(ReportError::invalid("image", "unsupported_type"));
        assert_eq!(sniff_image(GIF), unsupported);
        assert_eq!(sniff_image(&PNG[..4]), unsupported);
        assert_eq!(sniff_image(&[]), unsupported);
    }

    #[test]
    fn image_type_comes_from_bytes_and_size_is_limited() {
        assert_eq!(
            ReportImage::from_bytes(JPEG.to_vec()).unwrap().mime,
            ImageMime::Jpeg
        );
        assert_eq!(
            ReportImage::from_bytes(GIF.to_vec()),
            Err(ReportError::invalid("image", "unsupported_type"))
        );
        let mut big = PNG.to_vec();
        big.resize(MAX_IMAGE, 0);
        assert!(ReportImage::from_bytes(big.clone()).is_ok());
        big.push(0);
        assert_eq!(
            ReportImage::from_bytes(big),
            Err(ReportError::invalid("image", "too_large"))
        );
    }

    #[test]
    fn validate_re_checks_the_image() {
        let mut r = report("a", "b");
        // A GIF that claims to be a PNG.
        r.image = Some(ReportImage {
            bytes: GIF.to_vec(),
            mime: ImageMime::Png,
        });
        assert_eq!(r.validate(), invalid("image", "unsupported_type"));
        let mut r = report("a", "b");
        r.image = Some(ReportImage {
            bytes: JPEG.to_vec(),
            mime: ImageMime::Png,
        });
        assert_eq!(r.validate().unwrap().image.unwrap().mime, ImageMime::Jpeg);
    }

    #[test]
    fn reads_image_files() {
        let dir = tempfile::tempdir().unwrap();
        let write = |name: &str, bytes: &[u8]| {
            let path = dir.path().join(name);
            std::fs::write(&path, bytes).unwrap();
            path
        };
        // The file name never decides the type.
        let png = read_image(&write("shot.jpg", PNG)).unwrap();
        assert_eq!((png.mime, png.bytes.as_slice()), (ImageMime::Png, PNG));
        assert_eq!(
            read_image(&write("photo.jpeg", JPEG)).unwrap().mime,
            ImageMime::Jpeg
        );
        assert_eq!(
            read_image(&write("anim.png", GIF)),
            Err(ReportError::invalid("image", "unsupported_type"))
        );
        let mut big = PNG.to_vec();
        big.resize(MAX_IMAGE + 1, 0);
        assert_eq!(
            read_image(&write("big.png", &big)),
            Err(ReportError::invalid("image", "too_large"))
        );
        big.truncate(MAX_IMAGE);
        assert!(read_image(&write("edge.png", &big)).is_ok());
        assert_eq!(
            read_image(&dir.path().join("missing.png")),
            Err(ReportError::invalid("image", "unreadable"))
        );
        assert_eq!(
            read_image(dir.path()),
            Err(ReportError::invalid("image", "unreadable"))
        );
    }

    #[test]
    fn errors_serialize_for_the_ui() {
        let json = |e: ReportError| serde_json::to_value(e).unwrap();
        assert_eq!(
            json(ReportError::invalid("name", "empty")),
            serde_json::json!({"kind": "invalid", "field": "name", "reason": "empty"})
        );
        assert_eq!(
            json(ReportError::RateLimited {
                retry_after_secs: 120
            }),
            serde_json::json!({"kind": "rate_limited", "retryAfterSecs": 120})
        );
        assert_eq!(
            json(ReportError::NotConfigured),
            serde_json::json!({"kind": "not_configured"})
        );
        assert_eq!(
            json(ReportError::Refused { code: "x".into() }),
            serde_json::json!({"kind": "refused", "code": "x"})
        );
    }
}

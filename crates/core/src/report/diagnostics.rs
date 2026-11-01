//! What the app adds to every report (research R6). The user never types these.

use chrono::{DateTime, SecondsFormat, Utc};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostics {
    pub app_version: String,
    /// `macos`, `linux`, or `windows`.
    pub os: String,
    pub os_version: String,
    /// `aarch64` or `x86_64`.
    pub arch: String,
    /// RFC 3339 UTC time.
    pub sent_at: String,
}

/// Diagnostics of this computer, for a report sent at `now`.
pub fn collect(app_version: &str, now: DateTime<Utc>) -> Diagnostics {
    Diagnostics {
        app_version: app_version.to_owned(),
        os: std::env::consts::OS.to_owned(),
        os_version: crate::platform::os_version(),
        arch: std::env::consts::ARCH.to_owned(),
        sent_at: now.to_rfc3339_opts(SecondsFormat::Secs, true),
    }
}

/// [`collect`] at the current time.
pub fn collect_now(app_version: &str) -> Diagnostics {
    collect(app_version, Utc::now())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn collects_this_computer() {
        let now = Utc.with_ymd_and_hms(2026, 11, 1, 12, 30, 5).unwrap();
        let d = collect("0.3.0", now);
        assert_eq!(d.app_version, "0.3.0");
        assert!(["macos", "linux", "windows"].contains(&d.os.as_str()));
        assert_eq!(d.os, std::env::consts::OS);
        assert_eq!(d.arch, std::env::consts::ARCH);
        assert!(!d.os_version.is_empty());
        assert_eq!(d.sent_at, "2026-11-01T12:30:05Z");
        assert!(DateTime::parse_from_rfc3339(&d.sent_at).is_ok());
    }
}

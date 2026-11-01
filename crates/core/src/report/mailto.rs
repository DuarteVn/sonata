//! The fallback when the report service cannot be used (research R9): the plain-text report,
//! the same body the service emails, and a `mailto:` link to the public alias.

use super::BugReport;
use super::diagnostics::Diagnostics;

/// Longest `mailto:` body; longer URLs break some Windows mail handlers.
pub const MAX_BODY: usize = 1_500;
pub const CUT_MARKER: &str = "(cut, full text copied to clipboard)";
const SUBJECT_WORDS: usize = 60;

/// What the email body says about logs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogsState {
    Included,
    RequestedButUnavailable,
    NotRequested,
}

impl LogsState {
    pub fn as_str(self) -> &'static str {
        match self {
            LogsState::Included => "included",
            LogsState::RequestedButUnavailable => "requested but unavailable",
            LogsState::NotRequested => "not requested",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mailto {
    pub url: String,
    /// The whole report, for the clipboard.
    pub text: String,
    /// Whether the body in `url` was cut.
    pub truncated: bool,
}

/// The report as the service's email body (contracts/report-service.md).
pub fn plain_text(report: &BugReport, d: &Diagnostics, logs: LogsState) -> String {
    format!(
        "From: {}\nSent: {}\nSonata {} · {} {} · {}\nLogs: {}\n\n{}",
        report.name,
        d.sent_at,
        d.app_version,
        d.os,
        d.os_version,
        d.arch,
        logs.as_str(),
        report.description
    )
}

/// A `mailto:` link to `alias` with subject and (possibly cut) body.
pub fn build(alias: &str, report: &BugReport, d: &Diagnostics, logs: LogsState) -> Mailto {
    let text = plain_text(report, d, logs);
    let subject = format!(
        "Sonata {} bug: {}",
        d.app_version,
        first_words(&report.description)
    );
    let (body, truncated) = if text.chars().count() > MAX_BODY {
        let keep = MAX_BODY - CUT_MARKER.chars().count() - 1;
        let head: String = text.chars().take(keep).collect();
        (format!("{head}\n{CUT_MARKER}"), true)
    } else {
        (text.clone(), false)
    };
    // RFC 6068: line breaks in a mailto body are CRLF.
    let body = body.replace('\n', "\r\n");
    Mailto {
        url: format!(
            "mailto:{alias}?subject={}&body={}",
            encode(&subject),
            encode(&body)
        ),
        text,
        truncated,
    }
}

/// The description on one line, cut between words to at most [`SUBJECT_WORDS`] characters.
fn first_words(description: &str) -> String {
    let line = description.split_whitespace().collect::<Vec<_>>().join(" ");
    if line.chars().count() <= SUBJECT_WORDS {
        return line;
    }
    let head: String = line.chars().take(SUBJECT_WORDS + 1).collect();
    match head.rfind(' ') {
        Some(space) if space > 0 => head[..space].to_owned(),
        _ => head.chars().take(SUBJECT_WORDS).collect(),
    }
}

/// RFC 3986 percent-encoding: only unreserved characters stay as they are.
fn encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len() * 3);
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'.' | b'_' | b'~') {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn diagnostics() -> Diagnostics {
        Diagnostics {
            app_version: "0.3.0".into(),
            os: "linux".into(),
            os_version: "Ubuntu 24.04.1 LTS (6.8.0-45-generic)".into(),
            arch: "x86_64".into(),
            sent_at: "2026-11-01T12:00:00Z".into(),
        }
    }

    fn report(description: &str) -> BugReport {
        BugReport {
            name: "Ana".into(),
            description: description.into(),
            ..Default::default()
        }
    }

    /// Decodes RFC 3986 percent-encoding, for checking round trips.
    fn decode(s: &str) -> String {
        let bytes = s.as_bytes();
        let mut out = Vec::new();
        let mut i = 0;
        while i < bytes.len() {
            if bytes[i] == b'%' {
                out.push(u8::from_str_radix(&s[i + 1..i + 3], 16).unwrap());
                i += 3;
            } else {
                out.push(bytes[i]);
                i += 1;
            }
        }
        String::from_utf8(out).unwrap()
    }

    fn query<'a>(url: &'a str, key: &str) -> &'a str {
        let q = url.split_once('?').unwrap().1;
        q.split('&')
            .find_map(|kv| kv.strip_prefix(&format!("{key}=")))
            .unwrap()
    }

    #[test]
    fn plain_text_matches_the_email_body() {
        let text = plain_text(
            &report("It broke.\nTwice."),
            &diagnostics(),
            LogsState::NotRequested,
        );
        assert_eq!(
            text,
            "From: Ana\nSent: 2026-11-01T12:00:00Z\n\
             Sonata 0.3.0 · linux Ubuntu 24.04.1 LTS (6.8.0-45-generic) · x86_64\n\
             Logs: not requested\n\nIt broke.\nTwice."
        );
        assert!(
            plain_text(
                &report("x"),
                &diagnostics(),
                LogsState::RequestedButUnavailable
            )
            .contains("\nLogs: requested but unavailable\n")
        );
    }

    #[test]
    fn builds_an_encoded_mailto() {
        let m = build(
            "bugs@example.com",
            &report("Drag & drop? #1 + more ção\nnext"),
            &diagnostics(),
            LogsState::NotRequested,
        );
        assert!(m.url.starts_with("mailto:bugs@example.com?subject="));
        assert!(!m.truncated);
        let subject = query(&m.url, "subject");
        let body = query(&m.url, "body");
        for raw in [subject, body] {
            assert!(
                raw.bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"-._~%".contains(&b)),
                "only unreserved characters and escapes: {raw}"
            );
        }
        assert!(body.contains("%20"));
        assert!(body.contains("%0D%0A"));
        assert!(!body.contains('+'));
        for (c, enc) in [
            ("&", "%26"),
            ("?", "%3F"),
            ("#", "%23"),
            ("+", "%2B"),
            ("ç", "%C3%A7"),
        ] {
            assert!(body.contains(enc), "{c} encoded as {enc}");
        }
        assert_eq!(decode(body), m.text.replace('\n', "\r\n"));
        assert_eq!(
            decode(subject),
            "Sonata 0.3.0 bug: Drag & drop? #1 + more ção next"
        );
    }

    #[test]
    fn subject_uses_the_first_words_only() {
        let long = "word ".repeat(40);
        let m = build(
            "a@b.c",
            &report(&long),
            &diagnostics(),
            LogsState::NotRequested,
        );
        let subject = decode(query(&m.url, "subject"));
        let summary = subject.strip_prefix("Sonata 0.3.0 bug: ").unwrap();
        assert!(summary.chars().count() <= SUBJECT_WORDS);
        assert!(summary.ends_with("word"), "cut between words: {summary:?}");
    }

    #[test]
    fn long_body_is_cut_with_a_marker() {
        let description = "é".repeat(5_000);
        let m = build(
            "a@b.c",
            &report(&description),
            &diagnostics(),
            LogsState::NotRequested,
        );
        assert!(m.truncated);
        assert!(
            m.text.ends_with(&description),
            "full text kept for the clipboard"
        );
        let body = decode(query(&m.url, "body")).replace("\r\n", "\n");
        assert!(body.chars().count() <= MAX_BODY);
        assert!(body.ends_with(CUT_MARKER));
        assert!(body.starts_with("From: Ana\n"));
    }
}

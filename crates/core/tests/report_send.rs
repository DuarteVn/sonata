//! `report::send` against a local HTTP stub started by each test (feature 013). The real report
//! service is never called.

use std::time::Duration;

use sonata_core::report::diagnostics::Diagnostics;
use sonata_core::report::{BugReport, ImageMime, LogExcerpt, ReportError, ReportImage, send};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::oneshot;

const PNG: &[u8] = &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 1, 2, 3];

fn diagnostics() -> Diagnostics {
    Diagnostics {
        app_version: "0.3.0".into(),
        os: "macos".into(),
        os_version: "15.6.1".into(),
        arch: "aarch64".into(),
        sent_at: "2026-11-01T12:00:00Z".into(),
    }
}

fn report() -> BugReport {
    BugReport {
        name: "Ana".into(),
        description: "The note vanished.".into(),
        image: Some(ReportImage {
            bytes: PNG.to_vec(),
            mime: ImageMime::Png,
        }),
        include_logs: false,
    }
}

/// A one-shot HTTP server: answers the first request with `response` (or never, when `None`)
/// and hands back the raw request it received.
async fn stub(response: Option<String>) -> (String, oneshot::Receiver<Vec<u8>>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/report", listener.local_addr().unwrap());
    let (tx, rx) = oneshot::channel();
    tokio::spawn(async move {
        let (mut sock, _) = listener.accept().await.unwrap();
        let request = read_request(&mut sock).await;
        let _ = tx.send(request);
        match response {
            Some(r) => {
                sock.write_all(r.as_bytes()).await.unwrap();
                sock.shutdown().await.ok();
            }
            None => tokio::time::sleep(Duration::from_secs(60)).await,
        }
    });
    (url, rx)
}

async fn read_request(sock: &mut tokio::net::TcpStream) -> Vec<u8> {
    let mut buf = Vec::new();
    let mut chunk = [0u8; 8192];
    loop {
        let n = sock.read(&mut chunk).await.unwrap();
        if n == 0 {
            return buf;
        }
        buf.extend_from_slice(&chunk[..n]);
        if let Some(end) = find(&buf, b"\r\n\r\n") {
            let head = String::from_utf8_lossy(&buf[..end]).to_lowercase();
            let len = head
                .lines()
                .find_map(|l| l.strip_prefix("content-length:"))
                .map(|v| v.trim().parse::<usize>().unwrap())
                .unwrap_or(0);
            if buf.len() >= end + 4 + len {
                return buf;
            }
        }
    }
}

fn find(hay: &[u8], needle: &[u8]) -> Option<usize> {
    hay.windows(needle.len()).position(|w| w == needle)
}

fn text_part(request: &str, name: &str, value: &str) -> bool {
    request.contains(&format!(
        "Content-Disposition: form-data; name=\"{name}\"\r\n\r\n{value}\r\n"
    ))
}

/// An HTTP/1.1 response with a JSON body and extra header lines.
fn http(status: &str, extra: &str, body: &str) -> Option<String> {
    Some(format!(
        "HTTP/1.1 {status}\r\n{extra}Content-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    ))
}

fn ok() -> Option<String> {
    http("200 OK", "", r#"{"ok":true}"#)
}

#[tokio::test]
async fn sends_every_part_and_succeeds_on_200() {
    let (url, rx) = stub(ok()).await;
    let result = send(
        Some(&url),
        report(),
        &diagnostics(),
        None,
        Duration::from_secs(5),
    )
    .await;
    assert_eq!(result, Ok(()));
    let raw = rx.await.unwrap();
    let request = String::from_utf8_lossy(&raw);
    assert!(request.starts_with("POST /report "), "{request}");
    assert!(
        request
            .to_lowercase()
            .contains("user-agent: sonata/0.3.0\r\n")
    );
    assert!(
        request
            .to_lowercase()
            .contains("content-type: multipart/form-data; boundary=")
    );
    for (name, value) in [
        ("name", "Ana"),
        ("description", "The note vanished."),
        ("appVersion", "0.3.0"),
        ("os", "macos"),
        ("osVersion", "15.6.1"),
        ("arch", "aarch64"),
        ("sentAt", "2026-11-01T12:00:00Z"),
        ("logsRequested", "false"),
    ] {
        assert!(text_part(&request, name, value), "missing part {name}");
    }
    assert!(request.contains(
        "Content-Disposition: form-data; name=\"image\"; filename=\"screenshot.png\"\r\nContent-Type: image/png\r\n\r\n"
    ));
    assert!(find(&raw, PNG).is_some(), "image bytes sent unchanged");
    assert!(!request.contains("name=\"log\""));
}

#[tokio::test]
async fn sends_the_log_only_when_requested() {
    let log = || LogExcerpt {
        text: "line one\nline two\n".into(),
        source: "sonata.log.2026-11-01".into(),
    };
    // Ticked with a log.
    let (url, rx) = stub(ok()).await;
    let mut r = report();
    r.include_logs = true;
    send(
        Some(&url),
        r,
        &diagnostics(),
        Some(log()),
        Duration::from_secs(5),
    )
    .await
    .unwrap();
    let raw = rx.await.unwrap();
    let request = String::from_utf8_lossy(&raw);
    assert!(text_part(&request, "logsRequested", "true"));
    assert!(request.contains(
        "name=\"log\"; filename=\"sonata-log.txt\"\r\nContent-Type: text/plain\r\n\r\nline one\nline two\n\r\n"
    ));

    // Ticked without a log.
    let (url, rx) = stub(ok()).await;
    let mut r = report();
    r.image = None;
    r.include_logs = true;
    send(Some(&url), r, &diagnostics(), None, Duration::from_secs(5))
        .await
        .unwrap();
    let request = String::from_utf8(rx.await.unwrap()).unwrap();
    assert!(text_part(&request, "logsRequested", "true"));
    assert!(!request.contains("name=\"log\""));

    // Unticked: a log handed in by mistake is still not sent.
    let (url, rx) = stub(ok()).await;
    let mut r = report();
    r.image = None;
    send(
        Some(&url),
        r,
        &diagnostics(),
        Some(log()),
        Duration::from_secs(5),
    )
    .await
    .unwrap();
    let request = String::from_utf8(rx.await.unwrap()).unwrap();
    assert!(text_part(&request, "logsRequested", "false"));
    assert!(!request.contains("name=\"log\""));
}

async fn answer(response: Option<String>) -> Result<(), ReportError> {
    let (url, _rx) = stub(response).await;
    send(
        Some(&url),
        report(),
        &diagnostics(),
        None,
        Duration::from_secs(5),
    )
    .await
}

#[tokio::test]
async fn maps_429_to_rate_limited_with_retry_after() {
    let r = answer(http(
        "429 Too Many Requests",
        "Retry-After: 120\r\n",
        r#"{"ok":false,"error":"address_limit"}"#,
    ))
    .await;
    assert_eq!(
        r,
        Err(ReportError::RateLimited {
            retry_after_secs: 120
        })
    );
}

#[tokio::test]
async fn maps_400_to_refused_with_the_code() {
    let r = answer(http(
        "400 Bad Request",
        "",
        r#"{"ok":false,"error":"invalid_field:name"}"#,
    ))
    .await;
    assert_eq!(
        r,
        Err(ReportError::Refused {
            code: "invalid_field:name".into()
        })
    );
}

#[tokio::test]
async fn maps_5xx_to_unreachable() {
    let r = answer(http(
        "502 Bad Gateway",
        "",
        r#"{"ok":false,"error":"email_failed"}"#,
    ))
    .await;
    assert_eq!(r, Err(ReportError::Unreachable));
}

#[tokio::test]
async fn times_out_as_unreachable() {
    let (url, _rx) = stub(None).await;
    let started = std::time::Instant::now();
    let r = send(
        Some(&url),
        report(),
        &diagnostics(),
        None,
        Duration::from_millis(500),
    )
    .await;
    assert_eq!(r, Err(ReportError::Unreachable));
    assert!(started.elapsed() < Duration::from_secs(5));
}

#[tokio::test]
async fn closed_port_is_unreachable() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/report", listener.local_addr().unwrap());
    drop(listener);
    let r = send(
        Some(&url),
        report(),
        &diagnostics(),
        None,
        Duration::from_secs(5),
    )
    .await;
    assert_eq!(r, Err(ReportError::Unreachable));
}

#[tokio::test]
async fn no_url_is_not_configured() {
    let r = send(None, report(), &diagnostics(), None, Duration::from_secs(5)).await;
    assert_eq!(r, Err(ReportError::NotConfigured));
}

#[tokio::test]
async fn invalid_report_never_connects() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/report", listener.local_addr().unwrap());
    let mut r = report();
    r.name = "  ".into();
    let result = send(Some(&url), r, &diagnostics(), None, Duration::from_secs(5)).await;
    assert_eq!(
        result,
        Err(ReportError::Invalid {
            field: "name",
            reason: "empty"
        })
    );
    let accepted = tokio::time::timeout(Duration::from_millis(200), listener.accept()).await;
    assert!(accepted.is_err(), "no connection was made");
}

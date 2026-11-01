//! The log excerpt a report carries when the user ticks "Include logs" (research R7): the end
//! of the newest non-empty `sonata.log.*` file, at most [`MAX_LOG`] bytes, starting at a line.

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

use super::{LogExcerpt, MAX_LOG};
use crate::logging::PREFIX;

/// The excerpt, or `None` when there is no readable, non-empty log file.
pub fn read(logs_dir: &Path) -> Option<LogExcerpt> {
    let mut logs: Vec<String> = std::fs::read_dir(logs_dir)
        .ok()?
        .flatten()
        .filter_map(|e| e.file_name().into_string().ok())
        .filter(|name| name.starts_with(PREFIX))
        .collect();
    // Names end in the date, so the newest sorts last.
    logs.sort_unstable_by(|a, b| b.cmp(a));
    logs.into_iter().find_map(|name| {
        let text = tail(&logs_dir.join(&name))?;
        Some(LogExcerpt { text, source: name })
    })
}

/// The last [`MAX_LOG`] bytes of a non-empty file, starting at a line.
fn tail(path: &Path) -> Option<String> {
    let mut file = File::open(path).ok()?;
    let len = file.metadata().ok()?.len();
    if len == 0 {
        return None;
    }
    let max = MAX_LOG as u64;
    // Read one byte before the window too, to know whether the window starts a line.
    let start = len.saturating_sub(max + 1);
    file.seek(SeekFrom::Start(start)).ok()?;
    let mut bytes = Vec::with_capacity((len - start) as usize);
    file.take(max + 1).read_to_end(&mut bytes).ok()?;
    let body = if start == 0 && (bytes.len() as u64) <= max {
        &bytes[..]
    } else {
        let (before, window) = bytes.split_at(bytes.len().saturating_sub(MAX_LOG));
        if before.last() == Some(&b'\n') {
            window
        } else {
            let cut = window.iter().position(|&b| b == b'\n')?;
            &window[cut + 1..]
        }
    };
    (!body.is_empty()).then(|| String::from_utf8_lossy(body).into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(dir: &Path, name: &str, text: &[u8]) {
        std::fs::write(dir.join(name), text).unwrap();
    }

    #[test]
    fn picks_the_newest_non_empty_log() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "sonata.log.2026-10-30", b"old\n");
        write(dir.path(), "sonata.log.2026-10-31", b"yesterday\n");
        write(dir.path(), "sonata.log.2026-11-01", b"");
        write(dir.path(), "other.txt", b"not a log\n");
        let excerpt = read(dir.path()).unwrap();
        assert_eq!(excerpt.source, "sonata.log.2026-10-31");
        assert_eq!(excerpt.text, "yesterday\n");
    }

    #[test]
    fn small_file_is_returned_whole() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "sonata.log.2026-11-01", b"first\nsecond");
        assert_eq!(read(dir.path()).unwrap().text, "first\nsecond");
    }

    #[test]
    fn large_file_keeps_the_end_from_a_line_start() {
        let dir = tempfile::tempdir().unwrap();
        let line = "0123456789abcdefghijklmnopqrstuvwxyz0123456789abcdefghijklmnopq\n"; // 64 bytes
        let text = format!("HEAD-{}", line.repeat(MAX_LOG / 64 + 100));
        write(dir.path(), "sonata.log.2026-11-01", text.as_bytes());
        let excerpt = read(dir.path()).unwrap();
        assert!(excerpt.text.len() <= MAX_LOG);
        assert!(excerpt.text.len() > MAX_LOG - 64);
        assert!(excerpt.text.starts_with("0123"), "starts at a line");
        assert!(text.ends_with(&excerpt.text));
    }

    #[test]
    fn cut_that_falls_on_a_line_start_keeps_the_whole_line() {
        let dir = tempfile::tempdir().unwrap();
        let line = format!("{}\n", "x".repeat(1023)); // 1024 bytes, so MAX_LOG holds whole lines
        let text = format!("{}\n{}", "y".repeat(9), line.repeat(MAX_LOG / 1024));
        write(dir.path(), "sonata.log.2026-11-01", text.as_bytes());
        let excerpt = read(dir.path()).unwrap();
        assert_eq!(excerpt.text.len(), MAX_LOG);
        assert!(excerpt.text.starts_with('x'));
    }

    #[test]
    fn invalid_utf8_is_replaced() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "sonata.log.2026-11-01", b"ok \xff\xfe end\n");
        assert_eq!(read(dir.path()).unwrap().text, "ok \u{FFFD}\u{FFFD} end\n");
    }

    #[test]
    fn nothing_to_send() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(read(&dir.path().join("missing")), None);
        assert_eq!(read(dir.path()), None);
        write(dir.path(), "sonata.log.2026-11-01", b"");
        assert_eq!(read(dir.path()), None);
    }
}

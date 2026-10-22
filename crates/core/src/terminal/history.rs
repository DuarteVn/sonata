//! Saved terminal output (FR-015, research R7). Plain UTF-8 text with SGR colors, one row per
//! line, oldest first, capped by rows and bytes.

use std::io;
use std::path::Path;

use crate::persistence::atomic;

pub const MAX_BYTES: usize = 8 * 1024 * 1024;

/// Writes the newest rows that fit in `max_rows` and `max_bytes`.
pub fn save(path: &Path, rows: &[String], max_rows: usize, max_bytes: usize) -> io::Result<()> {
    let mut kept: Vec<&str> = Vec::new();
    let mut size = 0usize;
    for row in rows.iter().rev().take(max_rows) {
        let len = row.len() + 1;
        if size + len > max_bytes {
            break;
        }
        size += len;
        kept.push(row);
    }
    kept.reverse();
    let mut text = kept.join("\n");
    text.push('\n');
    atomic::write(path, text.as_bytes())
}

/// Reads a history file (at most `max_bytes`) as terminal input: rows separated by CRLF and
/// followed by a dim separator line. Returns `None` when there is no file.
pub fn load(path: &Path, max_bytes: u64) -> Option<Vec<u8>> {
    let bytes = atomic::read_capped(path, max_bytes).ok()?;
    let text = String::from_utf8_lossy(&bytes);
    let text = text.trim_end_matches('\n');
    if text.is_empty() {
        return None;
    }
    let when = std::fs::metadata(path)
        .and_then(|m| m.modified())
        .map(|t| {
            chrono::DateTime::<chrono::Local>::from(t)
                .format("%Y-%m-%d %H:%M")
                .to_string()
        })
        .unwrap_or_default();
    let mut out = text.replace('\n', "\r\n");
    out.push_str(&format!(
        "\x1b[0m\r\n\x1b[2m── restored history · {when} ──\x1b[0m\r\n"
    ));
    Some(out.into_bytes())
}

pub fn delete(path: &Path) -> io::Result<()> {
    match std::fs::remove_file(path) {
        Err(e) if e.kind() != io::ErrorKind::NotFound => Err(e),
        _ => Ok(()),
    }
}

//! Pictures inserted into notes (FR-018): validated, then copied into the workspace's `files/`.

use std::path::Path;

use uuid::Uuid;

use crate::error::{AppError, AppResult, ErrorCode};
use crate::paths::{DataRoot, ensure_private_dir};
use crate::persistence::atomic;

pub const MAX_IMAGE_BYTES: u64 = 20 * 1024 * 1024;

/// Extension to store, if the bytes really are that image type.
fn image_kind(ext: &str, head: &[u8]) -> Option<&'static str> {
    let png = head.starts_with(&[0x89, b'P', b'N', b'G']);
    let jpg = head.starts_with(&[0xFF, 0xD8, 0xFF]);
    let gif = head.starts_with(b"GIF8");
    let webp = head.len() >= 12 && &head[..4] == b"RIFF" && &head[8..12] == b"WEBP";
    match ext {
        "png" if png => Some("png"),
        "jpg" | "jpeg" if jpg => Some("jpg"),
        "gif" if gif => Some("gif"),
        "webp" if webp => Some("webp"),
        _ => None,
    }
}

/// Copies `source` into `workspaces/<ws>/files/<uuid>.<ext>` and returns the Markdown that
/// references it from a note (`![](../files/<name>)`). The source is only read.
pub fn insert(root: &DataRoot, ws: Uuid, source: &Path) -> AppResult<String> {
    let unsupported = || {
        AppError::new(
            ErrorCode::ImageUnsupported,
            "Choose a PNG, JPEG, GIF, or WebP picture.",
        )
    };
    let meta = std::fs::metadata(source)?;
    if !meta.is_file() {
        return Err(unsupported());
    }
    if meta.len() > MAX_IMAGE_BYTES {
        return Err(AppError::new(
            ErrorCode::ImageTooLarge,
            "Pictures can be at most 20 MB.",
        ));
    }
    let ext = source
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or_default()
        .to_lowercase();
    let bytes = std::fs::read(source)?;
    let kind = image_kind(&ext, &bytes[..bytes.len().min(16)]).ok_or_else(unsupported)?;
    let dir = root.workspace_dir(ws).join("files");
    ensure_private_dir(&dir)?;
    let name = format!("{}.{kind}", Uuid::new_v4());
    atomic::write(&dir.join(&name), &bytes)?;
    Ok(format!("![](../files/{name})"))
}

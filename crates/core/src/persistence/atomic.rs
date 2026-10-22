//! All-or-nothing file writes: temp file in the same folder, fsync, rename, fsync folder.

use std::fs::{self, File};
use std::io::{self, Write};
use std::path::Path;

use crate::paths::{FILE_MODE, set_mode};

pub fn write(path: &Path, bytes: &[u8]) -> io::Result<()> {
    write_with(path, |file| file.write_all(bytes))
}

/// Like [`write`], with the content produced by `fill`. If `fill` fails, the temporary file is
/// removed and `path` is left untouched.
pub fn write_with(path: &Path, fill: impl FnOnce(&mut File) -> io::Result<()>) -> io::Result<()> {
    let dir = path
        .parent()
        .ok_or_else(|| io::Error::other("path has no parent folder"))?;
    let mut tmp = tempfile::Builder::new().prefix(".tmp-").tempfile_in(dir)?;
    set_mode(tmp.path(), FILE_MODE)?;
    fill(tmp.as_file_mut())?;
    tmp.as_file().sync_all()?;
    tmp.persist(path).map_err(|e| e.error)?;
    File::open(dir)?.sync_all()?;
    Ok(())
}

/// Reads at most `limit` bytes of a file.
pub fn read_capped(path: &Path, limit: u64) -> io::Result<Vec<u8>> {
    use std::io::Read;
    let mut buf = Vec::new();
    fs::File::open(path)?.take(limit).read_to_end(&mut buf)?;
    Ok(buf)
}

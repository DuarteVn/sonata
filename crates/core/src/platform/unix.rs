//! Unix bodies (Linux and macOS) of the shared helpers in [`super`] (feature 012).

use std::fs::{self, DirBuilder, File, OpenOptions};
use std::io;
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};

use super::Kind;
use crate::paths::{DIR_MODE, FILE_MODE};

fn mode(kind: Kind) -> u32 {
    match kind {
        Kind::Dir => DIR_MODE,
        Kind::File => FILE_MODE,
    }
}

pub fn make_private(path: &Path, kind: Kind) -> io::Result<()> {
    crate::paths::set_mode(path, mode(kind))
}

pub fn is_private(path: &Path, kind: Kind) -> io::Result<bool> {
    Ok(fs::symlink_metadata(path)?.permissions().mode() & 0o777 == mode(kind))
}

pub fn create_dir_all(path: &Path) -> io::Result<()> {
    DirBuilder::new()
        .recursive(true)
        .mode(DIR_MODE)
        .create(path)
}

pub fn open_private_append(path: &Path) -> io::Result<File> {
    OpenOptions::new()
        .create(true)
        .append(true)
        .mode(FILE_MODE)
        .open(path)
}

pub fn is_executable(path: &Path) -> bool {
    fs::metadata(path).is_ok_and(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
}

pub fn process_alive(pid: u32) -> bool {
    let Ok(pid) = libc::pid_t::try_from(pid) else {
        return false;
    };
    // SAFETY: signal 0 only checks that the process exists.
    unsafe { libc::kill(pid, 0) == 0 }
}

pub fn simplify(path: &Path) -> PathBuf {
    path.to_path_buf()
}

pub fn executable_names(name: &str) -> Vec<String> {
    vec![name.to_string()]
}

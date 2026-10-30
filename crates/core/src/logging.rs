//! File logging to `<data root>/logs/sonata.log.<date>`. Files are private (`0600`, or a user-only access list on Windows), kept for 7
//! days. Terminal content is never logged.

use std::fs::{self, File};
use std::io::{self, Write};
use std::path::PathBuf;

use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::EnvFilter;

use crate::paths::{DataRoot, ensure_private_dir};

const PREFIX: &str = "sonata.log.";
const KEEP_DAYS: usize = 7;

/// Appends to one file per day, created private.
struct DailyFile {
    dir: PathBuf,
    date: String,
    file: Option<File>,
}

impl DailyFile {
    fn current(&mut self) -> io::Result<&mut File> {
        let today = chrono::Local::now().format("%Y-%m-%d").to_string();
        if self.file.is_none() || self.date != today {
            let path = self.dir.join(format!("{PREFIX}{today}"));
            let file = crate::platform::open_private_append(&path)?;
            self.file = Some(file);
            self.date = today;
            prune(&self.dir);
        }
        Ok(self.file.as_mut().expect("just opened"))
    }
}

impl Write for DailyFile {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.current()?.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        match &mut self.file {
            Some(f) => f.flush(),
            None => Ok(()),
        }
    }
}

fn prune(dir: &PathBuf) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    let mut logs: Vec<PathBuf> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with(PREFIX))
        })
        .collect();
    logs.sort();
    let excess = logs.len().saturating_sub(KEEP_DAYS);
    for old in logs.into_iter().take(excess) {
        let _ = fs::remove_file(old);
    }
}

/// Installs the global subscriber. Keep the returned guard alive for the app's lifetime.
pub fn init(root: &DataRoot) -> io::Result<WorkerGuard> {
    let dir = root.logs_dir();
    ensure_private_dir(&dir)?;
    let (writer, guard) = tracing_appender::non_blocking(DailyFile {
        dir,
        date: String::new(),
        file: None,
    });
    let filter = EnvFilter::try_from_env("SONATA_LOG").unwrap_or_else(|_| EnvFilter::new("info"));
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(writer)
        .with_ansi(false)
        .try_init();
    Ok(guard)
}

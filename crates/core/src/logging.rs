//! File logging to `<data root>/logs/sonata.log.<date>`. Terminal content is never logged.

use tracing_appender::non_blocking::WorkerGuard;
use tracing_appender::rolling::{Builder, Rotation};
use tracing_subscriber::EnvFilter;

use crate::paths::{DataRoot, ensure_private_dir};

/// Installs the global subscriber. Keep the returned guard alive for the app's lifetime.
pub fn init(root: &DataRoot) -> std::io::Result<WorkerGuard> {
    let dir = root.logs_dir();
    ensure_private_dir(&dir)?;
    let appender = Builder::new()
        .rotation(Rotation::DAILY)
        .filename_prefix("sonata.log")
        .max_log_files(7)
        .build(&dir)
        .map_err(std::io::Error::other)?;
    let (writer, guard) = tracing_appender::non_blocking(appender);
    let filter = EnvFilter::try_from_env("SONATA_LOG").unwrap_or_else(|_| EnvFilter::new("info"));
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(writer)
        .with_ansi(false)
        .try_init();
    Ok(guard)
}

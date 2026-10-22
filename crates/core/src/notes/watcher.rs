//! Watches a workspace's `notes/` folder (FSEvents through `notify`, 300 ms debounce).

use std::path::Path;
use std::time::Duration;

use notify::RecursiveMode;
use notify_debouncer_full::{DebounceEventResult, Debouncer, RecommendedCache, new_debouncer};
use uuid::Uuid;

pub const DEBOUNCE: Duration = Duration::from_millis(300);

pub type NotesWatcher = Debouncer<notify::RecommendedWatcher, RecommendedCache>;

/// Note id for `<uuid>.md`; `None` for conflict copies, temporary files, and anything else.
pub fn note_id_of(path: &Path) -> Option<Uuid> {
    let name = path.file_name()?.to_str()?;
    let stem = name.strip_suffix(".md")?;
    crate::paths::parse_workspace_folder(stem)
}

/// Calls `on_change` with the note id of every changed `<uuid>.md` file in `dir`.
pub fn watch(
    dir: &Path,
    on_change: impl Fn(Uuid) + Send + 'static,
) -> notify::Result<NotesWatcher> {
    let mut debouncer = new_debouncer(DEBOUNCE, None, move |result: DebounceEventResult| {
        let Ok(events) = result else { return };
        let mut ids: Vec<Uuid> = events
            .iter()
            .flat_map(|e| e.paths.iter())
            .filter_map(|p| note_id_of(p))
            .collect();
        ids.sort();
        ids.dedup();
        for id in ids {
            on_change(id);
        }
    })?;
    debouncer.watch(dir, RecursiveMode::NonRecursive)?;
    Ok(debouncer)
}

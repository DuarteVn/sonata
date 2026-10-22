//! Markdown notes stored as `notes/<nodeId>.md` (FR-020) and kept in sync with edits made
//! outside the app (FR-020a, research R8).

use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::io;
use std::path::PathBuf;

use serde::{Serialize, Serializer};
use uuid::Uuid;

use crate::events::{CoreEvent, NoteChangedEvent, NoteConflictEvent};
use crate::paths::DataRoot;
use crate::persistence::atomic;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NoteSync {
    Clean,
    /// In-app edits not yet written to disk.
    Dirty,
    /// The file changed outside the app while it had unsaved edits; the outside text was kept
    /// at `path`.
    Conflict {
        path: PathBuf,
    },
    /// The file was deleted outside the app; it is written again at the next save.
    FileMissing,
}

impl NoteSync {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Clean => "clean",
            Self::Dirty => "dirty",
            Self::Conflict { .. } => "conflict",
            Self::FileMissing => "fileMissing",
        }
    }
}

impl Serialize for NoteSync {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(self.name())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NoteView {
    pub text: String,
    pub sync: NoteSync,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conflict_path: Option<PathBuf>,
    /// The note's folder, so relative image paths can be resolved.
    pub asset_base: PathBuf,
}

#[derive(Debug)]
struct NoteState {
    ws: Uuid,
    text: String,
    dirty: bool,
    /// Hash of the content Sonata last wrote or read; matching file events are our own.
    disk_hash: Option<u64>,
    sync: NoteSync,
}

fn hash(text: &str) -> u64 {
    let mut h = DefaultHasher::new();
    text.hash(&mut h);
    h.finish()
}

pub struct NoteStore {
    root: DataRoot,
    notes: HashMap<Uuid, NoteState>,
}

impl NoteStore {
    pub fn new(root: DataRoot) -> Self {
        Self {
            root,
            notes: HashMap::new(),
        }
    }

    pub fn path(&self, ws: Uuid, node: Uuid) -> PathBuf {
        self.root.notes_dir(ws).join(format!("{node}.md"))
    }

    pub fn conflict_path(&self, ws: Uuid, node: Uuid) -> PathBuf {
        self.root
            .notes_dir(ws)
            .join(format!("{node} (conflict).md"))
    }

    /// Creates an empty note file.
    pub fn create(&mut self, ws: Uuid, node: Uuid) -> io::Result<()> {
        crate::paths::ensure_private_dir(&self.root.notes_dir(ws))?;
        atomic::write(&self.path(ws, node), b"")?;
        self.notes.insert(
            node,
            NoteState {
                ws,
                text: String::new(),
                dirty: false,
                disk_hash: Some(hash("")),
                sync: NoteSync::Clean,
            },
        );
        Ok(())
    }

    fn state(&mut self, ws: Uuid, node: Uuid) -> &mut NoteState {
        let path = self.path(ws, node);
        self.notes
            .entry(node)
            .or_insert_with(|| match std::fs::read_to_string(&path) {
                Ok(text) => NoteState {
                    ws,
                    disk_hash: Some(hash(&text)),
                    text,
                    dirty: false,
                    sync: NoteSync::Clean,
                },
                Err(_) => NoteState {
                    ws,
                    text: String::new(),
                    dirty: true,
                    disk_hash: None,
                    sync: NoteSync::FileMissing,
                },
            })
    }

    pub fn read(&mut self, ws: Uuid, node: Uuid) -> NoteView {
        let asset_base = self.root.notes_dir(ws);
        let s = self.state(ws, node);
        let conflict_path = match &s.sync {
            NoteSync::Conflict { path } => Some(path.clone()),
            _ => None,
        };
        NoteView {
            text: s.text.clone(),
            sync: s.sync.clone(),
            conflict_path,
            asset_base,
        }
    }

    /// Updates the text. With `flush`, writes the file now; otherwise the next save does.
    pub fn write(&mut self, ws: Uuid, node: Uuid, text: &str, flush: bool) -> io::Result<()> {
        let s = self.state(ws, node);
        if s.text != text {
            s.text = text.to_string();
            s.dirty = true;
            if !matches!(s.sync, NoteSync::Conflict { .. }) {
                s.sync = NoteSync::Dirty;
            }
        }
        if flush { self.save(node) } else { Ok(()) }
    }

    fn save(&mut self, node: Uuid) -> io::Result<()> {
        let Some(s) = self.notes.get(&node) else {
            return Ok(());
        };
        if !s.dirty {
            return Ok(());
        }
        let (ws, text) = (s.ws, s.text.clone());
        crate::paths::ensure_private_dir(&self.root.notes_dir(ws))?;
        atomic::write(&self.path(ws, node), text.as_bytes())?;
        let s = self.notes.get_mut(&node).expect("checked");
        s.dirty = false;
        s.disk_hash = Some(hash(&text));
        s.sync = NoteSync::Clean;
        Ok(())
    }

    /// Writes every note with unsaved edits. Returns error messages.
    pub fn save_dirty(&mut self) -> Vec<String> {
        let dirty: Vec<Uuid> = self
            .notes
            .iter()
            .filter(|(_, s)| s.dirty)
            .map(|(id, _)| *id)
            .collect();
        dirty
            .into_iter()
            .filter_map(|id| self.save(id).err().map(|e| format!("note {id}: {e}")))
            .collect()
    }

    pub fn delete(&mut self, ws: Uuid, node: Uuid) -> io::Result<()> {
        self.notes.remove(&node);
        match std::fs::remove_file(self.path(ws, node)) {
            Err(e) if e.kind() != io::ErrorKind::NotFound => Err(e),
            _ => Ok(()),
        }
    }

    pub fn forget_workspace(&mut self, ws: Uuid) {
        self.notes.retain(|_, s| s.ws != ws);
    }

    /// Reacts to a file event for a note. Returns the event to send to the UI, if any.
    pub fn external_change(&mut self, ws: Uuid, node: Uuid) -> Option<CoreEvent> {
        let path = self.path(ws, node);
        let conflict = self.conflict_path(ws, node);
        let s = self.notes.get_mut(&node)?;
        match std::fs::read_to_string(&path) {
            Ok(text) => {
                let h = hash(&text);
                if s.disk_hash == Some(h) {
                    return None;
                }
                if s.dirty {
                    if let Err(e) = atomic::write(&conflict, text.as_bytes()) {
                        tracing::error!(error = %e, "could not write note conflict copy");
                        return None;
                    }
                    s.disk_hash = Some(h);
                    s.sync = NoteSync::Conflict {
                        path: conflict.clone(),
                    };
                    Some(CoreEvent::NoteConflict(NoteConflictEvent {
                        node_id: node,
                        conflict_path: conflict.display().to_string(),
                    }))
                } else {
                    s.text = text.clone();
                    s.disk_hash = Some(h);
                    s.sync = NoteSync::Clean;
                    Some(CoreEvent::NoteChanged(NoteChangedEvent {
                        node_id: node,
                        text,
                        sync: "clean".into(),
                    }))
                }
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                if s.sync == NoteSync::FileMissing {
                    return None;
                }
                s.dirty = true;
                s.disk_hash = None;
                s.sync = NoteSync::FileMissing;
                Some(CoreEvent::NoteChanged(NoteChangedEvent {
                    node_id: node,
                    text: s.text.clone(),
                    sync: "fileMissing".into(),
                }))
            }
            Err(_) => None,
        }
    }
}

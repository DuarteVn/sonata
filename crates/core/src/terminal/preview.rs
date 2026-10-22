//! Text previews for terminals that are not live on screen (FR-014, research R3).

use std::collections::{HashMap, HashSet};
use std::time::Duration;

use uuid::Uuid;

use super::manager::TerminalManager;
use crate::events::PreviewEvent;

/// Each subscribed terminal gets at most one preview per interval.
pub const PREVIEW_INTERVAL: Duration = Duration::from_secs(1);

#[derive(Default)]
pub struct PreviewState {
    subscribed: HashSet<Uuid>,
    /// Output sequence number at the last preview sent, per terminal.
    sent_seq: HashMap<Uuid, u64>,
}

impl PreviewState {
    /// Replaces the subscription set. Returns ids that were not subscribed before.
    pub fn set(&mut self, ids: impl IntoIterator<Item = Uuid>) -> Vec<Uuid> {
        let next: HashSet<Uuid> = ids.into_iter().collect();
        let added = next.difference(&self.subscribed).copied().collect();
        self.sent_seq.retain(|id, _| next.contains(id));
        self.subscribed = next;
        added
    }

    /// Previews for `ids` (or all subscribed when `None`) whose output changed since last sent.
    pub fn due(&mut self, terminals: &TerminalManager, ids: Option<&[Uuid]>) -> Vec<PreviewEvent> {
        let candidates: Vec<Uuid> = match ids {
            Some(ids) => ids.to_vec(),
            None => self.subscribed.iter().copied().collect(),
        };
        let mut out = Vec::new();
        for id in candidates {
            let Some(seq) = terminals.output_seq(id) else {
                continue;
            };
            if self.sent_seq.get(&id) == Some(&seq) {
                continue;
            }
            let Some((lines, cursor)) = terminals.preview(id) else {
                continue;
            };
            self.sent_seq.insert(id, seq);
            out.push(PreviewEvent {
                node_id: id,
                lines,
                cursor: Some(cursor),
            });
        }
        out
    }
}

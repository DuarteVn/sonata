//! Forwards core events to the main window.

use sonata_core::events::{CoreEvent, EventSink};
use tauri::{AppHandle, Emitter};

pub struct TauriEventSink {
    pub handle: AppHandle,
}

impl EventSink for TauriEventSink {
    fn emit(&self, event: CoreEvent) {
        if let Err(err) = self.handle.emit(event.name(), event.payload()) {
            tracing::warn!(error = %err, event = event.name(), "failed to emit event");
        }
    }
}

//! Sonata core: data model, persistence, workspaces, terminals, and notes.
//!
//! This crate has no Tauri dependency (constitution principle II). The desktop shell
//! calls into [`App`] and receives [`events::CoreEvent`]s through an [`events::EventSink`].

pub mod agents;
pub mod app;
pub mod autosave;
pub mod browser;
pub mod crash_guard;
pub mod device;
pub mod error;
pub mod events;
pub mod lanes;
pub mod logging;
pub mod messaging;
pub mod model;
pub mod notes;
pub mod paths;
pub mod persistence;
pub mod presets;
pub mod roles;
pub mod terminal;
pub mod update;
pub mod usage;
pub mod workspace;

pub use app::App;
pub use error::{AppError, ErrorCode};

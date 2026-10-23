//! Device nodes (feature 007): Android emulators and iOS simulators on the canvas. Core owns
//! detection, the emulator processes, adb and gRPC calls, and agent commands; the shell only
//! forwards frames and intents (research R12).

pub mod grpc;
pub mod still;

use uuid::Uuid;

/// Default node frame: a phone screen plus the node's chrome (data-model.md).
pub const DEFAULT_SIZE: (f64, f64) = (380.0, 820.0);

/// Short id printed by `sonata list` for devices (`d-xxxx`).
pub fn short_id(id: Uuid) -> String {
    format!("d-{}", &id.simple().to_string()[..4])
}

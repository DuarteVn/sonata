//! Device nodes (feature 007): Android emulators and iOS simulators on the canvas. Core owns
//! detection, the emulator processes, adb and gRPC calls, and agent commands; the shell only
//! forwards frames and intents (research R12).

pub mod grpc;

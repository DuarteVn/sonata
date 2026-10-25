//! Errors returned to the frontend. Codes match `contracts/ipc.md`.

use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum ErrorCode {
    NameEmpty,
    NameTooLong,
    NameTaken,
    DirectoryMissing,
    NotADirectory,
    WorkspaceNotFound,
    NodeNotFound,
    WorkspaceNotLoaded,
    TerminalBusy,
    LinkSelf,
    LinkExists,
    RoleNameTaken,
    ImageTooLarge,
    ImageUnsupported,
    InvalidAddress,
    UsageProviderNotFound,
    ConsentMismatch,
    InvalidColor,
    LaneUnavailable,
    LaneBlocked,
    LaneConflict,
    LaneUnsafePath,
    LaneHasChanges,
    ConfirmationRequired,
    LaneConfirmationNeeded,
    DeviceInUse,
    DeviceNotRunning,
    ToolMissing,
    InvalidConversation,
    Io,
    Internal,
}

/// A user-facing error: a stable code plus an English message.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, thiserror::Error)]
#[error("{message}")]
pub struct AppError {
    pub code: ErrorCode,
    pub message: String,
}

impl AppError {
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    pub fn workspace_not_found(id: impl std::fmt::Display) -> Self {
        Self::new(
            ErrorCode::WorkspaceNotFound,
            format!("No workspace with id {id}."),
        )
    }

    pub fn node_not_found(id: impl std::fmt::Display) -> Self {
        Self::new(
            ErrorCode::NodeNotFound,
            format!("No canvas item with id {id}."),
        )
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::Internal, message)
    }
}

impl From<std::io::Error> for AppError {
    fn from(err: std::io::Error) -> Self {
        Self::new(ErrorCode::Io, err.to_string())
    }
}

pub type AppResult<T> = Result<T, AppError>;

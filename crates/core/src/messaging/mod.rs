//! Agent messaging (feature 003): the socket the `sonata` CLI talks to, the ask broker, and the
//! command handlers. Every permission check happens here, on the app side.

pub mod broker;
pub mod commands;
pub mod extract;
pub mod instructions;
#[cfg(windows)]
pub mod pipe;
pub mod server;

use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const EXIT_OK: u8 = 0;
pub const EXIT_ERROR: u8 = 1;
pub const EXIT_USAGE: u8 = 2;
pub const EXIT_DENIED: u8 = 3;
pub const EXIT_NOT_FOUND: u8 = 4;
pub const EXIT_TIMEOUT: u8 = 5;

/// Limits from the spec (FR-029) and the socket contract.
pub const MAX_REQUEST_BYTES: usize = 128 * 1024;
pub const MAX_MESSAGE_BYTES: usize = 100 * 1024;
pub const MAX_RESULT_BYTES: usize = 1024 * 1024;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Request {
    pub v: u32,
    pub terminal_id: uuid::Uuid,
    pub token: String,
    #[serde(default)]
    pub cwd: String,
    pub args: Vec<String>,
    #[serde(default)]
    pub stdin: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Reply {
    pub ok: bool,
    pub exit_code: u8,
    pub text: String,
    #[serde(skip_serializing_if = "Value::is_null")]
    pub data: Value,
}

impl Reply {
    pub fn ok(text: impl Into<String>) -> Self {
        Self {
            ok: true,
            exit_code: EXIT_OK,
            text: text.into(),
            data: Value::Null,
        }
    }

    pub fn with_data(mut self, data: Value) -> Self {
        self.data = data;
        self
    }

    pub fn err(code: u8, text: impl Into<String>) -> Self {
        Self {
            ok: false,
            exit_code: code,
            text: text.into(),
            data: Value::Null,
        }
    }
}

/// Cuts text to at most `max` bytes on a character boundary, with a note.
pub fn cap_text(mut text: String, max: usize) -> String {
    if text.len() <= max {
        return text;
    }
    let mut cut = max;
    while !text.is_char_boundary(cut) {
        cut -= 1;
    }
    text.truncate(cut);
    text.push_str("\n[answer truncated at 1 MB]");
    text
}

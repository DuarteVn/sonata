//! Per-provider consent to read usage (FR-012, FR-019).

use serde::{Deserialize, Serialize};

/// The user's answer for one provider. `Granted` holds the exact command it approves.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "camelCase")]
pub enum Consent {
    #[default]
    Unasked,
    #[serde(rename_all = "camelCase")]
    Granted {
        approved_command: Vec<String>,
    },
    Denied,
}

impl Consent {
    pub fn granted(command: Vec<String>) -> Self {
        Self::Granted {
            approved_command: command,
        }
    }

    pub fn is_granted_for(&self, command: &[String]) -> bool {
        matches!(self, Self::Granted { approved_command } if approved_command == command)
    }

    /// `granted`, `denied`, or `unasked` (also when a different command was approved).
    pub fn state_for(&self, command: &[String]) -> &'static str {
        match self {
            Self::Denied => "denied",
            _ if self.is_granted_for(command) => "granted",
            _ => "unasked",
        }
    }
}

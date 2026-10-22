//! Schema migrations applied to raw JSON before typed decoding.

use serde_json::Value;

use super::envelope::EnvelopeError;

/// Upgrades a payload from version `n` to `n + 1`.
pub type Step = fn(Value) -> Result<Value, String>;

#[derive(Debug, Clone)]
pub struct Migrations {
    current: u32,
    steps: Vec<(u32, Step)>,
}

impl Migrations {
    pub fn new(current: u32) -> Self {
        Self {
            current,
            steps: Vec::new(),
        }
    }

    /// Registers the step that upgrades version `from` to `from + 1`.
    pub fn step(mut self, from: u32, step: Step) -> Self {
        self.steps.push((from, step));
        self
    }

    pub fn current(&self) -> u32 {
        self.current
    }

    pub fn migrate(&self, from: u32, mut payload: Value) -> Result<Value, EnvelopeError> {
        if from > self.current {
            return Err(EnvelopeError::NewerVersion {
                found: from,
                supported: self.current,
            });
        }
        for version in from..self.current {
            let step = self
                .steps
                .iter()
                .find(|(v, _)| *v == version)
                .map(|(_, s)| *s)
                .ok_or_else(|| EnvelopeError::MigrationFailed {
                    step: version,
                    detail: format!("missing migration step for version {version}"),
                })?;
            payload = step(payload).map_err(|detail| EnvelopeError::MigrationFailed {
                step: version,
                detail,
            })?;
        }
        Ok(payload)
    }
}

//! The `{type, schemaVersion, payload}` envelope used by every JSON file.

use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::{Map, Value};

use super::migrate::Migrations;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum EnvelopeError {
    #[error("file contains invalid JSON: {0}")]
    InvalidJson(String),
    #[error("expected a file of type '{expected}', found '{found}'")]
    WrongType { expected: String, found: String },
    #[error(
        "file was written by a newer version (format {found}, this build supports up to {supported})"
    )]
    NewerVersion { found: u32, supported: u32 },
    #[error("upgrade from format {step} failed: {detail}")]
    MigrationFailed { step: u32, detail: String },
    #[error("could not decode file: {0}")]
    DecodeFailed(String),
}

/// Encodes `payload` as a pretty-printed envelope with keys sorted at every level.
pub fn encode<T: Serialize>(
    kind: &str,
    version: u32,
    payload: &T,
) -> Result<Vec<u8>, EnvelopeError> {
    let payload =
        serde_json::to_value(payload).map_err(|e| EnvelopeError::DecodeFailed(e.to_string()))?;
    let mut root = Map::new();
    root.insert("payload".into(), payload);
    root.insert("schemaVersion".into(), Value::from(version));
    root.insert("type".into(), Value::from(kind));
    let mut bytes = serde_json::to_vec_pretty(&sort_keys(Value::Object(root)))
        .map_err(|e| EnvelopeError::DecodeFailed(e.to_string()))?;
    bytes.push(b'\n');
    Ok(bytes)
}

/// Decodes an envelope of type `kind`, migrating older payloads first.
pub fn decode<T: DeserializeOwned>(
    bytes: &[u8],
    kind: &str,
    migrations: &Migrations,
) -> Result<T, EnvelopeError> {
    let value: Value =
        serde_json::from_slice(bytes).map_err(|e| EnvelopeError::InvalidJson(e.to_string()))?;
    let Value::Object(mut root) = value else {
        return Err(EnvelopeError::InvalidJson(
            "root is not a JSON object".into(),
        ));
    };
    let found = root
        .get("type")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    if found != kind {
        return Err(EnvelopeError::WrongType {
            expected: kind.into(),
            found,
        });
    }
    let version = root
        .get("schemaVersion")
        .and_then(Value::as_u64)
        .and_then(|v| u32::try_from(v).ok())
        .ok_or_else(|| EnvelopeError::DecodeFailed("missing or invalid schemaVersion".into()))?;
    let payload = root.remove("payload").unwrap_or(Value::Null);
    if !payload.is_object() {
        return Err(EnvelopeError::DecodeFailed(
            "payload must be a JSON object".into(),
        ));
    }
    let payload = migrations.migrate(version, payload)?;
    serde_json::from_value(payload).map_err(|e| EnvelopeError::DecodeFailed(e.to_string()))
}

/// Rebuilds objects with keys inserted in sorted order, so output is sorted whether or not
/// `serde_json`'s `preserve_order` feature is enabled somewhere in the build.
fn sort_keys(value: Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut entries: Vec<_> = map.into_iter().collect();
            entries.sort_by(|a, b| a.0.cmp(&b.0));
            Value::Object(
                entries
                    .into_iter()
                    .map(|(k, v)| (k, sort_keys(v)))
                    .collect(),
            )
        }
        Value::Array(items) => Value::Array(items.into_iter().map(sort_keys).collect()),
        other => other,
    }
}

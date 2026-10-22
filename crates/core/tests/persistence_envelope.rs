use pretty_assertions::assert_eq;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sonata_core::persistence::envelope::{self, EnvelopeError};
use sonata_core::persistence::migrate::Migrations;

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Sample {
    zeta: u32,
    alpha: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    maybe: Option<String>,
}

fn sample() -> Sample {
    Sample {
        zeta: 1,
        alpha: "a".into(),
        maybe: None,
    }
}

#[test]
fn round_trip() {
    let bytes = envelope::encode("sample", 1, &sample()).unwrap();
    let back: Sample = envelope::decode(&bytes, "sample", &Migrations::new(1)).unwrap();
    assert_eq!(back, sample());
}

#[test]
fn keys_sorted_and_none_omitted() {
    let bytes = envelope::encode("sample", 1, &sample()).unwrap();
    let text = String::from_utf8(bytes).unwrap();
    let expected = "{\n  \"payload\": {\n    \"alpha\": \"a\",\n    \"zeta\": 1\n  },\n  \"schemaVersion\": 1,\n  \"type\": \"sample\"\n}\n";
    assert_eq!(text, expected);
}

#[test]
fn wrong_type_is_rejected() {
    let bytes = envelope::encode("other", 1, &sample()).unwrap();
    let err = envelope::decode::<Sample>(&bytes, "sample", &Migrations::new(1)).unwrap_err();
    assert!(matches!(err, EnvelopeError::WrongType { .. }), "{err:?}");
}

#[test]
fn invalid_json_is_rejected() {
    let err = envelope::decode::<Sample>(b"{", "sample", &Migrations::new(1)).unwrap_err();
    assert!(matches!(err, EnvelopeError::InvalidJson(_)), "{err:?}");
}

#[test]
fn newer_version_is_rejected() {
    let bytes = envelope::encode("sample", 3, &sample()).unwrap();
    let err = envelope::decode::<Sample>(&bytes, "sample", &Migrations::new(1)).unwrap_err();
    assert_eq!(
        err,
        EnvelopeError::NewerVersion {
            found: 3,
            supported: 1
        }
    );
}

fn v0_to_v1(mut v: Value) -> Result<Value, String> {
    let name = v["name"].take();
    Ok(json!({ "alpha": name, "zeta": 0 }))
}

fn v1_to_v2(mut v: Value) -> Result<Value, String> {
    v["zeta"] = json!(v["zeta"].as_u64().unwrap() + 2);
    Ok(v)
}

#[test]
fn migrations_apply_in_order_before_decode() {
    let raw = json!({ "type": "sample", "schemaVersion": 0, "payload": { "name": "old" } });
    let bytes = serde_json::to_vec(&raw).unwrap();
    let migrations = Migrations::new(2).step(0, v0_to_v1).step(1, v1_to_v2);
    let back: Sample = envelope::decode(&bytes, "sample", &migrations).unwrap();
    assert_eq!(
        back,
        Sample {
            zeta: 2,
            alpha: "old".into(),
            maybe: None
        }
    );
}

#[test]
fn missing_migration_step_fails() {
    let raw = json!({ "type": "sample", "schemaVersion": 0, "payload": {} });
    let bytes = serde_json::to_vec(&raw).unwrap();
    let migrations = Migrations::new(2).step(1, v1_to_v2);
    let err = envelope::decode::<Sample>(&bytes, "sample", &migrations).unwrap_err();
    assert!(
        matches!(err, EnvelopeError::MigrationFailed { step: 0, .. }),
        "{err:?}"
    );
}

#[test]
fn decode_failure_is_reported() {
    let raw = json!({ "type": "sample", "schemaVersion": 1, "payload": { "zeta": "nope" } });
    let bytes = serde_json::to_vec(&raw).unwrap();
    let err = envelope::decode::<Sample>(&bytes, "sample", &Migrations::new(1)).unwrap_err();
    assert!(matches!(err, EnvelopeError::DecodeFailed(_)), "{err:?}");
}

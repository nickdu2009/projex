//! Delta compress / decompress / checksum tests

use app_lib::sync::{Delta, Operation, OperationType};
use app_lib::sync::VectorClock;

fn make_sample_ops() -> Vec<Operation> {
    vec![
        Operation {
            table_name: "projects".into(),
            record_id: "proj-001".into(),
            op_type: OperationType::Insert,
            data: Some(serde_json::json!({
                "id": "proj-001",
                "name": "Test Project"
            })),
            version: 1,
        },
        Operation {
            table_name: "persons".into(),
            record_id: "per-001".into(),
            op_type: OperationType::Update,
            data: Some(serde_json::json!({
                "id": "per-001",
                "display_name": "Alice"
            })),
            version: 2,
        },
        Operation {
            table_name: "partners".into(),
            record_id: "part-001".into(),
            op_type: OperationType::Delete,
            data: None,
            version: 3,
        },
    ]
}

fn make_delta() -> Delta {
    let ops = make_sample_ops();
    let checksum = Delta::calculate_checksum(&ops);
    Delta {
        id: 1,
        operations: ops,
        device_id: "test-device".into(),
        vector_clock: VectorClock::new("test-device".into()),
        created_at: "2026-01-01T00:00:00Z".into(),
        checksum,
    }
}

// ──────────────────────── Checksum ────────────────────────

#[test]
fn checksum_is_deterministic() {
    let ops = make_sample_ops();
    let c1 = Delta::calculate_checksum(&ops);
    let c2 = Delta::calculate_checksum(&ops);
    assert_eq!(c1, c2);
}

#[test]
fn checksum_is_64_hex_chars() {
    let ops = make_sample_ops();
    let c = Delta::calculate_checksum(&ops);
    assert_eq!(c.len(), 64); // SHA-256 = 32 bytes = 64 hex chars
    assert!(c.chars().all(|ch| ch.is_ascii_hexdigit()));
}

#[test]
fn different_ops_produce_different_checksum() {
    let ops1 = make_sample_ops();
    let ops2 = vec![ops1[0].clone()]; // only first op
    assert_ne!(
        Delta::calculate_checksum(&ops1),
        Delta::calculate_checksum(&ops2)
    );
}

#[test]
fn empty_ops_checksum_is_valid() {
    let c = Delta::calculate_checksum(&[]);
    assert_eq!(c.len(), 64);
}

// ──────────────────────── Compress / Decompress ────────────────────────

#[test]
fn compress_decompress_roundtrip() {
    let delta = make_delta();

    let compressed = delta.compress().expect("compress should succeed");
    assert!(!compressed.is_empty());

    let restored = Delta::decompress(&compressed).expect("decompress should succeed");
    assert_eq!(restored.id, delta.id);
    assert_eq!(restored.device_id, delta.device_id);
    assert_eq!(restored.checksum, delta.checksum);
    assert_eq!(restored.operations.len(), delta.operations.len());
}

#[test]
fn compressed_is_smaller_than_json() {
    let delta = make_delta();
    let json_bytes = serde_json::to_vec(&delta).unwrap();
    let compressed = delta.compress().unwrap();

    // Gzip should produce smaller output for typical JSON
    assert!(
        compressed.len() < json_bytes.len(),
        "compressed {} bytes should be < json {} bytes",
        compressed.len(),
        json_bytes.len()
    );
}

#[test]
fn decompress_invalid_data_returns_error() {
    let result = Delta::decompress(b"this is not valid gzip");
    assert!(result.is_err());
}

// ──────────────────────── Serde ────────────────────────

#[test]
fn delta_serde_json_roundtrip() {
    let delta = make_delta();
    let json = serde_json::to_string(&delta).unwrap();
    let restored: Delta = serde_json::from_str(&json).unwrap();
    assert_eq!(restored.id, delta.id);
    assert_eq!(restored.device_id, delta.device_id);
    assert_eq!(restored.operations.len(), 3);
}

#[test]
fn operation_types_serialize_correctly() {
    let insert = serde_json::to_string(&OperationType::Insert).unwrap();
    let update = serde_json::to_string(&OperationType::Update).unwrap();
    let delete = serde_json::to_string(&OperationType::Delete).unwrap();
    assert_eq!(insert, "\"Insert\"");
    assert_eq!(update, "\"Update\"");
    assert_eq!(delete, "\"Delete\"");
}

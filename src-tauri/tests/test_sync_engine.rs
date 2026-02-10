//! DeltaSyncEngine integration tests (in-memory SQLite)

use app_lib::infra::db::init_test_db;
use app_lib::sync::DeltaSyncEngine;

// ──────────────────────── Helper ────────────────────────

fn setup() -> (app_lib::infra::DbPool, String) {
    let pool = init_test_db();
    let device_id = {
        let conn = pool.0.lock().unwrap();
        conn.query_row(
            "SELECT value FROM sync_config WHERE key = 'device_id'",
            [],
            |row: &rusqlite::Row<'_>| row.get::<_, String>(0),
        )
        .unwrap()
    };
    (pool, device_id)
}

fn insert_person(pool: &app_lib::infra::DbPool, id: &str, name: &str) {
    let conn = pool.0.lock().unwrap();
    conn.execute(
        "INSERT INTO persons (id, display_name, email, role, note, is_active, created_at, updated_at, _version)
         VALUES (?1, ?2, '', '', '', 1, datetime('now'), datetime('now'), 1)",
        rusqlite::params![id, name],
    )
    .unwrap();
}

fn insert_sync_metadata(
    pool: &app_lib::infra::DbPool,
    table: &str,
    record_id: &str,
    operation: &str,
    device_id: &str,
) {
    let conn = pool.0.lock().unwrap();
    conn.execute(
        "INSERT INTO sync_metadata (table_name, record_id, operation, data_snapshot, device_id, version, created_at, synced)
         VALUES (?1, ?2, ?3, ?4, ?5, 1, datetime('now'), 0)",
        rusqlite::params![
            table,
            record_id,
            operation,
            format!(r#"{{"id":"{}","display_name":"test"}}"#, record_id),
            device_id,
        ],
    )
    .unwrap();
}

// ──────────────────────── Tests ────────────────────────

#[test]
fn collect_empty_delta_when_no_changes() {
    let (pool, device_id) = setup();
    let engine = DeltaSyncEngine::new(&pool, device_id);

    let delta = engine.collect_local_delta().unwrap();
    assert!(delta.operations.is_empty());
    assert_eq!(delta.checksum.len(), 64);
}

#[test]
fn collect_delta_picks_up_unsynced_metadata() {
    let (pool, device_id) = setup();

    // Manually insert some unsynced metadata
    insert_sync_metadata(&pool, "persons", "p-001", "INSERT", &device_id);
    insert_sync_metadata(&pool, "persons", "p-002", "INSERT", &device_id);
    insert_sync_metadata(&pool, "persons", "p-001", "UPDATE", &device_id);

    let engine = DeltaSyncEngine::new(&pool, device_id);
    let delta = engine.collect_local_delta().unwrap();

    assert_eq!(delta.operations.len(), 3);
}

#[test]
fn mark_synced_excludes_from_next_collect() {
    let (pool, device_id) = setup();

    insert_sync_metadata(&pool, "persons", "p-001", "INSERT", &device_id);
    insert_sync_metadata(&pool, "persons", "p-002", "INSERT", &device_id);

    let engine = DeltaSyncEngine::new(&pool, device_id.clone());

    // First collect: 2 operations
    let delta1 = engine.collect_local_delta().unwrap();
    assert_eq!(delta1.operations.len(), 2);

    // Mark all as synced (up to id=2)
    engine.mark_synced(2).unwrap();

    // Second collect: 0 operations
    let delta2 = engine.collect_local_delta().unwrap();
    assert!(delta2.operations.is_empty());
}

#[test]
fn mark_synced_partial() {
    let (pool, device_id) = setup();

    insert_sync_metadata(&pool, "persons", "p-001", "INSERT", &device_id);
    insert_sync_metadata(&pool, "persons", "p-002", "INSERT", &device_id);
    insert_sync_metadata(&pool, "persons", "p-003", "INSERT", &device_id);

    let engine = DeltaSyncEngine::new(&pool, device_id.clone());

    // Only mark first one as synced
    engine.mark_synced(1).unwrap();

    let delta = engine.collect_local_delta().unwrap();
    assert_eq!(delta.operations.len(), 2); // p-002 and p-003 remain
}

#[test]
fn delta_checksum_matches_operations() {
    let (pool, device_id) = setup();

    insert_sync_metadata(&pool, "persons", "p-001", "INSERT", &device_id);

    let engine = DeltaSyncEngine::new(&pool, device_id);
    let delta = engine.collect_local_delta().unwrap();

    // Checksum should match recalculated value
    let recalculated = app_lib::sync::Delta::calculate_checksum(&delta.operations);
    assert_eq!(delta.checksum, recalculated);
}

#[test]
fn device_id_is_consistent() {
    let (pool, _) = setup();
    let conn = pool.0.lock().unwrap();
    let id1 = DeltaSyncEngine::get_device_id(&conn).unwrap();
    let id2 = DeltaSyncEngine::get_device_id(&conn).unwrap();
    assert_eq!(id1, id2);
    assert_eq!(id1.len(), 32); // hex(randomblob(16)) = 32 chars
}

#[test]
fn apply_remote_delta_inserts_person() {
    let (pool, device_id) = setup();
    let engine = DeltaSyncEngine::new(&pool, device_id);

    let delta = app_lib::sync::Delta {
        id: 1,
        operations: vec![app_lib::sync::Operation {
            table_name: "persons".into(),
            record_id: "remote-p1".into(),
            op_type: app_lib::sync::OperationType::Insert,
            data: Some(serde_json::json!({
                "id": "remote-p1",
                "display_name": "Remote Person",
                "email": "rp@test.com",
                "role": "tester",
                "note": "",
                "is_active": 1,
                "created_at": "2026-01-01T00:00:00Z",
                "updated_at": "2026-01-01T00:00:00Z"
            })),
            version: 1,
        }],
        device_id: "remote-device".into(),
        vector_clock: app_lib::sync::VectorClock::new("remote-device".into()),
        created_at: "2026-01-01T00:00:00Z".into(),
        checksum: "ignored".into(),
    };

    engine.apply_delta(&delta).unwrap();

    // Verify person exists
    let conn = pool.0.lock().unwrap();
    let name: String = conn
        .query_row(
            "SELECT display_name FROM persons WHERE id = 'remote-p1'",
            [],
            |row: &rusqlite::Row<'_>| row.get(0),
        )
        .unwrap();
    assert_eq!(name, "Remote Person");
}

#[test]
fn apply_remote_delta_deletes_person() {
    let (pool, device_id) = setup();

    // Insert a person first
    insert_person(&pool, "del-p1", "To Be Deleted");

    let engine = DeltaSyncEngine::new(&pool, device_id);

    let delta = app_lib::sync::Delta {
        id: 2,
        operations: vec![app_lib::sync::Operation {
            table_name: "persons".into(),
            record_id: "del-p1".into(),
            op_type: app_lib::sync::OperationType::Delete,
            data: None,
            version: 1,
        }],
        device_id: "remote-device".into(),
        vector_clock: app_lib::sync::VectorClock::new("remote-device".into()),
        created_at: "2026-01-01T00:00:00Z".into(),
        checksum: "ignored".into(),
    };

    engine.apply_delta(&delta).unwrap();

    // Verify person is gone
    let conn = pool.0.lock().unwrap();
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM persons WHERE id = 'del-p1'",
            [],
            |row: &rusqlite::Row<'_>| row.get(0),
        )
        .unwrap();
    assert_eq!(count, 0);
}

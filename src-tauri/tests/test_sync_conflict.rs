//! Delta apply (all table upserts) and vector clock update integration tests

use app_lib::error::AppError;
use app_lib::infra::db::init_test_db;
use app_lib::sync::{Delta, DeltaSyncEngine, Operation, OperationType, VectorClock};
use serde_json::json;

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

fn seed_person_and_partner(pool: &app_lib::infra::DbPool) {
    let conn = pool.0.lock().unwrap();
    conn.execute(
        "INSERT INTO persons (id, display_name, email, role, note, is_active, created_at, updated_at, _version)
         VALUES ('person-1', 'Alice', 'a@t.com', 'dev', '', 1, datetime('now'), datetime('now'), 1)",
        [],
    ).unwrap();
    conn.execute(
        "INSERT INTO partners (id, name, note, is_active, created_at, updated_at, _version)
         VALUES ('partner-1', 'Corp', '', 1, datetime('now'), datetime('now'), 1)",
        [],
    )
    .unwrap();
}

fn make_delta(operations: Vec<Operation>) -> Delta {
    let checksum = Delta::calculate_checksum(&operations);
    Delta {
        id: 1,
        operations,
        device_id: "remote-device".into(),
        vector_clock: VectorClock::new("remote-device".into()),
        created_at: "2026-01-01T00:00:00Z".into(),
        checksum,
    }
}

fn count_table(pool: &app_lib::infra::DbPool, table: &str) -> i64 {
    let conn = pool.0.lock().unwrap();
    conn.query_row(
        &format!("SELECT COUNT(*) FROM {}", table),
        [],
        |row: &rusqlite::Row<'_>| row.get(0),
    )
    .unwrap()
}

// ══════════════════════════════════════════════════════════
//  upsert_person
// ══════════════════════════════════════════════════════════

#[test]
fn apply_delta_upsert_person_insert() {
    let (pool, device_id) = setup();
    let engine = DeltaSyncEngine::new(&pool, device_id);

    let delta = make_delta(vec![Operation {
        table_name: "persons".into(),
        record_id: "rp-1".into(),
        op_type: OperationType::Insert,
        data: Some(json!({
            "id": "rp-1",
            "display_name": "Remote Person",
            "email": "rp@test.com",
            "role": "tester",
            "note": "synced",
            "is_active": 1,
            "created_at": "2026-01-01T00:00:00Z",
            "updated_at": "2026-01-01T00:00:00Z"
        })),
        version: 1,
    }]);

    engine.apply_delta(&delta).unwrap();

    let conn = pool.0.lock().unwrap();
    let name: String = conn
        .query_row(
            "SELECT display_name FROM persons WHERE id = 'rp-1'",
            [],
            |r: &rusqlite::Row<'_>| r.get(0),
        )
        .unwrap();
    assert_eq!(name, "Remote Person");
}

#[test]
fn apply_delta_upsert_person_update() {
    let (pool, device_id) = setup();
    seed_person_and_partner(&pool);
    let engine = DeltaSyncEngine::new(&pool, device_id);

    let delta = make_delta(vec![Operation {
        table_name: "persons".into(),
        record_id: "person-1".into(),
        op_type: OperationType::Update,
        data: Some(json!({
            "id": "person-1",
            "display_name": "Alice Updated",
            "email": "alice-new@t.com",
            "role": "lead",
            "note": "promoted",
            "is_active": 1,
            "created_at": "2026-01-01T00:00:00Z",
            "updated_at": "2026-02-01T00:00:00Z"
        })),
        version: 2,
    }]);

    engine.apply_delta(&delta).unwrap();

    let conn = pool.0.lock().unwrap();
    let (name, email): (String, String) = conn
        .query_row(
            "SELECT display_name, email FROM persons WHERE id = 'person-1'",
            [],
            |r: &rusqlite::Row<'_>| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    assert_eq!(name, "Alice Updated");
    assert_eq!(email, "alice-new@t.com");
}

#[test]
fn apply_delta_upsert_person_stale_version_is_ignored() {
    let (pool, device_id) = setup();
    seed_person_and_partner(&pool);
    {
        let conn = pool.0.lock().unwrap();
        conn.execute(
            "UPDATE persons SET _version = 5, display_name = 'Alice Local Newer' WHERE id = 'person-1'",
            [],
        )
        .unwrap();
    }

    let engine = DeltaSyncEngine::new(&pool, device_id);
    let delta = make_delta(vec![Operation {
        table_name: "persons".into(),
        record_id: "person-1".into(),
        op_type: OperationType::Update,
        data: Some(json!({
            "id": "person-1",
            "display_name": "Alice Remote Older",
            "email": "old@remote.com",
            "role": "lead",
            "note": "should be ignored",
            "is_active": 1,
            "created_at": "2026-01-01T00:00:00Z",
            "updated_at": "2026-01-01T00:00:00Z"
        })),
        version: 4,
    }]);

    engine.apply_delta(&delta).unwrap();

    let conn = pool.0.lock().unwrap();
    let (name, version): (String, i64) = conn
        .query_row(
            "SELECT display_name, _version FROM persons WHERE id = 'person-1'",
            [],
            |r: &rusqlite::Row<'_>| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    assert_eq!(name, "Alice Local Newer");
    assert_eq!(version, 5);
}

// ══════════════════════════════════════════════════════════
//  upsert_partner
// ══════════════════════════════════════════════════════════

#[test]
fn apply_delta_upsert_partner() {
    let (pool, device_id) = setup();
    let engine = DeltaSyncEngine::new(&pool, device_id);

    let delta = make_delta(vec![Operation {
        table_name: "partners".into(),
        record_id: "rptr-1".into(),
        op_type: OperationType::Insert,
        data: Some(json!({
            "id": "rptr-1",
            "name": "Remote Corp",
            "note": "synced partner",
            "is_active": 1,
            "created_at": "2026-01-01T00:00:00Z",
            "updated_at": "2026-01-01T00:00:00Z"
        })),
        version: 1,
    }]);

    engine.apply_delta(&delta).unwrap();

    let conn = pool.0.lock().unwrap();
    let name: String = conn
        .query_row(
            "SELECT name FROM partners WHERE id = 'rptr-1'",
            [],
            |r: &rusqlite::Row<'_>| r.get(0),
        )
        .unwrap();
    assert_eq!(name, "Remote Corp");
}

// ══════════════════════════════════════════════════════════
//  upsert_project
// ══════════════════════════════════════════════════════════

#[test]
fn apply_delta_upsert_project() {
    let (pool, device_id) = setup();
    seed_person_and_partner(&pool);
    let engine = DeltaSyncEngine::new(&pool, device_id);

    let delta = make_delta(vec![Operation {
        table_name: "projects".into(),
        record_id: "rproj-1".into(),
        op_type: OperationType::Insert,
        data: Some(json!({
            "id": "rproj-1",
            "name": "Remote Project",
            "description": "synced",
            "priority": 4,
            "current_status": "BACKLOG",
            "country_code": "US",
            "partner_id": "partner-1",
            "owner_person_id": "person-1",
            "start_date": "2026-01-01",
            "due_date": "2026-12-31",
            "created_at": "2026-01-01T00:00:00Z",
            "updated_at": "2026-01-01T00:00:00Z",
            "archived_at": null
        })),
        version: 1,
    }]);

    engine.apply_delta(&delta).unwrap();

    let conn = pool.0.lock().unwrap();
    let (name, status): (String, String) = conn
        .query_row(
            "SELECT name, current_status FROM projects WHERE id = 'rproj-1'",
            [],
            |r: &rusqlite::Row<'_>| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    assert_eq!(name, "Remote Project");
    assert_eq!(status, "BACKLOG");
}

// ══════════════════════════════════════════════════════════
//  upsert_assignment
// ══════════════════════════════════════════════════════════

#[test]
fn apply_delta_upsert_assignment() {
    let (pool, device_id) = setup();
    seed_person_and_partner(&pool);

    // Need a project first
    {
        let conn = pool.0.lock().unwrap();
        conn.execute(
            "INSERT INTO projects (id, name, description, priority, current_status, country_code, partner_id, owner_person_id, created_at, updated_at, _version)
             VALUES ('proj-1', 'P1', '', 3, 'BACKLOG', 'US', 'partner-1', 'person-1', datetime('now'), datetime('now'), 1)",
            [],
        ).unwrap();
    }

    let engine = DeltaSyncEngine::new(&pool, device_id);

    let delta = make_delta(vec![Operation {
        table_name: "assignments".into(),
        record_id: "rasgn-1".into(),
        op_type: OperationType::Insert,
        data: Some(json!({
            "id": "rasgn-1",
            "project_id": "proj-1",
            "person_id": "person-1",
            "role": "developer",
            "start_at": "2026-01-01T00:00:00Z",
            "end_at": null,
            "created_at": "2026-01-01T00:00:00Z"
        })),
        version: 1,
    }]);

    engine.apply_delta(&delta).unwrap();
    assert_eq!(count_table(&pool, "assignments"), 1);
}

// ══════════════════════════════════════════════════════════
//  upsert_status_history
// ══════════════════════════════════════════════════════════

#[test]
fn apply_delta_upsert_status_history() {
    let (pool, device_id) = setup();
    seed_person_and_partner(&pool);

    {
        let conn = pool.0.lock().unwrap();
        conn.execute(
            "INSERT INTO projects (id, name, description, priority, current_status, country_code, partner_id, owner_person_id, created_at, updated_at, _version)
             VALUES ('proj-sh', 'SH', '', 3, 'BACKLOG', 'US', 'partner-1', 'person-1', datetime('now'), datetime('now'), 1)",
            [],
        ).unwrap();
    }

    let engine = DeltaSyncEngine::new(&pool, device_id);

    let delta = make_delta(vec![Operation {
        table_name: "status_history".into(),
        record_id: "rsh-1".into(),
        op_type: OperationType::Insert,
        data: Some(json!({
            "id": "rsh-1",
            "project_id": "proj-sh",
            "from_status": null,
            "to_status": "BACKLOG",
            "changed_at": "2026-01-01T00:00:00Z",
            "changed_by_person_id": "person-1",
            "note": "created"
        })),
        version: 1,
    }]);

    engine.apply_delta(&delta).unwrap();
    assert_eq!(count_table(&pool, "status_history"), 1);
}

#[test]
fn apply_delta_upsert_project_tags() {
    let (pool, device_id) = setup();
    seed_person_and_partner(&pool);

    {
        let conn = pool.0.lock().unwrap();
        conn.execute(
            "INSERT INTO projects (id, name, description, priority, current_status, country_code, partner_id, owner_person_id, created_at, updated_at, _version)
             VALUES ('proj-tag', 'TagProj', '', 3, 'BACKLOG', 'US', 'partner-1', 'person-1', datetime('now'), datetime('now'), 1)",
            [],
        )
        .unwrap();
    }

    let engine = DeltaSyncEngine::new(&pool, device_id);
    let delta = make_delta(vec![Operation {
        table_name: "project_tags".into(),
        record_id: "proj-tag:urgent".into(),
        op_type: OperationType::Insert,
        data: Some(json!({
            "project_id": "proj-tag",
            "tag": "urgent",
            "created_at": "2026-01-01T00:00:00Z"
        })),
        version: 1,
    }]);

    engine.apply_delta(&delta).unwrap();
    assert_eq!(count_table(&pool, "project_tags"), 1);
}

#[test]
fn apply_delta_upsert_project_comments() {
    let (pool, device_id) = setup();
    seed_person_and_partner(&pool);

    {
        let conn = pool.0.lock().unwrap();
        conn.execute(
            "INSERT INTO projects (id, name, description, priority, current_status, country_code, partner_id, owner_person_id, created_at, updated_at, _version)
             VALUES ('proj-comment', 'CommentProj', '', 3, 'BACKLOG', 'US', 'partner-1', 'person-1', datetime('now'), datetime('now'), 1)",
            [],
        )
        .unwrap();
    }

    let engine = DeltaSyncEngine::new(&pool, device_id);
    let delta = make_delta(vec![Operation {
        table_name: "project_comments".into(),
        record_id: "comment-1".into(),
        op_type: OperationType::Insert,
        data: Some(json!({
            "id": "comment-1",
            "project_id": "proj-comment",
            "person_id": "person-1",
            "content": "{\"type\":\"doc\",\"content\":[]}",
            "is_pinned": 1,
            "created_at": "2026-01-01T00:00:00Z",
            "updated_at": "2026-01-01T00:00:00Z"
        })),
        version: 1,
    }]);

    engine.apply_delta(&delta).unwrap();
    assert_eq!(count_table(&pool, "project_comments"), 1);
}

// ══════════════════════════════════════════════════════════
//  delete operations for all tables
// ══════════════════════════════════════════════════════════

#[test]
fn apply_delta_delete_partner() {
    let (pool, device_id) = setup();
    seed_person_and_partner(&pool);
    assert_eq!(count_table(&pool, "partners"), 1);

    let engine = DeltaSyncEngine::new(&pool, device_id);
    let delta = make_delta(vec![Operation {
        table_name: "partners".into(),
        record_id: "partner-1".into(),
        op_type: OperationType::Delete,
        data: None,
        version: 1,
    }]);

    engine.apply_delta(&delta).unwrap();
    assert_eq!(count_table(&pool, "partners"), 0);
}

#[test]
fn apply_delta_delete_nonexistent_is_ok() {
    let (pool, device_id) = setup();
    let engine = DeltaSyncEngine::new(&pool, device_id);

    let delta = make_delta(vec![Operation {
        table_name: "persons".into(),
        record_id: "ghost-id".into(),
        op_type: OperationType::Delete,
        data: None,
        version: 1,
    }]);

    // Should not error even if the record doesn't exist
    engine.apply_delta(&delta).unwrap();
}

#[test]
fn apply_delta_delete_project_tag_by_composite_record_id() {
    let (pool, device_id) = setup();
    seed_person_and_partner(&pool);
    {
        let conn = pool.0.lock().unwrap();
        conn.execute(
            "INSERT INTO projects (id, name, description, priority, current_status, country_code, partner_id, owner_person_id, created_at, updated_at, _version)
             VALUES ('proj-del-tag', 'TagDelProj', '', 3, 'BACKLOG', 'US', 'partner-1', 'person-1', datetime('now'), datetime('now'), 1)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO project_tags (project_id, tag, created_at) VALUES ('proj-del-tag', 'urgent', datetime('now'))",
            [],
        )
        .unwrap();
    }

    let engine = DeltaSyncEngine::new(&pool, device_id);
    let delta = make_delta(vec![Operation {
        table_name: "project_tags".into(),
        record_id: "proj-del-tag:urgent".into(),
        op_type: OperationType::Delete,
        data: None,
        version: 1,
    }]);

    engine.apply_delta(&delta).unwrap();
    assert_eq!(count_table(&pool, "project_tags"), 0);
}

// ══════════════════════════════════════════════════════════
//  multiple operations in single delta
// ══════════════════════════════════════════════════════════

#[test]
fn apply_delta_batch_operations() {
    let (pool, device_id) = setup();
    let engine = DeltaSyncEngine::new(&pool, device_id);

    let delta = make_delta(vec![
        Operation {
            table_name: "persons".into(),
            record_id: "batch-p1".into(),
            op_type: OperationType::Insert,
            data: Some(json!({
                "id": "batch-p1", "display_name": "P1", "email": "", "role": "",
                "note": "", "is_active": 1, "created_at": "2026-01-01T00:00:00Z",
                "updated_at": "2026-01-01T00:00:00Z"
            })),
            version: 1,
        },
        Operation {
            table_name: "persons".into(),
            record_id: "batch-p2".into(),
            op_type: OperationType::Insert,
            data: Some(json!({
                "id": "batch-p2", "display_name": "P2", "email": "", "role": "",
                "note": "", "is_active": 1, "created_at": "2026-01-01T00:00:00Z",
                "updated_at": "2026-01-01T00:00:00Z"
            })),
            version: 1,
        },
        Operation {
            table_name: "partners".into(),
            record_id: "batch-ptr1".into(),
            op_type: OperationType::Insert,
            data: Some(json!({
                "id": "batch-ptr1", "name": "BatchCorp", "note": "",
                "is_active": 1, "created_at": "2026-01-01T00:00:00Z",
                "updated_at": "2026-01-01T00:00:00Z"
            })),
            version: 1,
        },
    ]);

    engine.apply_delta(&delta).unwrap();
    assert_eq!(count_table(&pool, "persons"), 2);
    assert_eq!(count_table(&pool, "partners"), 1);
}

// ══════════════════════════════════════════════════════════
//  vector clock update after apply
// ══════════════════════════════════════════════════════════

#[test]
fn apply_delta_updates_vector_clock() {
    let (pool, device_id) = setup();
    let engine = DeltaSyncEngine::new(&pool, device_id);

    let mut vc = VectorClock::new("remote-device".into());
    vc.increment("remote-device");
    vc.increment("remote-device");

    let delta = Delta {
        id: 1,
        operations: vec![Operation {
            table_name: "persons".into(),
            record_id: "vc-p1".into(),
            op_type: OperationType::Insert,
            data: Some(json!({
                "id": "vc-p1", "display_name": "VCTest", "email": "", "role": "",
                "note": "", "is_active": 1, "created_at": "2026-01-01T00:00:00Z",
                "updated_at": "2026-01-01T00:00:00Z"
            })),
            version: 1,
        }],
        device_id: "remote-device".into(),
        vector_clock: vc,
        created_at: "2026-01-01T00:00:00Z".into(),
        checksum: "ignored".into(),
    };

    engine.apply_delta(&delta).unwrap();

    // Verify global vector clock was updated
    let conn = pool.0.lock().unwrap();
    let clock_val: i64 = conn.query_row(
        "SELECT clock_value FROM vector_clocks WHERE table_name = '_global' AND record_id = '_global' AND device_id = 'remote-device'",
        [],
        |r: &rusqlite::Row<'_>| r.get(0),
    ).unwrap();
    assert_eq!(clock_val, 2); // incremented twice
}

// ══════════════════════════════════════════════════════════
//  trigger + engine roundtrip
// ══════════════════════════════════════════════════════════

#[test]
fn trigger_generates_metadata_then_engine_collects_it() {
    let (pool, device_id) = setup();

    // Enable sync
    {
        let conn = pool.0.lock().unwrap();
        conn.execute(
            "UPDATE sync_config SET value = '1' WHERE key = 'sync_enabled'",
            [],
        )
        .unwrap();
    }

    // Insert a person (trigger fires)
    {
        let conn = pool.0.lock().unwrap();
        conn.execute(
            "INSERT INTO persons (id, display_name, email, role, note, is_active, created_at, updated_at, _version)
             VALUES ('rt-p1', 'Roundtrip', '', '', '', 1, datetime('now'), datetime('now'), 1)",
            [],
        ).unwrap();
    }

    // Collect delta via engine
    let engine = DeltaSyncEngine::new(&pool, device_id);
    let collected = engine.collect_local_delta().unwrap();
    let delta = collected.delta;

    assert_eq!(delta.operations.len(), 1);
    assert_eq!(delta.operations[0].table_name, "persons");
    assert_eq!(delta.operations[0].record_id, "rt-p1");
    assert!(delta.operations[0].data.is_some());

    let data = delta.operations[0].data.as_ref().unwrap();
    assert_eq!(data["display_name"], "Roundtrip");
}

#[test]
fn full_roundtrip_trigger_collect_compress_decompress() {
    let (pool, device_id) = setup();

    // Enable sync
    {
        let conn = pool.0.lock().unwrap();
        conn.execute(
            "UPDATE sync_config SET value = '1' WHERE key = 'sync_enabled'",
            [],
        )
        .unwrap();
    }

    // Insert data
    {
        let conn = pool.0.lock().unwrap();
        conn.execute(
            "INSERT INTO persons (id, display_name, email, role, note, is_active, created_at, updated_at, _version)
             VALUES ('full-p1', 'FullTrip', 'ft@t.com', 'dev', 'note', 1, datetime('now'), datetime('now'), 1)",
            [],
        ).unwrap();
    }

    // Collect → compress → decompress
    let engine = DeltaSyncEngine::new(&pool, device_id);
    let collected = engine.collect_local_delta().unwrap();
    let delta = collected.delta;
    let compressed = delta.compress().unwrap();
    let decompressed = Delta::decompress(&compressed).unwrap();

    assert_eq!(decompressed.operations.len(), 1);
    assert_eq!(decompressed.operations[0].record_id, "full-p1");
    assert_eq!(decompressed.checksum, delta.checksum);
}

// ══════════════════════════════════════════════════════════
//  边界: unknown table, data=None
// ══════════════════════════════════════════════════════════

#[test]
fn apply_delta_upsert_unknown_table_silently_skips() {
    let (pool, device_id) = setup();
    let engine = DeltaSyncEngine::new(&pool, device_id);

    let delta = make_delta(vec![Operation {
        table_name: "nonexistent_table".into(),
        record_id: "x".into(),
        op_type: OperationType::Insert,
        data: Some(json!({"id": "x", "name": "test"})),
        version: 1,
    }]);

    // Should succeed (unknown table is silently skipped in upsert)
    engine.apply_delta(&delta).unwrap();
}

#[test]
fn apply_delta_upsert_with_none_data_skips() {
    let (pool, device_id) = setup();
    let engine = DeltaSyncEngine::new(&pool, device_id);

    let delta = make_delta(vec![Operation {
        table_name: "persons".into(),
        record_id: "no-data".into(),
        op_type: OperationType::Insert,
        data: None, // No data → should skip
        version: 1,
    }]);

    engine.apply_delta(&delta).unwrap();
    // Person should NOT exist since data was None
    assert_eq!(count_table(&pool, "persons"), 0);
}

// ══════════════════════════════════════════════════════════
//  AppError 测试
// ══════════════════════════════════════════════════════════

#[test]
fn error_codes_mapping() {
    assert_eq!(AppError::Db("x".into()).code(), "DB_ERROR");
    assert_eq!(AppError::Validation("x".into()).code(), "VALIDATION_ERROR");
    assert_eq!(AppError::NotFound("x".into()).code(), "NOT_FOUND");
    assert_eq!(AppError::Conflict("x".into()).code(), "CONFLICT");
    assert_eq!(AppError::PartnerImmutable.code(), "PARTNER_IMMUTABLE");
    assert_eq!(
        AppError::InvalidStatusTransition("x".into()).code(),
        "INVALID_STATUS_TRANSITION"
    );
    assert_eq!(AppError::NoteRequired.code(), "NOTE_REQUIRED");
    assert_eq!(
        AppError::AssignmentAlreadyActive.code(),
        "ASSIGNMENT_ALREADY_ACTIVE"
    );
    assert_eq!(
        AppError::AssignmentNotActive.code(),
        "ASSIGNMENT_NOT_ACTIVE"
    );
}

#[test]
fn error_to_serde_dto_structure() {
    let err = AppError::Validation("bad input".into());
    let dto = err.to_serde();
    assert_eq!(dto.code, "VALIDATION_ERROR");
    assert!(dto.message.contains("bad input"));
    assert!(dto.details.is_none());
}

#[test]
fn error_serializes_to_json() {
    let err = AppError::NotFound("project xyz".into());
    let json = serde_json::to_value(&err).unwrap();
    assert_eq!(json["code"], "NOT_FOUND");
    assert!(json["message"].as_str().unwrap().contains("project xyz"));
}

#[test]
fn error_from_rusqlite() {
    let rusqlite_err = rusqlite::Error::QueryReturnedNoRows;
    let app_err: AppError = rusqlite_err.into();
    assert_eq!(app_err.code(), "DB_ERROR");
}

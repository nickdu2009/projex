//! Integration tests for `wipe_business_data` use case.

use app_lib::app::{
    assignment_add_member, partner_create, person_create, project_create, wipe_business_data,
    AssignmentAddReq, PartnerCreateReq, PersonCreateReq, ProjectCreateReq,
};
use app_lib::infra::db::init_test_db;

// ── helpers ────────────────────────────────────────────────────────────────────

fn set_sync_enabled(pool: &app_lib::infra::db::DbPool, enabled: bool) {
    let conn = pool.0.lock().unwrap();
    conn.execute(
        "INSERT OR REPLACE INTO sync_config (key, value) VALUES ('sync_enabled', ?1)",
        [if enabled { "1" } else { "0" }],
    )
    .unwrap();
}

fn count_rows(pool: &app_lib::infra::db::DbPool, table: &str) -> i64 {
    let conn = pool.0.lock().unwrap();
    conn.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get(0))
        .unwrap_or(0)
}

/// Seed a minimal dataset: 1 partner, 1 person, 1 project, 1 assignment.
fn seed_minimal(
    pool: &app_lib::infra::db::DbPool,
) -> (String, String, String) {
    let partner = partner_create(
        pool,
        PartnerCreateReq {
            name: "WipeCorp".to_string(),
            note: None,
        },
    )
    .unwrap();

    let person = person_create(
        pool,
        PersonCreateReq {
            display_name: "Wipe User".to_string(),
            email: None,
            role: None,
            note: None,
        },
    )
    .unwrap();

    let project = project_create(
        pool,
        ProjectCreateReq {
            name: "Wipe Project".to_string(),
            description: None,
            priority: None,
            country_code: "US".to_string(),
            partner_id: partner.id.clone(),
            owner_person_id: person.id.clone(),
            product_name: None,
            start_date: None,
            due_date: None,
            tags: Some(vec!["wipe-tag".to_string()]),
            created_by_person_id: None,
        },
    )
    .unwrap();

    // Add an extra member to create an assignment row.
    let member = person_create(
        pool,
        PersonCreateReq {
            display_name: "Extra Member".to_string(),
            email: None,
            role: None,
            note: None,
        },
    )
    .unwrap();
    assignment_add_member(
        pool,
        AssignmentAddReq {
            project_id: project.id.clone(),
            person_id: member.id.clone(),
            role: Some("developer".to_string()),
            start_at: None,
        },
    )
    .unwrap();

    (partner.id, person.id, project.id)
}

// ── guard: sync must be enabled ────────────────────────────────────────────────

#[test]
fn wipe_fails_when_sync_disabled() {
    let pool = init_test_db();
    // sync_enabled defaults to "0" in init_test_db.
    let err = wipe_business_data(&pool).unwrap_err();
    assert_eq!(err.code(), "VALIDATION_ERROR");
    let msg = format!("{err:?}");
    assert!(msg.contains("SYNC_DISABLED"), "expected SYNC_DISABLED in: {msg}");
}

#[test]
fn wipe_fails_when_sync_explicitly_disabled() {
    let pool = init_test_db();
    set_sync_enabled(&pool, false);
    let err = wipe_business_data(&pool).unwrap_err();
    assert_eq!(err.code(), "VALIDATION_ERROR");
}

// ── successful wipe: all business tables emptied ───────────────────────────────

#[test]
fn wipe_clears_all_business_tables_when_sync_enabled() {
    let pool = init_test_db();
    set_sync_enabled(&pool, true);

    // Ensure device_id is present (required by wipe logic).
    {
        let conn = pool.0.lock().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO sync_config (key, value) VALUES ('device_id', 'test-device-wipe')",
            [],
        )
        .unwrap();
    }

    seed_minimal(&pool);

    // Verify data exists before wipe.
    assert!(count_rows(&pool, "partners") > 0, "partners should have rows before wipe");
    assert!(count_rows(&pool, "persons") > 0, "persons should have rows before wipe");
    assert!(count_rows(&pool, "projects") > 0, "projects should have rows before wipe");
    assert!(count_rows(&pool, "assignments") > 0, "assignments should have rows before wipe");
    assert!(count_rows(&pool, "project_tags") > 0, "project_tags should have rows before wipe");
    assert!(count_rows(&pool, "status_history") > 0, "status_history should have rows before wipe");

    let result = wipe_business_data(&pool).unwrap();

    // All business tables must be empty.
    assert_eq!(count_rows(&pool, "partners"), 0, "partners should be empty after wipe");
    assert_eq!(count_rows(&pool, "persons"), 0, "persons should be empty after wipe");
    assert_eq!(count_rows(&pool, "projects"), 0, "projects should be empty after wipe");
    assert_eq!(count_rows(&pool, "assignments"), 0, "assignments should be empty after wipe");
    assert_eq!(count_rows(&pool, "project_tags"), 0, "project_tags should be empty after wipe");
    assert_eq!(count_rows(&pool, "status_history"), 0, "status_history should be empty after wipe");
    assert_eq!(count_rows(&pool, "project_comments"), 0, "project_comments should be empty after wipe");

    // WipeResult counts must match.
    assert!(result.deleted_partners > 0);
    assert!(result.deleted_persons > 0);
    assert!(result.deleted_projects > 0);
    assert!(result.deleted_assignments > 0);
    assert!(result.deleted_project_tags > 0);
    assert!(result.deleted_status_history > 0);
    assert!(!result.wipe_id.is_empty(), "wipe_id must be set");
}

// ── wipe inserts WIPE_INTENT into sync_metadata ───────────────────────────────

#[test]
fn wipe_inserts_wipe_intent_control_op_into_sync_metadata() {
    let pool = init_test_db();
    set_sync_enabled(&pool, true);

    {
        let conn = pool.0.lock().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO sync_config (key, value) VALUES ('device_id', 'test-device-wipe-intent')",
            [],
        )
        .unwrap();
    }

    let result = wipe_business_data(&pool).unwrap();

    // Verify that a _control/WIPE_INTENT row exists in sync_metadata.
    let conn = pool.0.lock().unwrap();
    let (table_name, record_id, operation, data_snapshot): (String, String, String, String) = conn
        .query_row(
            "SELECT table_name, record_id, operation, data_snapshot
             FROM sync_metadata
             WHERE table_name = '_control'
             ORDER BY id DESC
             LIMIT 1",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .expect("should have a _control row in sync_metadata");

    assert_eq!(table_name, "_control");
    assert_eq!(record_id, result.wipe_id, "record_id must match wipe_id");
    assert_eq!(operation, "INSERT");

    let data: serde_json::Value = serde_json::from_str(&data_snapshot).unwrap();
    assert_eq!(data["type"], "WIPE_INTENT");
    assert_eq!(data["wipe_id"], result.wipe_id);
    assert_eq!(data["reason"], "user_initiated");
}

// ── wipe on empty DB still succeeds ───────────────────────────────────────────

#[test]
fn wipe_on_empty_db_succeeds_with_zero_counts() {
    let pool = init_test_db();
    set_sync_enabled(&pool, true);

    {
        let conn = pool.0.lock().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO sync_config (key, value) VALUES ('device_id', 'test-device-empty')",
            [],
        )
        .unwrap();
    }

    let result = wipe_business_data(&pool).unwrap();

    assert_eq!(result.deleted_partners, 0);
    assert_eq!(result.deleted_persons, 0);
    assert_eq!(result.deleted_projects, 0);
    assert_eq!(result.deleted_assignments, 0);
    assert_eq!(result.deleted_project_tags, 0);
    assert_eq!(result.deleted_status_history, 0);
    assert_eq!(result.deleted_project_comments, 0);
    assert!(!result.wipe_id.is_empty());
}

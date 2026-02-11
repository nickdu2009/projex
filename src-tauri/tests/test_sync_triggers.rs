//! Sync metadata trigger integration tests
//! 验证当 sync_enabled='1' 时，业务表的 INSERT/UPDATE/DELETE 操作自动生成 sync_metadata 记录

use app_lib::infra::db::init_test_db;

// ──────────────────────── Helper ────────────────────────

fn setup_with_sync_enabled() -> (app_lib::infra::DbPool, String) {
    let pool = init_test_db();
    let device_id = {
        let conn = pool.0.lock().unwrap();
        // Enable sync
        conn.execute(
            "UPDATE sync_config SET value = '1' WHERE key = 'sync_enabled'",
            [],
        )
        .unwrap();
        conn.query_row(
            "SELECT value FROM sync_config WHERE key = 'device_id'",
            [],
            |row: &rusqlite::Row<'_>| row.get::<_, String>(0),
        )
        .unwrap()
    };
    (pool, device_id)
}

fn setup_with_sync_disabled() -> app_lib::infra::DbPool {
    let pool = init_test_db();
    // sync_enabled defaults to '0', no need to change
    pool
}

fn count_sync_metadata(pool: &app_lib::infra::DbPool) -> i64 {
    let conn = pool.0.lock().unwrap();
    conn.query_row(
        "SELECT COUNT(*) FROM sync_metadata",
        [],
        |row: &rusqlite::Row<'_>| row.get(0),
    )
    .unwrap()
}

fn get_last_sync_metadata(
    pool: &app_lib::infra::DbPool,
) -> (String, String, String, Option<String>) {
    let conn = pool.0.lock().unwrap();
    conn.query_row(
        "SELECT table_name, record_id, operation, data_snapshot FROM sync_metadata ORDER BY id DESC LIMIT 1",
        [],
        |row: &rusqlite::Row<'_>| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, Option<String>>(3)?,
            ))
        },
    )
    .unwrap()
}

/// Insert prerequisite data: a person and a partner (needed for FK constraints on projects)
fn seed_person_and_partner(pool: &app_lib::infra::DbPool) {
    let conn = pool.0.lock().unwrap();
    conn.execute(
        "INSERT INTO persons (id, display_name, email, role, note, is_active, created_at, updated_at, _version)
         VALUES ('owner-1', 'Owner', 'owner@test.com', 'PM', '', 1, datetime('now'), datetime('now'), 1)",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO partners (id, name, note, is_active, created_at, updated_at, _version)
         VALUES ('partner-1', 'Acme Corp', '', 1, datetime('now'), datetime('now'), 1)",
        [],
    )
    .unwrap();
}

// ══════════════════════════════════════════════════════════
//  persons 触发器测试
// ══════════════════════════════════════════════════════════

#[test]
fn trigger_persons_insert_generates_metadata() {
    let (pool, device_id) = setup_with_sync_enabled();

    {
        let conn = pool.0.lock().unwrap();
        conn.execute(
            "INSERT INTO persons (id, display_name, email, role, note, is_active, created_at, updated_at, _version)
             VALUES ('p-trigger-1', 'Alice', 'alice@test.com', 'dev', 'note', 1, datetime('now'), datetime('now'), 1)",
            [],
        )
        .unwrap();
    }

    assert_eq!(count_sync_metadata(&pool), 1);
    let (table, record_id, op, snapshot) = get_last_sync_metadata(&pool);
    assert_eq!(table, "persons");
    assert_eq!(record_id, "p-trigger-1");
    assert_eq!(op, "INSERT");

    // Verify data_snapshot contains key fields as JSON
    let snapshot = snapshot.expect("INSERT should have data_snapshot");
    let json: serde_json::Value = serde_json::from_str(&snapshot).unwrap();
    assert_eq!(json["id"], "p-trigger-1");
    assert_eq!(json["display_name"], "Alice");
    assert_eq!(json["email"], "alice@test.com");
    assert_eq!(json["role"], "dev");

    // Verify device_id is correctly populated
    let conn = pool.0.lock().unwrap();
    let meta_device: String = conn
        .query_row(
            "SELECT device_id FROM sync_metadata WHERE id = 1",
            [],
            |row: &rusqlite::Row<'_>| row.get(0),
        )
        .unwrap();
    assert_eq!(meta_device, device_id);
}

#[test]
fn trigger_persons_update_generates_metadata() {
    let (pool, _device_id) = setup_with_sync_enabled();

    {
        let conn = pool.0.lock().unwrap();
        conn.execute(
            "INSERT INTO persons (id, display_name, email, role, note, is_active, created_at, updated_at, _version)
             VALUES ('p-trigger-2', 'Bob', 'bob@test.com', 'qa', '', 1, datetime('now'), datetime('now'), 1)",
            [],
        )
        .unwrap();
    }
    assert_eq!(count_sync_metadata(&pool), 1); // INSERT metadata

    {
        let conn = pool.0.lock().unwrap();
        conn.execute(
            "UPDATE persons SET display_name = 'Bob Updated', _version = 2 WHERE id = 'p-trigger-2'",
            [],
        )
        .unwrap();
    }
    assert_eq!(count_sync_metadata(&pool), 2); // INSERT + UPDATE

    let (table, record_id, op, snapshot) = get_last_sync_metadata(&pool);
    assert_eq!(table, "persons");
    assert_eq!(record_id, "p-trigger-2");
    assert_eq!(op, "UPDATE");

    let snapshot = snapshot.expect("UPDATE should have data_snapshot");
    let json: serde_json::Value = serde_json::from_str(&snapshot).unwrap();
    assert_eq!(json["display_name"], "Bob Updated");
    assert_eq!(json["_version"], 2);
}

#[test]
fn trigger_persons_delete_generates_metadata() {
    let (pool, _device_id) = setup_with_sync_enabled();

    {
        let conn = pool.0.lock().unwrap();
        conn.execute(
            "INSERT INTO persons (id, display_name, email, role, note, is_active, created_at, updated_at, _version)
             VALUES ('p-trigger-3', 'Charlie', '', '', '', 1, datetime('now'), datetime('now'), 1)",
            [],
        )
        .unwrap();
    }
    assert_eq!(count_sync_metadata(&pool), 1);

    {
        let conn = pool.0.lock().unwrap();
        conn.execute("DELETE FROM persons WHERE id = 'p-trigger-3'", [])
            .unwrap();
    }
    assert_eq!(count_sync_metadata(&pool), 2);

    let (table, record_id, op, snapshot) = get_last_sync_metadata(&pool);
    assert_eq!(table, "persons");
    assert_eq!(record_id, "p-trigger-3");
    assert_eq!(op, "DELETE");
    assert!(snapshot.is_none(), "DELETE should have NULL data_snapshot");
}

// ══════════════════════════════════════════════════════════
//  sync_enabled='0' 时不应生成 metadata
// ══════════════════════════════════════════════════════════

#[test]
fn no_metadata_when_sync_disabled() {
    let pool = setup_with_sync_disabled();

    {
        let conn = pool.0.lock().unwrap();
        conn.execute(
            "INSERT INTO persons (id, display_name, email, role, note, is_active, created_at, updated_at, _version)
             VALUES ('p-no-sync', 'NoSync', '', '', '', 1, datetime('now'), datetime('now'), 1)",
            [],
        )
        .unwrap();
        conn.execute(
            "UPDATE persons SET display_name = 'NoSync Updated' WHERE id = 'p-no-sync'",
            [],
        )
        .unwrap();
        conn.execute("DELETE FROM persons WHERE id = 'p-no-sync'", [])
            .unwrap();
    }

    assert_eq!(
        count_sync_metadata(&pool),
        0,
        "No metadata should be created when sync is disabled"
    );
}

// ══════════════════════════════════════════════════════════
//  partners 触发器测试
// ══════════════════════════════════════════════════════════

#[test]
fn trigger_partners_insert_update_delete() {
    let (pool, _device_id) = setup_with_sync_enabled();

    // INSERT
    {
        let conn = pool.0.lock().unwrap();
        conn.execute(
            "INSERT INTO partners (id, name, note, is_active, created_at, updated_at, _version)
             VALUES ('ptr-1', 'TestPartner', 'note', 1, datetime('now'), datetime('now'), 1)",
            [],
        )
        .unwrap();
    }
    assert_eq!(count_sync_metadata(&pool), 1);
    let (table, _, op, _) = get_last_sync_metadata(&pool);
    assert_eq!(table, "partners");
    assert_eq!(op, "INSERT");

    // UPDATE
    {
        let conn = pool.0.lock().unwrap();
        conn.execute(
            "UPDATE partners SET name = 'Updated Partner', _version = 2 WHERE id = 'ptr-1'",
            [],
        )
        .unwrap();
    }
    assert_eq!(count_sync_metadata(&pool), 2);
    let (_, _, op, snapshot) = get_last_sync_metadata(&pool);
    assert_eq!(op, "UPDATE");
    let json: serde_json::Value = serde_json::from_str(&snapshot.unwrap()).unwrap();
    assert_eq!(json["name"], "Updated Partner");

    // DELETE
    {
        let conn = pool.0.lock().unwrap();
        conn.execute("DELETE FROM partners WHERE id = 'ptr-1'", [])
            .unwrap();
    }
    assert_eq!(count_sync_metadata(&pool), 3);
    let (_, _, op, snapshot) = get_last_sync_metadata(&pool);
    assert_eq!(op, "DELETE");
    assert!(snapshot.is_none());
}

// ══════════════════════════════════════════════════════════
//  projects 触发器测试
// ══════════════════════════════════════════════════════════

#[test]
fn trigger_projects_insert_update_delete() {
    let (pool, _device_id) = setup_with_sync_enabled();

    // Seed FK dependencies (persons + partners triggers will fire too)
    seed_person_and_partner(&pool);
    let base_count = count_sync_metadata(&pool); // should be 2 (person insert + partner insert)

    // INSERT project
    {
        let conn = pool.0.lock().unwrap();
        conn.execute(
            "INSERT INTO projects (id, name, description, priority, current_status, country_code, partner_id, owner_person_id, created_at, updated_at, _version)
             VALUES ('proj-1', 'Test Project', 'desc', 3, 'open', 'CN', 'partner-1', 'owner-1', datetime('now'), datetime('now'), 1)",
            [],
        )
        .unwrap();
    }
    assert_eq!(count_sync_metadata(&pool), base_count + 1);
    let (table, record_id, op, snapshot) = get_last_sync_metadata(&pool);
    assert_eq!(table, "projects");
    assert_eq!(record_id, "proj-1");
    assert_eq!(op, "INSERT");
    let json: serde_json::Value = serde_json::from_str(&snapshot.unwrap()).unwrap();
    assert_eq!(json["name"], "Test Project");
    assert_eq!(json["country_code"], "CN");
    assert_eq!(json["partner_id"], "partner-1");

    // UPDATE project
    {
        let conn = pool.0.lock().unwrap();
        conn.execute(
            "UPDATE projects SET name = 'Updated Project', _version = 2 WHERE id = 'proj-1'",
            [],
        )
        .unwrap();
    }
    assert_eq!(count_sync_metadata(&pool), base_count + 2);
    let (_, _, op, _) = get_last_sync_metadata(&pool);
    assert_eq!(op, "UPDATE");

    // DELETE project
    {
        let conn = pool.0.lock().unwrap();
        conn.execute("DELETE FROM projects WHERE id = 'proj-1'", [])
            .unwrap();
    }
    assert_eq!(count_sync_metadata(&pool), base_count + 3);
    let (_, _, op, snapshot) = get_last_sync_metadata(&pool);
    assert_eq!(op, "DELETE");
    assert!(snapshot.is_none());
}

// ══════════════════════════════════════════════════════════
//  assignments 触发器测试
// ══════════════════════════════════════════════════════════

#[test]
fn trigger_assignments_insert_update_delete() {
    let (pool, _device_id) = setup_with_sync_enabled();

    // Seed FK dependencies
    seed_person_and_partner(&pool);
    {
        let conn = pool.0.lock().unwrap();
        conn.execute(
            "INSERT INTO projects (id, name, description, priority, current_status, country_code, partner_id, owner_person_id, created_at, updated_at, _version)
             VALUES ('proj-a', 'Proj A', '', 3, 'open', 'US', 'partner-1', 'owner-1', datetime('now'), datetime('now'), 1)",
            [],
        )
        .unwrap();
    }
    let base_count = count_sync_metadata(&pool);

    // INSERT assignment
    {
        let conn = pool.0.lock().unwrap();
        conn.execute(
            "INSERT INTO assignments (id, project_id, person_id, role, start_at, created_at, _version)
             VALUES ('asgn-1', 'proj-a', 'owner-1', 'lead', datetime('now'), datetime('now'), 1)",
            [],
        )
        .unwrap();
    }
    assert_eq!(count_sync_metadata(&pool), base_count + 1);
    let (table, _, op, snapshot) = get_last_sync_metadata(&pool);
    assert_eq!(table, "assignments");
    assert_eq!(op, "INSERT");
    let json: serde_json::Value = serde_json::from_str(&snapshot.unwrap()).unwrap();
    assert_eq!(json["project_id"], "proj-a");
    assert_eq!(json["person_id"], "owner-1");
    assert_eq!(json["role"], "lead");

    // UPDATE assignment
    {
        let conn = pool.0.lock().unwrap();
        conn.execute(
            "UPDATE assignments SET role = 'member', _version = 2 WHERE id = 'asgn-1'",
            [],
        )
        .unwrap();
    }
    assert_eq!(count_sync_metadata(&pool), base_count + 2);
    let (_, _, op, snapshot) = get_last_sync_metadata(&pool);
    assert_eq!(op, "UPDATE");
    let json: serde_json::Value = serde_json::from_str(&snapshot.unwrap()).unwrap();
    assert_eq!(json["role"], "member");

    // DELETE assignment
    {
        let conn = pool.0.lock().unwrap();
        conn.execute("DELETE FROM assignments WHERE id = 'asgn-1'", [])
            .unwrap();
    }
    assert_eq!(count_sync_metadata(&pool), base_count + 3);
    let (_, _, op, snapshot) = get_last_sync_metadata(&pool);
    assert_eq!(op, "DELETE");
    assert!(snapshot.is_none());
}

// ══════════════════════════════════════════════════════════
//  status_history 触发器测试
// ══════════════════════════════════════════════════════════

#[test]
fn trigger_status_history_insert_delete() {
    let (pool, _device_id) = setup_with_sync_enabled();

    seed_person_and_partner(&pool);
    {
        let conn = pool.0.lock().unwrap();
        conn.execute(
            "INSERT INTO projects (id, name, description, priority, current_status, country_code, partner_id, owner_person_id, created_at, updated_at, _version)
             VALUES ('proj-sh', 'SH Proj', '', 3, 'open', 'US', 'partner-1', 'owner-1', datetime('now'), datetime('now'), 1)",
            [],
        )
        .unwrap();
    }
    let base_count = count_sync_metadata(&pool);

    // INSERT status_history
    {
        let conn = pool.0.lock().unwrap();
        conn.execute(
            "INSERT INTO status_history (id, project_id, from_status, to_status, changed_at, changed_by_person_id, note, _version)
             VALUES ('sh-1', 'proj-sh', 'open', 'in_progress', datetime('now'), 'owner-1', 'Started', 1)",
            [],
        )
        .unwrap();
    }
    assert_eq!(count_sync_metadata(&pool), base_count + 1);
    let (table, _, op, snapshot) = get_last_sync_metadata(&pool);
    assert_eq!(table, "status_history");
    assert_eq!(op, "INSERT");
    let json: serde_json::Value = serde_json::from_str(&snapshot.unwrap()).unwrap();
    assert_eq!(json["from_status"], "open");
    assert_eq!(json["to_status"], "in_progress");
    assert_eq!(json["note"], "Started");

    // DELETE status_history
    {
        let conn = pool.0.lock().unwrap();
        conn.execute("DELETE FROM status_history WHERE id = 'sh-1'", [])
            .unwrap();
    }
    assert_eq!(count_sync_metadata(&pool), base_count + 2);
    let (_, _, op, snapshot) = get_last_sync_metadata(&pool);
    assert_eq!(op, "DELETE");
    assert!(snapshot.is_none());
}

// ══════════════════════════════════════════════════════════
//  project_tags 触发器测试
// ══════════════════════════════════════════════════════════

#[test]
fn trigger_project_tags_insert_delete() {
    let (pool, _device_id) = setup_with_sync_enabled();

    seed_person_and_partner(&pool);
    {
        let conn = pool.0.lock().unwrap();
        conn.execute(
            "INSERT INTO projects (id, name, description, priority, current_status, country_code, partner_id, owner_person_id, created_at, updated_at, _version)
             VALUES ('proj-tag', 'Tag Proj', '', 3, 'open', 'US', 'partner-1', 'owner-1', datetime('now'), datetime('now'), 1)",
            [],
        )
        .unwrap();
    }
    let base_count = count_sync_metadata(&pool);

    // INSERT tag
    {
        let conn = pool.0.lock().unwrap();
        conn.execute(
            "INSERT INTO project_tags (project_id, tag, created_at) VALUES ('proj-tag', 'urgent', datetime('now'))",
            [],
        )
        .unwrap();
    }
    assert_eq!(count_sync_metadata(&pool), base_count + 1);
    let (table, record_id, op, snapshot) = get_last_sync_metadata(&pool);
    assert_eq!(table, "project_tags");
    assert_eq!(record_id, "proj-tag:urgent"); // composite key
    assert_eq!(op, "INSERT");
    let json: serde_json::Value = serde_json::from_str(&snapshot.unwrap()).unwrap();
    assert_eq!(json["project_id"], "proj-tag");
    assert_eq!(json["tag"], "urgent");

    // DELETE tag
    {
        let conn = pool.0.lock().unwrap();
        conn.execute(
            "DELETE FROM project_tags WHERE project_id = 'proj-tag' AND tag = 'urgent'",
            [],
        )
        .unwrap();
    }
    assert_eq!(count_sync_metadata(&pool), base_count + 2);
    let (_, record_id, op, snapshot) = get_last_sync_metadata(&pool);
    assert_eq!(record_id, "proj-tag:urgent");
    assert_eq!(op, "DELETE");
    assert!(snapshot.is_none());
}

// ══════════════════════════════════════════════════════════
//  边界场景测试
// ══════════════════════════════════════════════════════════

#[test]
fn trigger_multiple_operations_sequential() {
    let (pool, _device_id) = setup_with_sync_enabled();

    // Rapid sequence of operations
    {
        let conn = pool.0.lock().unwrap();
        for i in 0..5 {
            conn.execute(
                &format!(
                    "INSERT INTO persons (id, display_name, email, role, note, is_active, created_at, updated_at, _version)
                     VALUES ('batch-{}', 'Person {}', '', '', '', 1, datetime('now'), datetime('now'), 1)",
                    i, i
                ),
                [],
            )
            .unwrap();
        }
    }
    assert_eq!(count_sync_metadata(&pool), 5);
}

#[test]
fn trigger_toggle_sync_mid_stream() {
    let pool = setup_with_sync_disabled();

    // Insert while sync is off
    {
        let conn = pool.0.lock().unwrap();
        conn.execute(
            "INSERT INTO persons (id, display_name, email, role, note, is_active, created_at, updated_at, _version)
             VALUES ('toggle-1', 'Before', '', '', '', 1, datetime('now'), datetime('now'), 1)",
            [],
        )
        .unwrap();
    }
    assert_eq!(count_sync_metadata(&pool), 0);

    // Enable sync
    {
        let conn = pool.0.lock().unwrap();
        conn.execute(
            "UPDATE sync_config SET value = '1' WHERE key = 'sync_enabled'",
            [],
        )
        .unwrap();
    }

    // Insert after sync is on
    {
        let conn = pool.0.lock().unwrap();
        conn.execute(
            "INSERT INTO persons (id, display_name, email, role, note, is_active, created_at, updated_at, _version)
             VALUES ('toggle-2', 'After', '', '', '', 1, datetime('now'), datetime('now'), 1)",
            [],
        )
        .unwrap();
    }
    assert_eq!(count_sync_metadata(&pool), 1);

    // Disable sync again
    {
        let conn = pool.0.lock().unwrap();
        conn.execute(
            "UPDATE sync_config SET value = '0' WHERE key = 'sync_enabled'",
            [],
        )
        .unwrap();
    }

    // Insert should not generate metadata
    {
        let conn = pool.0.lock().unwrap();
        conn.execute(
            "INSERT INTO persons (id, display_name, email, role, note, is_active, created_at, updated_at, _version)
             VALUES ('toggle-3', 'Disabled Again', '', '', '', 1, datetime('now'), datetime('now'), 1)",
            [],
        )
        .unwrap();
    }
    assert_eq!(
        count_sync_metadata(&pool),
        1,
        "Should still be 1 after disabling sync"
    );
}

#[test]
fn trigger_data_snapshot_json_is_valid() {
    let (pool, _device_id) = setup_with_sync_enabled();

    seed_person_and_partner(&pool);
    {
        let conn = pool.0.lock().unwrap();
        conn.execute(
            "INSERT INTO projects (id, name, description, priority, current_status, country_code, partner_id, owner_person_id, start_date, due_date, created_at, updated_at, archived_at, _version)
             VALUES ('proj-json', '\"Quoted\" Name', 'Has ''apostrophe''', 5, 'open', 'JP', 'partner-1', 'owner-1', '2026-01-01', '2026-12-31', datetime('now'), datetime('now'), NULL, 1)",
            [],
        )
        .unwrap();
    }

    // Get the project's sync_metadata
    let conn = pool.0.lock().unwrap();
    let snapshot: String = conn
        .query_row(
            "SELECT data_snapshot FROM sync_metadata WHERE table_name = 'projects' AND record_id = 'proj-json'",
            [],
            |row: &rusqlite::Row<'_>| row.get(0),
        )
        .unwrap();

    // Verify it's valid JSON
    let json: serde_json::Value = serde_json::from_str(&snapshot).unwrap();
    assert_eq!(json["id"], "proj-json");
    assert_eq!(json["priority"], 5);
    // NULL fields should appear as JSON null
    assert!(json["archived_at"].is_null());
    assert_eq!(json["start_date"], "2026-01-01");
}

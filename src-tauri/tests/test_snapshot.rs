//! Snapshot manager integration tests (in-memory SQLite)

use app_lib::infra::db::init_test_db;
use app_lib::sync::snapshot::Snapshot;
use app_lib::sync::SnapshotManager;

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

fn seed_data(pool: &app_lib::infra::DbPool) {
    let conn = pool.0.lock().unwrap();

    // Insert a person
    conn.execute(
        "INSERT INTO persons (id, display_name, email, role, note, is_active, created_at, updated_at, _version)
         VALUES ('p1', 'Alice', 'a@test.com', 'developer', 'note', 1, '2026-01-01', '2026-01-01', 1)",
        [],
    )
    .unwrap();

    // Insert a partner
    conn.execute(
        "INSERT INTO partners (id, name, note, is_active, created_at, updated_at, _version)
         VALUES ('pt1', 'ACME Corp', '', 1, '2026-01-01', '2026-01-01', 1)",
        [],
    )
    .unwrap();

    // Insert a project
    conn.execute(
        "INSERT INTO projects (id, name, description, priority, current_status, country_code, partner_id, owner_person_id, created_at, updated_at, _version)
         VALUES ('proj1', 'Demo', 'desc', 3, 'active', 'CN', 'pt1', 'p1', '2026-01-01', '2026-01-01', 1)",
        [],
    )
    .unwrap();
}

// ──────────────────────── Snapshot 数据结构测试 ────────────────────────

#[test]
fn snapshot_checksum_is_deterministic() {
    let c1 = Snapshot::calculate_checksum("hello world");
    let c2 = Snapshot::calculate_checksum("hello world");
    assert_eq!(c1, c2);
    assert_eq!(c1.len(), 64);
}

#[test]
fn snapshot_checksum_differs_for_different_data() {
    let c1 = Snapshot::calculate_checksum("hello");
    let c2 = Snapshot::calculate_checksum("world");
    assert_ne!(c1, c2);
}

#[test]
fn snapshot_verify_passes_for_valid() {
    let snap = Snapshot {
        version: 1,
        created_at: "2026-01-01".into(),
        device_id: "d1".into(),
        data: "test data".into(),
        checksum: Snapshot::calculate_checksum("test data"),
    };
    assert!(snap.verify());
}

#[test]
fn snapshot_verify_fails_for_tampered_data() {
    let snap = Snapshot {
        version: 1,
        created_at: "2026-01-01".into(),
        device_id: "d1".into(),
        data: "tampered data".into(),
        checksum: Snapshot::calculate_checksum("original data"),
    };
    assert!(!snap.verify());
}

#[test]
fn snapshot_compress_decompress_roundtrip() {
    let snap = Snapshot {
        version: 1,
        created_at: "2026-01-01T00:00:00Z".into(),
        device_id: "test-device".into(),
        data: r#"{"persons":[],"partners":[],"projects":[]}"#.into(),
        checksum: Snapshot::calculate_checksum(r#"{"persons":[],"partners":[],"projects":[]}"#),
    };

    let compressed = snap.compress().unwrap();
    assert!(!compressed.is_empty());

    let restored = Snapshot::decompress(&compressed).unwrap();
    assert_eq!(restored.version, snap.version);
    assert_eq!(restored.device_id, snap.device_id);
    assert_eq!(restored.data, snap.data);
    assert_eq!(restored.checksum, snap.checksum);
    assert!(restored.verify());
}

#[test]
fn snapshot_decompress_invalid_data_returns_error() {
    let result = Snapshot::decompress(b"not gzip");
    assert!(result.is_err());
}

// ──────────────────────── SnapshotManager 集成测试 ────────────────────────

#[test]
fn create_snapshot_with_empty_db() {
    let (pool, device_id) = setup();
    let mgr = SnapshotManager::new(&pool, device_id.clone());

    let snap = mgr.create_snapshot().unwrap();
    assert!(snap.verify());
    assert_eq!(snap.device_id, device_id);
    assert!(!snap.data.is_empty());
}

#[test]
fn create_snapshot_with_data() {
    let (pool, device_id) = setup();
    seed_data(&pool);

    let mgr = SnapshotManager::new(&pool, device_id);
    let snap = mgr.create_snapshot().unwrap();

    assert!(snap.verify());

    // Snapshot data should contain our seeded records
    let data: serde_json::Value = serde_json::from_str(&snap.data).unwrap();
    let persons = data["persons"].as_array().unwrap();
    assert_eq!(persons.len(), 1);
    assert_eq!(persons[0]["displayName"], "Alice");

    let partners = data["partners"].as_array().unwrap();
    assert_eq!(partners.len(), 1);

    let projects = data["projects"].as_array().unwrap();
    assert_eq!(projects.len(), 1);
    assert_eq!(projects[0]["name"], "Demo");
}

#[test]
fn snapshot_compress_roundtrip_with_real_data() {
    let (pool, device_id) = setup();
    seed_data(&pool);

    let mgr = SnapshotManager::new(&pool, device_id);
    let snap = mgr.create_snapshot().unwrap();

    // Compress → decompress → verify
    let compressed = snap.compress().unwrap();
    let restored = Snapshot::decompress(&compressed).unwrap();
    assert!(restored.verify());
    assert_eq!(restored.data, snap.data);
}

// ══════════════════════════════════════════════════════════
//  SnapshotManager::restore_snapshot
// ══════════════════════════════════════════════════════════

fn count_table(pool: &app_lib::infra::DbPool, table: &str) -> i64 {
    let conn = pool.0.lock().unwrap();
    conn.query_row(
        &format!("SELECT COUNT(*) FROM {}", table),
        [],
        |r: &rusqlite::Row<'_>| r.get(0),
    )
    .unwrap()
}

fn seed_full_data(pool: &app_lib::infra::DbPool) {
    let conn = pool.0.lock().unwrap();
    conn.execute(
        "INSERT INTO persons (id, display_name, email, role, note, is_active, created_at, updated_at, _version)
         VALUES ('p1', 'Alice', 'a@t.com', 'dev', 'note', 1, '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z', 1)",
        [],
    ).unwrap();
    conn.execute(
        "INSERT INTO partners (id, name, note, is_active, created_at, updated_at, _version)
         VALUES ('pt1', 'ACME', '', 1, '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z', 1)",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO projects (id, name, description, priority, current_status, country_code, partner_id, owner_person_id, start_date, due_date, created_at, updated_at, archived_at, _version)
         VALUES ('proj1', 'Demo', 'desc', 3, 'BACKLOG', 'CN', 'pt1', 'p1', '2026-01-01', '2026-12-31', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z', NULL, 1)",
        [],
    ).unwrap();
    conn.execute(
        "INSERT INTO project_tags (project_id, tag, created_at) VALUES ('proj1', 'urgent', '2026-01-01T00:00:00Z')",
        [],
    ).unwrap();
    conn.execute(
        "INSERT INTO assignments (id, project_id, person_id, role, start_at, end_at, created_at, _version)
         VALUES ('a1', 'proj1', 'p1', 'owner', '2026-01-01T00:00:00Z', NULL, '2026-01-01T00:00:00Z', 1)",
        [],
    ).unwrap();
    conn.execute(
        "INSERT INTO status_history (id, project_id, from_status, to_status, changed_at, changed_by_person_id, note, _version)
         VALUES ('sh1', 'proj1', NULL, 'BACKLOG', '2026-01-01T00:00:00Z', 'p1', 'created', 1)",
        [],
    ).unwrap();
    conn.execute(
        "INSERT INTO project_comments (id, project_id, person_id, content, is_pinned, created_at, updated_at, _version)
         VALUES ('c1', 'proj1', 'p1', '{\"type\":\"doc\",\"content\":[]}', 1, '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z', 1)",
        [],
    )
    .unwrap();
}

#[test]
fn restore_snapshot_full_flow() {
    let (pool, device_id) = setup();
    seed_full_data(&pool);

    let mgr = SnapshotManager::new(&pool, device_id.clone());

    // Create snapshot
    let snap = mgr.create_snapshot().unwrap();

    // Add extra data that should be wiped during restore
    {
        let conn = pool.0.lock().unwrap();
        conn.execute(
            "INSERT INTO persons (id, display_name, email, role, note, is_active, created_at, updated_at, _version)
             VALUES ('extra', 'Extra', '', '', '', 1, datetime('now'), datetime('now'), 1)",
            [],
        ).unwrap();
    }
    assert_eq!(count_table(&pool, "persons"), 2);

    // Restore → should revert to snapshot state
    mgr.restore_snapshot(&snap).unwrap();

    assert_eq!(count_table(&pool, "persons"), 1);
    assert_eq!(count_table(&pool, "partners"), 1);
    assert_eq!(count_table(&pool, "projects"), 1);
    assert_eq!(count_table(&pool, "assignments"), 1);
    assert_eq!(count_table(&pool, "status_history"), 1);
    assert_eq!(count_table(&pool, "project_comments"), 1);

    // Verify content
    let conn = pool.0.lock().unwrap();
    let name: String = conn
        .query_row(
            "SELECT display_name FROM persons WHERE id = 'p1'",
            [],
            |r: &rusqlite::Row<'_>| r.get(0),
        )
        .unwrap();
    assert_eq!(name, "Alice");

    let proj_name: String = conn
        .query_row(
            "SELECT name FROM projects WHERE id = 'proj1'",
            [],
            |r: &rusqlite::Row<'_>| r.get(0),
        )
        .unwrap();
    assert_eq!(proj_name, "Demo");
}

#[test]
fn restore_snapshot_clears_all_tables() {
    let (pool, device_id) = setup();
    seed_full_data(&pool);

    // Snapshot empty DB (no seed data)
    let (empty_pool, empty_device) = setup();
    let empty_mgr = SnapshotManager::new(&empty_pool, empty_device);
    let empty_snap = empty_mgr.create_snapshot().unwrap();

    // Restore empty snapshot onto full DB
    let mgr = SnapshotManager::new(&pool, device_id);
    mgr.restore_snapshot(&empty_snap).unwrap();

    assert_eq!(count_table(&pool, "persons"), 0);
    assert_eq!(count_table(&pool, "partners"), 0);
    assert_eq!(count_table(&pool, "projects"), 0);
    assert_eq!(count_table(&pool, "assignments"), 0);
    assert_eq!(count_table(&pool, "status_history"), 0);
    assert_eq!(count_table(&pool, "project_comments"), 0);
}

#[test]
fn restore_snapshot_fails_for_tampered_checksum() {
    let (pool, device_id) = setup();
    let mgr = SnapshotManager::new(&pool, device_id);

    let snap = Snapshot {
        version: 1,
        created_at: "2026-01-01".into(),
        device_id: "d1".into(),
        data: r#"{"schemaVersion":1,"exportedAt":"","persons":[],"partners":[],"projects":[],"assignments":[],"statusHistory":[]}"#.into(),
        checksum: "wrong_checksum".into(),
    };

    let err = mgr.restore_snapshot(&snap);
    assert!(err.is_err());
    assert!(err.unwrap_err().to_string().contains("integrity"));
}

#[test]
fn restore_snapshot_fails_for_invalid_json() {
    let (pool, device_id) = setup();
    let mgr = SnapshotManager::new(&pool, device_id);

    let bad_data = "not valid json {{{{";
    let snap = Snapshot {
        version: 1,
        created_at: "2026-01-01".into(),
        device_id: "d1".into(),
        data: bad_data.into(),
        checksum: Snapshot::calculate_checksum(bad_data),
    };

    let err = mgr.restore_snapshot(&snap);
    assert!(err.is_err());
    assert!(err
        .unwrap_err()
        .to_string()
        .contains("Invalid snapshot data"));
}

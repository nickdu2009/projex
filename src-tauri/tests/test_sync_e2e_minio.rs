//! Optional MinIO end-to-end sync test.
//! Run only when SYNC_MINIO_TEST=1 is set.

use app_lib::infra::{db::init_test_db, DbPool};
use app_lib::sync::{Delta, S3SyncClient};
use app_lib::{
    sync_create_snapshot_for_pool, sync_full_for_pool, sync_full_with_runtime_for_pool,
    sync_hold_lock_for_test, sync_restore_snapshot_for_pool, SyncRuntime,
};
use aws_config::meta::region::RegionProviderChain;
use aws_credential_types::Credentials;
use aws_sdk_s3::config::Region;
use aws_sdk_s3::Client as AwsS3Client;
use std::env;
use tokio::time::{Duration, Instant};
use uuid::Uuid;

#[derive(Debug, Clone)]
struct MinioE2eConfig {
    endpoint: String,
    bucket_base: String,
    access_key: String,
    secret_key: String,
}

impl MinioE2eConfig {
    fn from_env() -> Option<Self> {
        if env::var("SYNC_MINIO_TEST").ok().as_deref() != Some("1") {
            return None;
        }

        Some(Self {
            endpoint: required_with_fallback("SYNC_TEST_S3_ENDPOINT", "SYNC_S3_ENDPOINT"),
            bucket_base: required_with_fallback("SYNC_TEST_S3_BUCKET", "SYNC_S3_BUCKET"),
            access_key: required_with_fallback("SYNC_TEST_S3_ACCESS_KEY", "SYNC_S3_ACCESS_KEY"),
            secret_key: required_with_fallback("SYNC_TEST_S3_SECRET_KEY", "SYNC_S3_SECRET_KEY"),
        })
    }
}

fn required_with_fallback(primary: &str, fallback: &str) -> String {
    env::var(primary)
        .or_else(|_| env::var(fallback))
        .unwrap_or_else(|_| panic!("Missing env var: {} (or fallback {})", primary, fallback))
}

fn normalize_bucket_base(raw: &str) -> String {
    let mut normalized = raw
        .to_ascii_lowercase()
        .chars()
        .map(|c| {
            if c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' {
                c
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .to_string();

    if normalized.is_empty() {
        normalized = "projex-sync-e2e".to_string();
    }

    // S3 bucket max length is 63.
    let max_base_len = 63usize - 1 - 8;
    if normalized.len() > max_base_len {
        normalized.truncate(max_base_len);
        normalized = normalized.trim_matches('-').to_string();
    }

    if normalized.is_empty() {
        "projex-sync-e2e".to_string()
    } else {
        normalized
    }
}

async fn create_isolated_bucket(cfg: &MinioE2eConfig) -> String {
    let base = normalize_bucket_base(&cfg.bucket_base);
    let suffix = Uuid::new_v4()
        .to_string()
        .replace('-', "")
        .chars()
        .take(8)
        .collect::<String>();
    let bucket = format!("{}-{}", base, suffix);

    let creds = Credentials::new(
        cfg.access_key.clone(),
        cfg.secret_key.clone(),
        None,
        None,
        "sync-e2e-test",
    );
    let region_provider =
        RegionProviderChain::first_try(Region::new("us-east-1".to_string())).or_else("us-east-1");
    let shared_config = aws_config::defaults(aws_config::BehaviorVersion::latest())
        .region(region_provider)
        .endpoint_url(cfg.endpoint.clone())
        .credentials_provider(creds)
        .load()
        .await;
    let s3_config = aws_sdk_s3::config::Builder::from(&shared_config)
        .force_path_style(true)
        .build();
    let client = AwsS3Client::from_conf(s3_config);

    client
        .create_bucket()
        .bucket(&bucket)
        .send()
        .await
        .unwrap_or_else(|e| panic!("failed to create isolated bucket {}: {}", bucket, e));

    bucket
}

fn random_suffix(len: usize) -> String {
    Uuid::new_v4()
        .to_string()
        .replace('-', "")
        .chars()
        .take(len)
        .collect()
}

fn set_config_value(conn: &rusqlite::Connection, key: &str, value: &str) {
    conn.execute(
        "INSERT INTO sync_config (key, value)
         VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        rusqlite::params![key, value],
    )
    .expect("set sync config value");
}

fn configure_pool(pool: &DbPool, cfg: &MinioE2eConfig, bucket: &str, device_id: &str) {
    let conn = pool.0.lock().expect("db lock");
    set_config_value(&conn, "sync_enabled", "1");
    set_config_value(&conn, "s3_bucket", bucket);
    set_config_value(&conn, "s3_endpoint", &cfg.endpoint);
    set_config_value(&conn, "s3_access_key", &cfg.access_key);
    set_config_value(&conn, "s3_secret_key", &cfg.secret_key);
    set_config_value(&conn, "device_id", device_id);
}

fn insert_person(pool: &DbPool, person_id: &str, display_name: &str) {
    let conn = pool.0.lock().expect("db lock");
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO persons (id, display_name, email, role, note, is_active, created_at, updated_at, _version)
         VALUES (?1, ?2, '', '', '', 1, ?3, ?3, 1)",
        rusqlite::params![person_id, display_name, now],
    )
    .expect("insert person");
}

fn update_person_with_version_bump(pool: &DbPool, person_id: &str, display_name: &str) {
    let conn = pool.0.lock().expect("db lock");
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "UPDATE persons
         SET display_name = ?1, updated_at = ?2, _version = _version + 1
         WHERE id = ?3",
        rusqlite::params![display_name, now, person_id],
    )
    .expect("update person");
}

fn get_person_display_name(pool: &DbPool, person_id: &str) -> Option<String> {
    let conn = pool.0.lock().expect("db lock");
    conn.query_row(
        "SELECT display_name FROM persons WHERE id = ?1",
        rusqlite::params![person_id],
        |row| row.get(0),
    )
    .ok()
}

fn person_count(pool: &DbPool, person_id: &str) -> i64 {
    let conn = pool.0.lock().expect("db lock");
    conn.query_row(
        "SELECT COUNT(*) FROM persons WHERE id = ?1",
        rusqlite::params![person_id],
        |row| row.get(0),
    )
    .expect("count person")
}

fn unsynced_meta_count(pool: &DbPool, table_name: &str, record_id: &str) -> i64 {
    let conn = pool.0.lock().expect("db lock");
    conn.query_row(
        "SELECT COUNT(*) FROM sync_metadata WHERE table_name = ?1 AND record_id = ?2 AND synced = 0",
        rusqlite::params![table_name, record_id],
        |row| row.get(0),
    )
    .expect("count unsynced metadata")
}

fn read_remote_cursor(pool: &DbPool, source_device_id: &str) -> Option<i64> {
    let conn = pool.0.lock().expect("db lock");
    conn.query_row(
        "SELECT value FROM sync_config WHERE key = ?1",
        rusqlite::params![format!("last_remote_delta_ts::{}", source_device_id)],
        |row| row.get::<_, String>(0),
    )
    .ok()
    .and_then(|v| v.parse::<i64>().ok())
}

fn read_config_value(pool: &DbPool, key: &str) -> Option<String> {
    let conn = pool.0.lock().expect("db lock");
    conn.query_row(
        "SELECT value FROM sync_config WHERE key = ?1",
        rusqlite::params![key],
        |row| row.get(0),
    )
    .ok()
}

fn set_sync_endpoint(pool: &DbPool, endpoint: &str) {
    let conn = pool.0.lock().expect("db lock");
    set_config_value(&conn, "s3_endpoint", endpoint);
}

async fn make_bucket_client(cfg: &MinioE2eConfig, bucket: &str, device_id: &str) -> S3SyncClient {
    S3SyncClient::new_with_endpoint(
        bucket.to_string(),
        device_id.to_string(),
        cfg.endpoint.clone(),
        cfg.access_key.clone(),
        cfg.secret_key.clone(),
    )
    .await
    .expect("create S3 sync client for test")
}

fn delete_person(pool: &DbPool, person_id: &str) {
    let conn = pool.0.lock().expect("db lock");
    conn.execute(
        "DELETE FROM persons WHERE id = ?1",
        rusqlite::params![person_id],
    )
    .expect("delete person");
}

fn get_person_version(pool: &DbPool, person_id: &str) -> Option<i64> {
    let conn = pool.0.lock().expect("db lock");
    conn.query_row(
        "SELECT _version FROM persons WHERE id = ?1",
        rusqlite::params![person_id],
        |row| row.get(0),
    )
    .ok()
}

fn insert_partner(pool: &DbPool, partner_id: &str, name: &str) {
    let conn = pool.0.lock().expect("db lock");
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO partners (id, name, note, is_active, created_at, updated_at, _version)
         VALUES (?1, ?2, '', 1, ?3, ?3, 1)",
        rusqlite::params![partner_id, name, now],
    )
    .expect("insert partner");
}

fn insert_project(pool: &DbPool, project_id: &str, partner_id: &str, owner_person_id: &str) {
    let conn = pool.0.lock().expect("db lock");
    let now = chrono::Utc::now().to_rfc3339();
    let empty_date: Option<&str> = None;
    conn.execute(
        "INSERT INTO projects (
            id, name, description, priority, current_status, country_code,
            partner_id, owner_person_id, start_date, due_date, created_at, updated_at, archived_at, _version
         ) VALUES (?1, ?2, '', 3, 'BACKLOG', 'US', ?3, ?4, ?5, ?6, ?7, ?7, ?8, 1)",
        rusqlite::params![
            project_id,
            format!("Project {}", &project_id[..project_id.len().min(8)]),
            partner_id,
            owner_person_id,
            empty_date,
            empty_date,
            now,
            empty_date
        ],
    )
    .expect("insert project");
}

fn insert_project_tag(pool: &DbPool, project_id: &str, tag: &str) {
    let conn = pool.0.lock().expect("db lock");
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO project_tags (project_id, tag, created_at) VALUES (?1, ?2, ?3)",
        rusqlite::params![project_id, tag, now],
    )
    .expect("insert project tag");
}

fn delete_project_tag(pool: &DbPool, project_id: &str, tag: &str) {
    let conn = pool.0.lock().expect("db lock");
    conn.execute(
        "DELETE FROM project_tags WHERE project_id = ?1 AND tag = ?2",
        rusqlite::params![project_id, tag],
    )
    .expect("delete project tag");
}

fn insert_project_comment(
    pool: &DbPool,
    comment_id: &str,
    project_id: &str,
    person_id: Option<&str>,
    content: &str,
    is_pinned: bool,
) {
    let conn = pool.0.lock().expect("db lock");
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO project_comments (
            id, project_id, person_id, content, is_pinned, created_at, updated_at, _version
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6, 1)",
        rusqlite::params![
            comment_id,
            project_id,
            person_id,
            content,
            if is_pinned { 1 } else { 0 },
            now
        ],
    )
    .expect("insert project comment");
}

fn update_project_comment_with_version_bump(
    pool: &DbPool,
    comment_id: &str,
    content: &str,
    is_pinned: bool,
) {
    let conn = pool.0.lock().expect("db lock");
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "UPDATE project_comments
         SET content = ?1, is_pinned = ?2, updated_at = ?3, _version = _version + 1
         WHERE id = ?4",
        rusqlite::params![content, if is_pinned { 1 } else { 0 }, now, comment_id],
    )
    .expect("update project comment");
}

fn delete_project_comment(pool: &DbPool, comment_id: &str) {
    let conn = pool.0.lock().expect("db lock");
    conn.execute(
        "DELETE FROM project_comments WHERE id = ?1",
        rusqlite::params![comment_id],
    )
    .expect("delete project comment");
}

fn project_count(pool: &DbPool, project_id: &str) -> i64 {
    let conn = pool.0.lock().expect("db lock");
    conn.query_row(
        "SELECT COUNT(*) FROM projects WHERE id = ?1",
        rusqlite::params![project_id],
        |row| row.get(0),
    )
    .expect("count project")
}

fn partner_count(pool: &DbPool, partner_id: &str) -> i64 {
    let conn = pool.0.lock().expect("db lock");
    conn.query_row(
        "SELECT COUNT(*) FROM partners WHERE id = ?1",
        rusqlite::params![partner_id],
        |row| row.get(0),
    )
    .expect("count partner")
}

fn project_tag_count(pool: &DbPool, project_id: &str, tag: &str) -> i64 {
    let conn = pool.0.lock().expect("db lock");
    conn.query_row(
        "SELECT COUNT(*) FROM project_tags WHERE project_id = ?1 AND tag = ?2",
        rusqlite::params![project_id, tag],
        |row| row.get(0),
    )
    .expect("count project tag")
}

fn project_comment_count(pool: &DbPool, comment_id: &str) -> i64 {
    let conn = pool.0.lock().expect("db lock");
    conn.query_row(
        "SELECT COUNT(*) FROM project_comments WHERE id = ?1",
        rusqlite::params![comment_id],
        |row| row.get(0),
    )
    .expect("count project comment")
}

fn project_comment_content(pool: &DbPool, comment_id: &str) -> Option<String> {
    let conn = pool.0.lock().expect("db lock");
    conn.query_row(
        "SELECT content FROM project_comments WHERE id = ?1",
        rusqlite::params![comment_id],
        |row| row.get(0),
    )
    .ok()
}

#[test]
fn sync_full_end_to_end_minio_two_devices() {
    let Some(cfg) = MinioE2eConfig::from_env() else {
        eprintln!("skip sync_full_end_to_end_minio_two_devices: SYNC_MINIO_TEST != 1");
        return;
    };

    let rt = tokio::runtime::Runtime::new().expect("create tokio runtime");
    rt.block_on(async {
        let bucket = create_isolated_bucket(&cfg).await;
        let pool_a = init_test_db();
        let pool_b = init_test_db();
        let device_a = format!("e2e-device-a-{}", random_suffix(6));
        let device_b = format!("e2e-device-b-{}", random_suffix(6));

        configure_pool(&pool_a, &cfg, &bucket, &device_a);
        configure_pool(&pool_b, &cfg, &bucket, &device_b);

        let person_id = format!("e2e-person-{}", random_suffix(8));
        insert_person(&pool_a, &person_id, "Alice E2E");
        assert_eq!(unsynced_meta_count(&pool_a, "persons", &person_id), 1);

        sync_full_for_pool(&pool_a)
            .await
            .expect("device A should upload local delta");
        sync_full_for_pool(&pool_b)
            .await
            .expect("device B should pull and apply device A delta");

        assert_eq!(
            get_person_display_name(&pool_b, &person_id).as_deref(),
            Some("Alice E2E")
        );
        assert_eq!(unsynced_meta_count(&pool_b, "persons", &person_id), 0);
        assert!(
            read_remote_cursor(&pool_b, &device_a).unwrap_or(0) > 0,
            "device B should persist remote cursor for device A"
        );

        update_person_with_version_bump(&pool_a, &person_id, "Alice E2E Updated");
        sync_full_for_pool(&pool_a)
            .await
            .expect("device A should upload updated person delta");
        sync_full_for_pool(&pool_b)
            .await
            .expect("device B should apply updated person delta");

        assert_eq!(
            get_person_display_name(&pool_b, &person_id).as_deref(),
            Some("Alice E2E Updated")
        );

        // Idempotency: pulling again should not duplicate data.
        sync_full_for_pool(&pool_b)
            .await
            .expect("device B idempotent sync should succeed");
        assert_eq!(person_count(&pool_b, &person_id), 1);
    });
}

#[test]
fn sync_full_delete_propagates_across_devices() {
    let Some(cfg) = MinioE2eConfig::from_env() else {
        eprintln!("skip sync_full_delete_propagates_across_devices: SYNC_MINIO_TEST != 1");
        return;
    };

    let rt = tokio::runtime::Runtime::new().expect("create tokio runtime");
    rt.block_on(async {
        let bucket = create_isolated_bucket(&cfg).await;
        let pool_a = init_test_db();
        let pool_b = init_test_db();
        let device_a = format!("e2e-device-a-{}", random_suffix(6));
        let device_b = format!("e2e-device-b-{}", random_suffix(6));
        configure_pool(&pool_a, &cfg, &bucket, &device_a);
        configure_pool(&pool_b, &cfg, &bucket, &device_b);

        let person_id = format!("e2e-delete-person-{}", random_suffix(8));
        insert_person(&pool_a, &person_id, "To Delete");

        sync_full_for_pool(&pool_a)
            .await
            .expect("device A should upload inserted person");
        sync_full_for_pool(&pool_b)
            .await
            .expect("device B should pull inserted person");
        assert_eq!(person_count(&pool_b, &person_id), 1);

        delete_person(&pool_a, &person_id);
        assert!(
            unsynced_meta_count(&pool_a, "persons", &person_id) >= 1,
            "delete operation should be tracked in sync_metadata"
        );

        sync_full_for_pool(&pool_a)
            .await
            .expect("device A should upload delete delta");
        sync_full_for_pool(&pool_b)
            .await
            .expect("device B should apply delete delta");
        assert_eq!(person_count(&pool_b, &person_id), 0);
    });
}

#[test]
fn sync_full_stale_remote_update_does_not_override_newer_local() {
    let Some(cfg) = MinioE2eConfig::from_env() else {
        eprintln!(
            "skip sync_full_stale_remote_update_does_not_override_newer_local: SYNC_MINIO_TEST != 1"
        );
        return;
    };

    let rt = tokio::runtime::Runtime::new().expect("create tokio runtime");
    rt.block_on(async {
        let bucket = create_isolated_bucket(&cfg).await;
        let pool_a = init_test_db();
        let pool_b = init_test_db();
        let device_a = format!("e2e-device-a-{}", random_suffix(6));
        let device_b = format!("e2e-device-b-{}", random_suffix(6));
        configure_pool(&pool_a, &cfg, &bucket, &device_a);
        configure_pool(&pool_b, &cfg, &bucket, &device_b);

        let person_id = format!("e2e-conflict-person-{}", random_suffix(8));
        insert_person(&pool_a, &person_id, "Base");
        sync_full_for_pool(&pool_a)
            .await
            .expect("device A should upload base person");
        sync_full_for_pool(&pool_b)
            .await
            .expect("device B should pull base person");

        // Simulate version skew:
        // - device A updates to v2
        // - device B updates to v3
        // Then B pulls A's v2 and must keep local v3.
        update_person_with_version_bump(&pool_a, &person_id, "A-v2");
        update_person_with_version_bump(&pool_b, &person_id, "B-v2");
        update_person_with_version_bump(&pool_b, &person_id, "B-v3");

        sync_full_for_pool(&pool_a)
            .await
            .expect("device A should upload v2 delta");
        sync_full_for_pool(&pool_b)
            .await
            .expect("device B should upload v3 and pull stale v2");

        assert_eq!(
            get_person_display_name(&pool_b, &person_id).as_deref(),
            Some("B-v3")
        );
        assert_eq!(get_person_version(&pool_b, &person_id), Some(3));

        // Converge A by pulling B's v3.
        sync_full_for_pool(&pool_a)
            .await
            .expect("device A should pull v3 from device B");
        assert_eq!(
            get_person_display_name(&pool_a, &person_id).as_deref(),
            Some("B-v3")
        );
        assert_eq!(get_person_version(&pool_a, &person_id), Some(3));
    });
}

#[test]
fn snapshot_create_restore_end_to_end_minio() {
    let Some(cfg) = MinioE2eConfig::from_env() else {
        eprintln!("skip snapshot_create_restore_end_to_end_minio: SYNC_MINIO_TEST != 1");
        return;
    };

    let rt = tokio::runtime::Runtime::new().expect("create tokio runtime");
    rt.block_on(async {
        let bucket = create_isolated_bucket(&cfg).await;
        let pool = init_test_db();
        let device_id = format!("e2e-device-snapshot-{}", random_suffix(6));
        configure_pool(&pool, &cfg, &bucket, &device_id);

        let person_id = format!("e2e-snapshot-person-{}", random_suffix(8));
        insert_person(&pool, &person_id, "Snapshot Alice");
        sync_create_snapshot_for_pool(&pool)
            .await
            .expect("snapshot create should succeed");

        update_person_with_version_bump(&pool, &person_id, "Mutated");
        let extra_person_id = format!("e2e-snapshot-extra-{}", random_suffix(8));
        insert_person(&pool, &extra_person_id, "Extra");
        assert_eq!(person_count(&pool, &person_id), 1);
        assert_eq!(person_count(&pool, &extra_person_id), 1);

        sync_restore_snapshot_for_pool(&pool)
            .await
            .expect("snapshot restore should succeed");

        assert_eq!(
            get_person_display_name(&pool, &person_id).as_deref(),
            Some("Snapshot Alice")
        );
        assert_eq!(person_count(&pool, &person_id), 1);
        assert_eq!(person_count(&pool, &extra_person_id), 0);
    });
}

#[test]
fn sync_full_multitable_project_tag_comment_roundtrip() {
    let Some(cfg) = MinioE2eConfig::from_env() else {
        eprintln!("skip sync_full_multitable_project_tag_comment_roundtrip: SYNC_MINIO_TEST != 1");
        return;
    };

    let rt = tokio::runtime::Runtime::new().expect("create tokio runtime");
    rt.block_on(async {
        let bucket = create_isolated_bucket(&cfg).await;
        let pool_a = init_test_db();
        let pool_b = init_test_db();
        let device_a = format!("e2e-device-a-{}", random_suffix(6));
        let device_b = format!("e2e-device-b-{}", random_suffix(6));
        configure_pool(&pool_a, &cfg, &bucket, &device_a);
        configure_pool(&pool_b, &cfg, &bucket, &device_b);

        let person_id = format!("e2e-owner-{}", random_suffix(8));
        let partner_id = format!("e2e-partner-{}", random_suffix(8));
        let project_id = format!("e2e-project-{}", random_suffix(8));
        let tag = "urgent";
        let comment_id = format!("e2e-comment-{}", random_suffix(8));
        let comment_v1 = r#"{"type":"doc","content":[{"type":"paragraph","content":[{"type":"text","text":"v1"}]}]}"#;
        let comment_v2 = r#"{"type":"doc","content":[{"type":"paragraph","content":[{"type":"text","text":"v2"}]}]}"#;

        insert_person(&pool_a, &person_id, "Owner");
        insert_partner(&pool_a, &partner_id, "Partner A");
        insert_project(&pool_a, &project_id, &partner_id, &person_id);
        insert_project_tag(&pool_a, &project_id, tag);
        insert_project_comment(
            &pool_a,
            &comment_id,
            &project_id,
            Some(&person_id),
            comment_v1,
            true,
        );

        sync_full_for_pool(&pool_a)
            .await
            .expect("device A should upload multitable delta");
        sync_full_for_pool(&pool_b)
            .await
            .expect("device B should pull multitable delta");

        assert_eq!(person_count(&pool_b, &person_id), 1);
        assert_eq!(partner_count(&pool_b, &partner_id), 1);
        assert_eq!(project_count(&pool_b, &project_id), 1);
        assert_eq!(project_tag_count(&pool_b, &project_id, tag), 1);
        assert_eq!(project_comment_count(&pool_b, &comment_id), 1);
        assert_eq!(
            project_comment_content(&pool_b, &comment_id).as_deref(),
            Some(comment_v1)
        );

        update_project_comment_with_version_bump(&pool_a, &comment_id, comment_v2, false);
        sync_full_for_pool(&pool_a)
            .await
            .expect("device A should upload updated comment");
        sync_full_for_pool(&pool_b)
            .await
            .expect("device B should apply updated comment");
        assert_eq!(
            project_comment_content(&pool_b, &comment_id).as_deref(),
            Some(comment_v2)
        );

        delete_project_tag(&pool_a, &project_id, tag);
        delete_project_comment(&pool_a, &comment_id);
        sync_full_for_pool(&pool_a)
            .await
            .expect("device A should upload tag/comment deletes");
        sync_full_for_pool(&pool_b)
            .await
            .expect("device B should apply tag/comment deletes");

        assert_eq!(project_tag_count(&pool_b, &project_id, tag), 0);
        assert_eq!(project_comment_count(&pool_b, &comment_id), 0);
        assert_eq!(project_count(&pool_b, &project_id), 1);
    });
}

#[test]
fn sync_full_recovers_after_temporary_endpoint_failure() {
    let Some(cfg) = MinioE2eConfig::from_env() else {
        eprintln!("skip sync_full_recovers_after_temporary_endpoint_failure: SYNC_MINIO_TEST != 1");
        return;
    };

    let rt = tokio::runtime::Runtime::new().expect("create tokio runtime");
    rt.block_on(async {
        let bucket = create_isolated_bucket(&cfg).await;
        let pool = init_test_db();
        let device_id = format!("e2e-device-net-{}", random_suffix(6));
        configure_pool(&pool, &cfg, &bucket, &device_id);

        let person_id = format!("e2e-net-person-{}", random_suffix(8));
        insert_person(&pool, &person_id, "Network Failure Test");
        assert_eq!(unsynced_meta_count(&pool, "persons", &person_id), 1);

        set_sync_endpoint(&pool, "http://127.0.0.1:9");
        let err = sync_full_for_pool(&pool)
            .await
            .expect_err("sync should fail with unreachable endpoint");
        assert_eq!(err.code(), "SYNC_ERROR");
        assert!(
            read_config_value(&pool, "last_sync_error").is_some(),
            "failed sync should persist last_sync_error"
        );
        assert_eq!(
            unsynced_meta_count(&pool, "persons", &person_id),
            1,
            "failed upload should keep local metadata unsynced"
        );

        set_sync_endpoint(&pool, &cfg.endpoint);
        sync_full_for_pool(&pool)
            .await
            .expect("sync should recover after endpoint restored");
        assert_eq!(unsynced_meta_count(&pool, "persons", &person_id), 0);
        assert!(
            read_config_value(&pool, "last_sync_error").is_none(),
            "successful sync should clear last_sync_error"
        );
    });
}

#[test]
fn sync_full_detects_corrupted_remote_delta_and_then_recovers() {
    let Some(cfg) = MinioE2eConfig::from_env() else {
        eprintln!(
            "skip sync_full_detects_corrupted_remote_delta_and_then_recovers: SYNC_MINIO_TEST != 1"
        );
        return;
    };

    let rt = tokio::runtime::Runtime::new().expect("create tokio runtime");
    rt.block_on(async {
        let bucket = create_isolated_bucket(&cfg).await;
        let pool_a = init_test_db();
        let pool_b = init_test_db();
        let device_a = format!("e2e-device-a-{}", random_suffix(6));
        let device_b = format!("e2e-device-b-{}", random_suffix(6));
        configure_pool(&pool_a, &cfg, &bucket, &device_a);
        configure_pool(&pool_b, &cfg, &bucket, &device_b);

        let person_id = format!("e2e-corrupt-person-{}", random_suffix(8));
        insert_person(&pool_a, &person_id, "Checksum Target");
        sync_full_for_pool(&pool_a)
            .await
            .expect("device A should upload delta before corruption");

        let admin_client = make_bucket_client(&cfg, &bucket, "e2e-admin").await;
        let prefix = format!("deltas/{}/", device_a);
        let mut delta_keys = admin_client
            .list(&prefix)
            .await
            .expect("list uploaded delta keys");
        delta_keys.sort();
        let target_key = delta_keys
            .last()
            .cloned()
            .expect("at least one delta should exist");

        let original_data = admin_client
            .download(&target_key)
            .await
            .expect("download original delta");
        let mut corrupted_delta = Delta::decompress(&original_data).expect("decompress delta");
        corrupted_delta.checksum = "corrupted-checksum".to_string();
        let corrupted_data = corrupted_delta
            .compress()
            .expect("compress corrupted delta");
        admin_client
            .upload(&target_key, corrupted_data)
            .await
            .expect("upload corrupted delta");

        let err = sync_full_for_pool(&pool_b)
            .await
            .expect_err("device B should reject corrupted delta");
        assert_eq!(err.code(), "SYNC_ERROR");
        assert!(
            err.to_string().contains("Checksum mismatch"),
            "error should indicate checksum mismatch"
        );
        assert!(
            read_remote_cursor(&pool_b, &device_a).is_none(),
            "cursor should not advance on corrupted remote delta"
        );
        assert_eq!(person_count(&pool_b, &person_id), 0);

        admin_client
            .upload(&target_key, original_data)
            .await
            .expect("restore original delta");
        sync_full_for_pool(&pool_b)
            .await
            .expect("device B should recover after restoring valid delta");
        assert_eq!(
            get_person_display_name(&pool_b, &person_id).as_deref(),
            Some("Checksum Target")
        );
        assert!(
            read_remote_cursor(&pool_b, &device_a).unwrap_or(0) > 0,
            "cursor should advance after successful apply"
        );
    });
}

#[test]
fn sync_full_three_devices_out_of_order_eventually_converge() {
    let Some(cfg) = MinioE2eConfig::from_env() else {
        eprintln!(
            "skip sync_full_three_devices_out_of_order_eventually_converge: SYNC_MINIO_TEST != 1"
        );
        return;
    };

    let rt = tokio::runtime::Runtime::new().expect("create tokio runtime");
    rt.block_on(async {
        let bucket = create_isolated_bucket(&cfg).await;
        let pool_a = init_test_db();
        let pool_b = init_test_db();
        let pool_c = init_test_db();
        let device_a = format!("a-device-{}", random_suffix(5));
        let device_b = format!("z-device-{}", random_suffix(5));
        let device_c = format!("m-device-{}", random_suffix(5));
        configure_pool(&pool_a, &cfg, &bucket, &device_a);
        configure_pool(&pool_b, &cfg, &bucket, &device_b);
        configure_pool(&pool_c, &cfg, &bucket, &device_c);

        let person_id = format!("e2e-3dev-person-{}", random_suffix(8));
        insert_person(&pool_a, &person_id, "Base");
        sync_full_for_pool(&pool_a).await.expect("A uploads base");
        sync_full_for_pool(&pool_b).await.expect("B pulls base");
        sync_full_for_pool(&pool_c).await.expect("C pulls base");

        // A will become v3, B only v2.
        update_person_with_version_bump(&pool_a, &person_id, "A-v2");
        update_person_with_version_bump(&pool_a, &person_id, "A-v3");
        update_person_with_version_bump(&pool_b, &person_id, "B-v2");
        sync_full_for_pool(&pool_a)
            .await
            .expect("A uploads newer v3");
        sync_full_for_pool(&pool_b)
            .await
            .expect("B uploads older v2 and may pull remote updates");

        // C applies remote deltas ordered by device_id (A before B), then must keep v3.
        sync_full_for_pool(&pool_c)
            .await
            .expect("C applies mixed remote deltas");
        assert_eq!(
            get_person_display_name(&pool_c, &person_id).as_deref(),
            Some("A-v3")
        );
        assert_eq!(get_person_version(&pool_c, &person_id), Some(3));

        // Final convergence check.
        sync_full_for_pool(&pool_a).await.expect("A converge pass");
        sync_full_for_pool(&pool_b).await.expect("B converge pass");
        sync_full_for_pool(&pool_c).await.expect("C converge pass");
        for pool in [&pool_a, &pool_b, &pool_c] {
            assert_eq!(get_person_version(pool, &person_id), Some(3));
            assert_eq!(
                get_person_display_name(pool, &person_id).as_deref(),
                Some("A-v3")
            );
        }
    });
}

#[test]
fn sync_runtime_lock_blocks_manual_sync_until_scheduler_slot_released() {
    let Some(cfg) = MinioE2eConfig::from_env() else {
        eprintln!(
            "skip sync_runtime_lock_blocks_manual_sync_until_scheduler_slot_released: SYNC_MINIO_TEST != 1"
        );
        return;
    };

    let rt = tokio::runtime::Runtime::new().expect("create tokio runtime");
    rt.block_on(async {
        let bucket = create_isolated_bucket(&cfg).await;
        let pool = init_test_db();
        let device_id = format!("e2e-device-lock-{}", random_suffix(6));
        configure_pool(&pool, &cfg, &bucket, &device_id);

        let person_id = format!("e2e-lock-person-{}", random_suffix(8));
        insert_person(&pool, &person_id, "Lock Contention");
        assert_eq!(unsynced_meta_count(&pool, "persons", &person_id), 1);

        let runtime = SyncRuntime::new();
        let hold_runtime = runtime.clone();
        let hold_task = tokio::spawn(async move {
            // Simulate a scheduled sync that currently holds the runtime lock.
            sync_hold_lock_for_test(&hold_runtime, Duration::from_millis(800)).await;
        });

        // Wait until the lock holder really starts (is_syncing is set by helper).
        let mut lock_started = false;
        for _ in 0..50 {
            if runtime.is_syncing() {
                lock_started = true;
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        assert!(lock_started, "runtime lock holder should start in time");

        let pool_manual = pool.clone();
        let runtime_manual = runtime.clone();
        let start = Instant::now();
        let manual_task = tokio::spawn(async move {
            sync_full_with_runtime_for_pool(&pool_manual, &runtime_manual).await
        });
        let manual_result = manual_task.await.expect("manual task join");
        manual_result.expect("manual sync should succeed after lock release");
        let elapsed = start.elapsed();
        assert!(
            elapsed >= Duration::from_millis(500),
            "manual sync should wait for lock holder; elapsed={:?}",
            elapsed
        );

        hold_task.await.expect("lock holder task join");
        assert_eq!(unsynced_meta_count(&pool, "persons", &person_id), 0);
    });
}

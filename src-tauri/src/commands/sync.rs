//! Tauri commands for sync operations

use crate::error::AppError;
use crate::infra::DbPool;
use crate::sync::{DeltaSyncEngine, S3SyncClient, SnapshotManager};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Debug, Deserialize)]
pub struct SyncConfigReq {
    pub enabled: bool,
    pub bucket: String,
    pub endpoint: Option<String>,
    pub access_key: String,
    pub secret_key: String,
}

#[derive(Debug, Serialize)]
pub struct SyncConfigResp {
    pub enabled: bool,
    pub bucket: Option<String>,
    pub endpoint: Option<String>,
    pub device_id: String,
    pub last_sync: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SyncStatusResp {
    pub is_syncing: bool,
    pub pending_changes: i64,
    pub last_sync: Option<String>,
    pub last_error: Option<String>,
}

/// Get current sync configuration
#[tauri::command]
pub fn cmd_sync_get_config(pool: State<DbPool>) -> Result<SyncConfigResp, AppError> {
    let conn = pool
        .inner()
        .0
        .lock()
        .map_err(|e: std::sync::PoisonError<_>| AppError::Db(e.to_string()))?;

    let device_id = get_config_value(&conn, "device_id")?;
    let enabled = get_config_value(&conn, "sync_enabled")? == "1";
    let bucket = get_config_value(&conn, "s3_bucket").ok();
    let endpoint = get_config_value(&conn, "s3_endpoint").ok();
    let last_sync = get_config_value(&conn, "last_sync").ok();

    Ok(SyncConfigResp {
        enabled,
        bucket,
        endpoint,
        device_id,
        last_sync,
    })
}

/// Update sync configuration
#[tauri::command]
pub fn cmd_sync_update_config(pool: State<DbPool>, req: SyncConfigReq) -> Result<String, AppError> {
    let conn = pool
        .inner()
        .0
        .lock()
        .map_err(|e: std::sync::PoisonError<_>| AppError::Db(e.to_string()))?;

    set_config_value(&conn, "sync_enabled", if req.enabled { "1" } else { "0" })?;
    set_config_value(&conn, "s3_bucket", &req.bucket)?;

    if let Some(endpoint) = req.endpoint {
        set_config_value(&conn, "s3_endpoint", &endpoint)?;
    }

    set_config_value(&conn, "s3_access_key", &req.access_key)?;
    set_config_value(&conn, "s3_secret_key", &req.secret_key)?;

    Ok("Sync configuration updated".to_string())
}

/// Get sync status
#[tauri::command]
pub fn cmd_sync_get_status(pool: State<DbPool>) -> Result<SyncStatusResp, AppError> {
    let conn = pool
        .inner()
        .0
        .lock()
        .map_err(|e: std::sync::PoisonError<_>| AppError::Db(e.to_string()))?;

    let pending_changes: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sync_metadata WHERE synced = 0",
            [],
            |row: &rusqlite::Row<'_>| row.get(0),
        )
        .map_err(|e: rusqlite::Error| AppError::Db(e.to_string()))?;

    let last_sync = get_config_value(&conn, "last_sync").ok();
    let last_error = get_config_value(&conn, "last_sync_error").ok();

    Ok(SyncStatusResp {
        is_syncing: false,
        pending_changes,
        last_sync,
        last_error,
    })
}

/// Perform full sync (upload + download)
#[tauri::command]
pub async fn cmd_sync_full(pool: State<'_, DbPool>) -> Result<String, AppError> {
    log::info!("Starting full sync...");

    let pool_ref = pool.inner();

    // Get config
    let (device_id, bucket, endpoint, access_key, secret_key) = {
        let conn = pool_ref
            .0
            .lock()
            .map_err(|e: std::sync::PoisonError<_>| AppError::Db(e.to_string()))?;
        let device_id = get_config_value(&conn, "device_id")?;
        let bucket = get_config_value(&conn, "s3_bucket")?;
        let endpoint = get_config_value(&conn, "s3_endpoint").ok();
        let access_key = get_config_value(&conn, "s3_access_key")?;
        let secret_key = get_config_value(&conn, "s3_secret_key")?;
        (device_id, bucket, endpoint, access_key, secret_key)
    };

    // Create S3 client
    let s3_client = if let Some(endpoint_url) = endpoint {
        S3SyncClient::new_with_endpoint(
            bucket.clone(),
            device_id.clone(),
            endpoint_url,
            access_key,
            secret_key,
        )
        .await
        .map_err(|e| AppError::Db(format!("S3 client error: {}", e)))?
    } else {
        S3SyncClient::new(bucket.clone(), device_id.clone())
            .await
            .map_err(|e| AppError::Db(format!("S3 client error: {}", e)))?
    };

    // Step 1: Upload local delta
    let delta_engine = DeltaSyncEngine::new(pool_ref, device_id.clone());
    let local_delta = delta_engine.collect_local_delta()?;

    if !local_delta.operations.is_empty() {
        log::info!("Uploading {} local changes", local_delta.operations.len());

        let delta_data = local_delta.compress()?;
        let delta_key = format!(
            "deltas/{}/delta-{}.gz",
            device_id,
            chrono::Utc::now().timestamp()
        );

        s3_client
            .upload(&delta_key, delta_data)
            .await
            .map_err(|e| AppError::Db(format!("S3 upload error: {:?}", e)))?;

        // Mark as synced
        let last_id = local_delta.operations.len() as i64;
        delta_engine.mark_synced(last_id)?;
    }

    // Step 2: Download remote deltas
    let remote_deltas = s3_client
        .list("deltas/")
        .await
        .map_err(|e| AppError::Db(format!("S3 list error: {:?}", e)))?;

    log::info!("Found {} remote delta files", remote_deltas.len());

    // TODO: Download and apply remote deltas
    // For now, just log them

    // Step 3: Update last sync time
    {
        let conn = pool_ref
            .0
            .lock()
            .map_err(|e: std::sync::PoisonError<_>| AppError::Db(e.to_string()))?;
        set_config_value(&conn, "last_sync", &chrono::Utc::now().to_rfc3339())?;
        // Clear error
        conn.execute("DELETE FROM sync_config WHERE key = 'last_sync_error'", [])
            .map_err(|e: rusqlite::Error| AppError::Db(e.to_string()))?;
    }

    log::info!("Sync completed successfully");

    Ok("Sync completed".to_string())
}

/// Create and upload snapshot
#[tauri::command]
pub async fn cmd_sync_create_snapshot(pool: State<'_, DbPool>) -> Result<String, AppError> {
    log::info!("Creating snapshot...");

    // Get config
    let pool_ref = pool.inner();
    let (device_id, bucket, endpoint, access_key, secret_key) = {
        let conn = pool_ref
            .0
            .lock()
            .map_err(|e: std::sync::PoisonError<_>| AppError::Db(e.to_string()))?;
        let device_id = get_config_value(&conn, "device_id")?;
        let bucket = get_config_value(&conn, "s3_bucket")?;
        let endpoint = get_config_value(&conn, "s3_endpoint").ok();
        let access_key = get_config_value(&conn, "s3_access_key")?;
        let secret_key = get_config_value(&conn, "s3_secret_key")?;
        (device_id, bucket, endpoint, access_key, secret_key)
    };

    // Create S3 client
    let s3_client = if let Some(endpoint_url) = endpoint {
        S3SyncClient::new_with_endpoint(
            bucket.clone(),
            device_id.clone(),
            endpoint_url,
            access_key,
            secret_key,
        )
        .await
        .map_err(|e| AppError::Db(format!("S3 client error: {}", e)))?
    } else {
        S3SyncClient::new(bucket.clone(), device_id.clone())
            .await
            .map_err(|e| AppError::Db(format!("S3 client error: {}", e)))?
    };

    //Create snapshot
    let snapshot_mgr = SnapshotManager::new(pool_ref, device_id.clone());
    let snapshot = snapshot_mgr.create_snapshot()?;

    // Upload snapshot
    let snapshot_data = snapshot.compress()?;
    let snapshot_key = format!("snapshots/latest-{}.gz", device_id);

    s3_client
        .upload(&snapshot_key, snapshot_data)
        .await
        .map_err(|e| AppError::Db(format!("S3 upload error: {:?}", e)))?;

    log::info!("Snapshot uploaded: {}", snapshot_key);

    Ok(format!("Snapshot created: {}", snapshot.checksum))
}

/// Download and restore from latest snapshot
#[tauri::command]
pub async fn cmd_sync_restore_snapshot(pool: State<'_, DbPool>) -> Result<String, AppError> {
    log::info!("Restoring from snapshot...");

    // Get config
    let pool_ref = pool.inner();
    let (device_id, bucket, endpoint, access_key, secret_key) = {
        let conn = pool_ref
            .0
            .lock()
            .map_err(|e: std::sync::PoisonError<_>| AppError::Db(e.to_string()))?;
        let device_id = get_config_value(&conn, "device_id")?;
        let bucket = get_config_value(&conn, "s3_bucket")?;
        let endpoint = get_config_value(&conn, "s3_endpoint").ok();
        let access_key = get_config_value(&conn, "s3_access_key")?;
        let secret_key = get_config_value(&conn, "s3_secret_key")?;
        (device_id, bucket, endpoint, access_key, secret_key)
    };

    // Create S3 client
    let s3_client = if let Some(endpoint_url) = endpoint {
        S3SyncClient::new_with_endpoint(
            bucket.clone(),
            device_id.clone(),
            endpoint_url,
            access_key,
            secret_key,
        )
        .await
        .map_err(|e| AppError::Db(format!("S3 client error: {}", e)))?
    } else {
        S3SyncClient::new(bucket.clone(), device_id.clone())
            .await
            .map_err(|e| AppError::Db(format!("S3 client error: {}", e)))?
    };

    // List snapshots
    let snapshots = s3_client
        .list("snapshots/")
        .await
        .map_err(|e| AppError::Db(format!("S3 list error: {:?}", e)))?;

    if snapshots.is_empty() {
        return Err(AppError::Db("No snapshots found".to_string()));
    }

    // Use latest
    let latest_key = snapshots.last().unwrap();
    log::info!("Downloading snapshot: {}", latest_key);

    let snapshot_data = s3_client
        .download(latest_key)
        .await
        .map_err(|e| AppError::Db(format!("S3 download error: {:?}", e)))?;

    // Decompress and restore
    use crate::sync::snapshot::Snapshot;
    let snapshot = Snapshot::decompress(&snapshot_data)?;

    let snapshot_mgr = SnapshotManager::new(pool_ref, device_id);
    snapshot_mgr.restore_snapshot(&snapshot)?;

    log::info!("Snapshot restored successfully");

    Ok(format!("Restored from snapshot: {}", snapshot.checksum))
}

// Helper functions

fn get_config_value(conn: &Connection, key: &str) -> Result<String, AppError> {
    conn.query_row(
        "SELECT value FROM sync_config WHERE key = ?1",
        [key],
        |row: &rusqlite::Row<'_>| row.get(0),
    )
    .map_err(|e: rusqlite::Error| AppError::Db(format!("Config key '{}' not found: {}", key, e)))
}

fn set_config_value(conn: &Connection, key: &str, value: &str) -> Result<(), AppError> {
    conn.execute(
        "INSERT OR REPLACE INTO sync_config (key, value) VALUES (?1, ?2)",
        [key, value],
    )
    .map_err(|e: rusqlite::Error| AppError::Db(e.to_string()))?;

    Ok(())
}

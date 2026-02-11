//! Tauri commands for sync operations

use crate::error::AppError;
use crate::infra::DbPool;
use crate::sync::{DeltaSyncEngine, S3SyncClient, SnapshotManager};
use aws_sdk_s3::error::ProvideErrorMetadata;
use aws_sdk_s3::error::SdkError;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::error::Error as StdError;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::State;
use tokio::sync::Mutex as AsyncMutex;
use tokio::task::JoinHandle;
use tokio::time::{sleep, Duration};

#[derive(Clone)]
pub struct SyncRuntime {
    inner: Arc<SyncRuntimeInner>,
}

struct SyncRuntimeInner {
    sync_lock: AsyncMutex<()>,
    is_syncing: AtomicBool,
    scheduler_handle: AsyncMutex<Option<JoinHandle<()>>>,
}

impl SyncRuntime {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(SyncRuntimeInner {
                sync_lock: AsyncMutex::new(()),
                is_syncing: AtomicBool::new(false),
                scheduler_handle: AsyncMutex::new(None),
            }),
        }
    }

    pub fn is_syncing(&self) -> bool {
        self.inner.is_syncing.load(Ordering::Relaxed)
    }

    pub async fn stop_scheduler(&self) {
        let mut guard = self.inner.scheduler_handle.lock().await;
        if let Some(handle) = guard.take() {
            handle.abort();
        }
        // Best-effort: if we aborted during a sync, clear the flag to avoid stale UI state.
        self.inner.is_syncing.store(false, Ordering::Relaxed);
    }

    pub async fn refresh_scheduler(&self, pool: DbPool) {
        // Always stop first to ensure only one scheduler is alive.
        self.stop_scheduler().await;

        let enabled = {
            let conn = match pool.0.lock() {
                Ok(c) => c,
                Err(poisoned) => {
                    log::error!("DB lock poisoned when refreshing scheduler: {}", poisoned);
                    return;
                }
            };
            get_config_value(&conn, "sync_enabled").ok().as_deref() == Some("1")
        };

        if !enabled {
            return;
        }

        let runtime = self.clone();
        let mut guard = self.inner.scheduler_handle.lock().await;
        *guard = Some(tokio::spawn(async move {
            loop {
                let (enabled, minutes) = match pool.0.lock() {
                    Ok(conn) => {
                        let enabled =
                            get_config_value(&conn, "sync_enabled").ok().as_deref() == Some("1");
                        let minutes = get_config_value(&conn, "auto_sync_interval_minutes")
                            .ok()
                            .and_then(|v| v.trim().parse::<i64>().ok())
                            .filter(|v| *v >= 1)
                            .unwrap_or(1);
                        (enabled, minutes)
                    }
                    Err(poisoned) => {
                        log::error!("DB lock poisoned in scheduler loop: {}", poisoned);
                        // Backoff; keep the scheduler alive.
                        (false, 1)
                    }
                };

                if !enabled {
                    log::info!("Sync scheduler exiting (sync disabled)");
                    break;
                }

                // Prevent concurrent scheduled/manual sync.
                let _lock = runtime.inner.sync_lock.lock().await;
                runtime.inner.is_syncing.store(true, Ordering::Relaxed);

                let res = sync_full_impl(&pool).await;
                if let Err(e) = res {
                    log::error!("Scheduled sync failed: {}", e);
                }

                runtime.inner.is_syncing.store(false, Ordering::Relaxed);

                let secs = (minutes.max(1) as u64) * 60;
                sleep(Duration::from_secs(secs)).await;
            }
        }));
    }
}

#[derive(Debug, Deserialize)]
pub struct SyncConfigReq {
    pub enabled: bool,
    pub bucket: String,
    pub endpoint: Option<String>,
    pub access_key: Option<String>,
    pub secret_key: Option<String>,
    /// Auto sync interval in minutes. If omitted, keep existing value.
    pub auto_sync_interval_minutes: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct SyncEnableReq {
    pub enabled: bool,
}

#[derive(Debug, Serialize)]
pub struct SyncConfigResp {
    pub enabled: bool,
    pub bucket: Option<String>,
    pub endpoint: Option<String>,
    pub access_key: Option<String>,
    pub has_secret_key: bool,
    pub secret_key_masked: Option<String>,
    pub device_id: String,
    pub last_sync: Option<String>,
    /// Auto sync interval in minutes (>= 1).
    pub auto_sync_interval_minutes: i64,
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
    let access_key = get_config_value(&conn, "s3_access_key").ok();
    let secret_key = get_config_value(&conn, "s3_secret_key").ok();
    let has_secret_key = secret_key
        .as_deref()
        .map(|v| !v.trim().is_empty())
        .unwrap_or(false);
    let secret_key_masked = secret_key
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(mask_credential);
    let last_sync = get_config_value(&conn, "last_sync").ok();
    let auto_sync_interval_minutes = get_config_value(&conn, "auto_sync_interval_minutes")
        .ok()
        .and_then(|v| v.trim().parse::<i64>().ok())
        .filter(|v| *v >= 1)
        .unwrap_or(1);

    Ok(SyncConfigResp {
        enabled,
        bucket,
        endpoint,
        access_key,
        has_secret_key,
        secret_key_masked,
        device_id,
        last_sync,
        auto_sync_interval_minutes,
    })
}

/// Update sync configuration
#[tauri::command]
pub async fn cmd_sync_update_config(
    pool: State<'_, DbPool>,
    runtime: State<'_, SyncRuntime>,
    req: SyncConfigReq,
) -> Result<String, AppError> {
    {
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

        // Security/UX: do not overwrite existing credentials with empty strings.
        // - If frontend omits the field, keep existing value.
        // - If frontend sends Some("") (unlikely), also keep existing value.
        if let Some(access_key) = req
            .access_key
            .as_deref()
            .map(str::trim)
            .filter(|v| !v.is_empty())
        {
            set_config_value(&conn, "s3_access_key", access_key)?;
        }
        if let Some(secret_key) = req
            .secret_key
            .as_deref()
            .map(str::trim)
            .filter(|v| !v.is_empty())
        {
            set_config_value(&conn, "s3_secret_key", secret_key)?;
        }

        if let Some(minutes) = req.auto_sync_interval_minutes {
            let minutes = minutes.max(1);
            set_config_value(&conn, "auto_sync_interval_minutes", &minutes.to_string())?;
        }
    } // Drop DB lock before await (Tauri commands require Send futures).

    // Backend timer: restart scheduler to apply new interval / enabled flag.
    runtime.refresh_scheduler(pool.inner().clone()).await;

    Ok("Sync configuration updated".to_string())
}

/// Enable/disable sync (independent from config editing).
/// When enabling, validate required S3 config exists.
#[tauri::command]
pub async fn cmd_sync_set_enabled(
    pool: State<'_, DbPool>,
    runtime: State<'_, SyncRuntime>,
    req: SyncEnableReq,
) -> Result<String, AppError> {
    {
        let conn = pool
            .inner()
            .0
            .lock()
            .map_err(|e: std::sync::PoisonError<_>| AppError::Db(e.to_string()))?;

        if req.enabled {
            let bucket_ok = get_config_value(&conn, "s3_bucket")
                .ok()
                .map(|v| !v.trim().is_empty())
                .unwrap_or(false);
            let access_ok = get_config_value(&conn, "s3_access_key")
                .ok()
                .map(|v| !v.trim().is_empty())
                .unwrap_or(false);
            let secret_ok = get_config_value(&conn, "s3_secret_key")
                .ok()
                .map(|v| !v.trim().is_empty())
                .unwrap_or(false);

            if !bucket_ok || !access_ok || !secret_ok {
                return Err(AppError::SyncConfigIncomplete);
            }
        }

        set_config_value(&conn, "sync_enabled", if req.enabled { "1" } else { "0" })?;
    } // Drop DB lock before await.

    runtime.refresh_scheduler(pool.inner().clone()).await;
    Ok("Sync enabled updated".to_string())
}

/// Test S3 connectivity and permissions.
#[tauri::command]
pub async fn cmd_sync_test_connection(pool: State<'_, DbPool>) -> Result<String, AppError> {
    let pool_ref = pool.inner();

    // Get config
    let (bucket, endpoint, access_key, secret_key) = {
        let conn = pool_ref
            .0
            .lock()
            .map_err(|e: std::sync::PoisonError<_>| AppError::Db(e.to_string()))?;
        let bucket = get_config_value(&conn, "s3_bucket").ok();
        let endpoint = get_config_value(&conn, "s3_endpoint").ok();
        let access_key = get_config_value(&conn, "s3_access_key").ok();
        let secret_key = get_config_value(&conn, "s3_secret_key").ok();
        (bucket, endpoint, access_key, secret_key)
    };

    let bucket = bucket.unwrap_or_default().trim().to_string();
    let access_key = access_key.unwrap_or_default().trim().to_string();
    let secret_key = secret_key.unwrap_or_default().trim().to_string();

    if bucket.is_empty() || access_key.is_empty() || secret_key.is_empty() {
        return Err(AppError::SyncConfigIncomplete);
    }

    // Reuse device_id only for namespacing; not required for the test itself.
    let device_id = {
        let conn = pool_ref
            .0
            .lock()
            .map_err(|e: std::sync::PoisonError<_>| AppError::Db(e.to_string()))?;
        get_config_value(&conn, "device_id")?
    };

    let s3_client = if let Some(endpoint_url) = endpoint {
        S3SyncClient::new_with_endpoint(
            bucket.clone(),
            device_id,
            endpoint_url,
            access_key,
            secret_key,
        )
        .await
        .map_err(|e| AppError::Sync(format!("S3 client error: {}", e)))?
    } else {
        // No custom endpoint: rely on environment credentials.
        S3SyncClient::new(bucket.clone(), device_id)
            .await
            .map_err(|e| AppError::Sync(format!("S3 client error: {}", e)))?
    };

    s3_client
        .test_connection()
        .await
        .map_err(|e| map_s3_error("test", e))?;

    Ok("Connection OK".to_string())
}

/// Get sync status
#[tauri::command]
pub fn cmd_sync_get_status(
    pool: State<DbPool>,
    runtime: State<SyncRuntime>,
) -> Result<SyncStatusResp, AppError> {
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
        is_syncing: runtime.is_syncing(),
        pending_changes,
        last_sync,
        last_error,
    })
}

/// Perform full sync (upload + download)
#[tauri::command]
pub async fn cmd_sync_full(
    pool: State<'_, DbPool>,
    runtime: State<'_, SyncRuntime>,
) -> Result<String, AppError> {
    let _lock = runtime.inner.sync_lock.lock().await;
    runtime.inner.is_syncing.store(true, Ordering::Relaxed);
    let res = sync_full_impl(pool.inner()).await;
    runtime.inner.is_syncing.store(false, Ordering::Relaxed);
    res
}

async fn sync_full_impl(pool_ref: &DbPool) -> Result<String, AppError> {
    let res: Result<String, AppError> = (async {
        log::info!("Starting full sync...");

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
        let local_collected = delta_engine.collect_local_delta()?;
        let has_local_delta = !local_collected.delta.operations.is_empty();

        if has_local_delta {
            log::info!(
                "Uploading {} local changes",
                local_collected.delta.operations.len()
            );

            let delta_data = local_collected.delta.compress()?;
            let delta_key = format!(
                "deltas/{}/delta-{}.gz",
                device_id,
                chrono::Utc::now().timestamp()
            );

            s3_client
                .upload(&delta_key, delta_data)
                .await
                .map_err(|e| {
                    // Log full debug info, but return a concise message to the UI.
                    log::error!("S3 upload error: {:?}", e);
                    map_s3_error("upload", e)
                })?;

            // Mark as synced (by sync_metadata.id, not by count)
            if let Some(max_id) = local_collected.max_sync_meta_id {
                delta_engine.mark_synced(max_id)?;
            }
        } else {
            log::info!("No local delta changes to upload");
        }

        // Bootstrap: if there are no deltas to upload and remote is empty, upload a snapshot once.
        // This avoids the confusing "sync succeeded but bucket is empty" experience.
        if !has_local_delta {
            let remote_snapshots = s3_client.list("snapshots/").await.map_err(|e| {
                log::error!("S3 list snapshots error: {:?}", e);
                map_s3_error("list", e)
            })?;

            let remote_deltas = s3_client.list("deltas/").await.map_err(|e| {
                log::error!("S3 list deltas error: {:?}", e);
                map_s3_error("list", e)
            })?;

            if remote_snapshots.is_empty() && remote_deltas.is_empty() {
                log::info!("Remote empty, uploading initial snapshot for bootstrap");
                let snapshot_mgr = SnapshotManager::new(pool_ref, device_id.clone());
                let snapshot = snapshot_mgr.create_snapshot()?;
                let snapshot_data = snapshot.compress()?;
                let snapshot_key = format!("snapshots/latest-{}.gz", device_id);

                s3_client
                    .upload(&snapshot_key, snapshot_data)
                    .await
                    .map_err(|e| {
                        log::error!("S3 snapshot upload error: {:?}", e);
                        map_s3_error("upload", e)
                    })?;

                log::info!(
                    "Bootstrap snapshot uploaded: {} (checksum {})",
                    snapshot_key,
                    snapshot.checksum
                );
            } else {
                log::info!(
                    "Remote not empty (snapshots: {}, deltas: {}), skipping bootstrap snapshot",
                    remote_snapshots.len(),
                    remote_deltas.len()
                );
            }
        }

        // Step 2: Download remote deltas
        let remote_deltas = s3_client.list("deltas/").await.map_err(|e| {
            log::error!("S3 list error: {:?}", e);
            map_s3_error("list", e)
        })?;

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
    })
    .await;

    if let Err(e) = &res {
        // Best-effort error recording for UI.
        if let Ok(conn) = pool_ref.0.lock() {
            let _ = set_config_value(&conn, "last_sync_error", &e.to_string());
        }
    }

    res
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
        .map_err(|e| {
            log::error!("S3 upload error: {:?}", e);
            map_s3_error("upload", e)
        })?;

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
    let snapshots = s3_client.list("snapshots/").await.map_err(|e| {
        log::error!("S3 list error: {:?}", e);
        map_s3_error("list", e)
    })?;

    if snapshots.is_empty() {
        return Err(AppError::Db("No snapshots found".to_string()));
    }

    // Use latest
    let latest_key = snapshots.last().unwrap();
    log::info!("Downloading snapshot: {}", latest_key);

    let snapshot_data = s3_client.download(latest_key).await.map_err(|e| {
        log::error!("S3 download error: {:?}", e);
        map_s3_error("download", e)
    })?;

    // Decompress and restore
    use crate::sync::snapshot::Snapshot;
    let snapshot = Snapshot::decompress(&snapshot_data)?;

    let snapshot_mgr = SnapshotManager::new(pool_ref, device_id);
    snapshot_mgr.restore_snapshot(&snapshot)?;

    log::info!("Snapshot restored successfully");

    Ok(format!("Restored from snapshot: {}", snapshot.checksum))
}

/// Reveal the stored secret key (use with caution).
#[tauri::command]
pub fn cmd_sync_reveal_secret_key(pool: State<DbPool>) -> Result<String, AppError> {
    let conn = pool
        .inner()
        .0
        .lock()
        .map_err(|e: std::sync::PoisonError<_>| AppError::Db(e.to_string()))?;

    let secret_key = get_config_value(&conn, "s3_secret_key")?;
    let secret_key = secret_key.trim().to_string();
    if secret_key.is_empty() {
        return Err(AppError::Db("Secret key is not set".to_string()));
    }

    Ok(secret_key)
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

fn map_s3_error(op: &str, err: Box<dyn StdError>) -> AppError {
    if let Some((code, message)) = extract_s3_error_code_message(err.as_ref()) {
        let msg = message.trim();
        if !msg.is_empty() {
            // Return server message directly for UI display.
            return AppError::Sync(format!("[{}] {}", code, msg));
        }
        return AppError::Sync(format!("[{}] {}", code, err));
    }

    AppError::Sync(format!("S3 {} failed: {}", op, err))
}

fn extract_s3_error_code_message(err: &(dyn StdError + 'static)) -> Option<(String, String)> {
    use aws_sdk_s3::operation::{
        get_object::GetObjectError, list_objects_v2::ListObjectsV2Error, put_object::PutObjectError,
    };

    // NOTE: we must extract code/message from structured metadata, NOT from Display/Debug strings.
    fn from_sdk_error<E>(e: &SdkError<E>) -> Option<(String, String)>
    where
        E: std::error::Error + Send + Sync + 'static + ProvideErrorMetadata,
    {
        match e {
            SdkError::ServiceError(se) => {
                // Most generated service errors provide `.meta()` for ErrorMetadata.
                let meta = se.err().meta();
                let code = meta.code()?.to_string();
                let message = meta.message().unwrap_or_default().to_string();
                Some((code, message))
            }
            _ => None,
        }
    }

    if let Some(e) = err.downcast_ref::<SdkError<ListObjectsV2Error>>() {
        return from_sdk_error(e);
    }
    if let Some(e) = err.downcast_ref::<SdkError<PutObjectError>>() {
        return from_sdk_error(e);
    }
    if let Some(e) = err.downcast_ref::<SdkError<GetObjectError>>() {
        return from_sdk_error(e);
    }

    None
}

fn mask_credential(value: &str) -> String {
    // Common UX: show prefix + "***" + suffix, without revealing the full secret.
    // Keys are ASCII in practice; bytes-based masking is fine here.
    let s = value.as_bytes();
    if s.is_empty() {
        return String::new();
    }

    let head = 3usize.min(s.len());
    let tail = 3usize.min(s.len().saturating_sub(head));
    if head + tail >= s.len() {
        // Too short: show first and last only.
        if s.len() == 1 {
            return "*".to_string();
        }
        let first = s[0] as char;
        let last = s[s.len() - 1] as char;
        return format!("{}***{}", first, last);
    }

    let prefix = String::from_utf8_lossy(&s[..head]);
    let suffix = String::from_utf8_lossy(&s[s.len() - tail..]);
    format!("{}***{}", prefix, suffix)
}

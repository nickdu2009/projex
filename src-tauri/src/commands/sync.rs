//! Tauri commands for sync operations

use crate::error::AppError;
use crate::infra::DbPool;
use crate::sync::{Delta, DeltaSyncEngine, S3ObjectSummary, S3SyncClient, SnapshotManager};
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
use uuid::Uuid;

/// Injected S3 credentials for Android (from Keystore).
/// On desktop the credentials are read from SQLite sync_config directly.
#[cfg(target_os = "android")]
pub struct SyncCredentials {
    pub access_key: String,
    pub secret_key: String,
}

/// Validate that endpoint is HTTPS (required on Android).
/// Returns Ok(()) if endpoint is None (falls back to AWS default) or starts with "https://".
#[cfg(target_os = "android")]
pub fn validate_endpoint_https(endpoint: &Option<String>) -> Result<(), AppError> {
    if let Some(ep) = endpoint {
        let ep = ep.trim();
        if !ep.is_empty() && !ep.to_ascii_lowercase().starts_with("https://") {
            return Err(AppError::Validation(
                "ENDPOINT_NOT_HTTPS: endpoint must use https:// on Android".to_string(),
            ));
        }
    }
    Ok(())
}

/// Outcome of a background sync attempt triggered by Android WorkManager.
#[cfg(target_os = "android")]
#[derive(Debug, Serialize)]
pub struct AndroidSyncResult {
    /// "ok" | "skipped" | "failed"
    pub status: String,
    pub message: String,
}

/// Android background sync entry point called from JNI.
///
/// 设计要点：
/// - 凭据从 SQLite sync_config 读取（与桌面一致）。
/// - 使用 data_dir 下的文件锁（sync.lock）互斥后台与前台同步，拿不到锁即跳过。
/// - HTTPS-only 校验：endpoint 若为 http:// 则直接返回错误。
#[cfg(target_os = "android")]
pub async fn android_run_sync_once(pool_ref: &DbPool) -> AndroidSyncResult {
    use fs2::FileExt;
    use std::fs::OpenOptions;

    // 1. Check sync_enabled in SQLite
    let sync_enabled = {
        match pool_ref.0.lock() {
            Ok(conn) => {
                get_config_value(&conn, "sync_enabled")
                    .ok()
                    .as_deref()
                    .unwrap_or("0")
                    .trim()
                    == "1"
            }
            Err(_) => false,
        }
    };
    if !sync_enabled {
        log::info!("[android_sync] sync_enabled=0, skipping");
        return AndroidSyncResult {
            status: "skipped".to_string(),
            message: "sync disabled".to_string(),
        };
    }

    // 3. Acquire file lock (sync.lock) for cross-process mutual exclusion.
    //    data_dir is derived from the same dirs crate path as the Tauri app.
    let lock_path = {
        let base = dirs::data_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
        base.join("com.nickdu.projex")
            .join("default")
            .join("sync.lock")
    };
    if let Some(parent) = lock_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let lock_file = match OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(false)
        .open(&lock_path)
    {
        Ok(f) => f,
        Err(e) => {
            log::warn!("[android_sync] cannot open lock file: {}", e);
            return AndroidSyncResult {
                status: "skipped".to_string(),
                message: format!("lock file unavailable: {}", e),
            };
        }
    };
    if lock_file.try_lock_exclusive().is_err() {
        log::info!("[android_sync] lock held by foreground, skipping this cycle");
        return AndroidSyncResult {
            status: "skipped".to_string(),
            message: "sync already running".to_string(),
        };
    }

    // 4. Read config from SQLite (including credentials, same as desktop)
    let (device_id_opt, bucket_opt, endpoint, access_key, secret_key) = {
        match pool_ref.0.lock() {
            Ok(conn) => {
                let device_id = get_config_value(&conn, "device_id").ok();
                let bucket = get_config_value(&conn, "s3_bucket").ok();
                let endpoint = get_config_value(&conn, "s3_endpoint").ok();
                let access_key = get_config_value(&conn, "s3_access_key").ok();
                let secret_key = get_config_value(&conn, "s3_secret_key").ok();
                (device_id, bucket, endpoint, access_key, secret_key)
            }
            Err(_) => {
                return AndroidSyncResult {
                    status: "failed".to_string(),
                    message: "db lock poisoned".to_string(),
                };
            }
        }
    };

    let device_id = match device_id_opt {
        Some(v) if !v.trim().is_empty() => v,
        _ => {
            return AndroidSyncResult {
                status: "failed".to_string(),
                message: "device_id not configured".to_string(),
            };
        }
    };
    let bucket = match bucket_opt {
        Some(v) if !v.trim().is_empty() => v,
        _ => {
            return AndroidSyncResult {
                status: "failed".to_string(),
                message: "s3_bucket not configured".to_string(),
            };
        }
    };
    let (access_key, secret_key) = match (access_key, secret_key) {
        (Some(ak), Some(sk)) if !ak.trim().is_empty() && !sk.trim().is_empty() => (ak, sk),
        _ => {
            return AndroidSyncResult {
                status: "skipped".to_string(),
                message: "credentials not configured".to_string(),
            };
        }
    };

    // 5. HTTPS-only enforcement for Android
    if let Err(e) = validate_endpoint_https(&endpoint) {
        log::error!("[android_sync] {}", e);
        if let Ok(conn) = pool_ref.0.lock() {
            let _ = set_config_value(&conn, "last_sync_error", &e.to_string());
        }
        return AndroidSyncResult {
            status: "failed".to_string(),
            message: e.to_string(),
        };
    }

    // 6. Run the actual sync pipeline (credentials from SQLite)
    log::info!("[android_sync] starting sync for device={}", device_id);
    let result = sync_full_impl_with_creds(
        pool_ref,
        device_id,
        bucket,
        endpoint,
        SyncCredentials {
            access_key,
            secret_key,
        },
    )
    .await;

    match result {
        Ok(msg) => AndroidSyncResult {
            status: "ok".to_string(),
            message: msg,
        },
        Err(e) => AndroidSyncResult {
            status: "failed".to_string(),
            message: e.to_string(),
        },
    }
}

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

                let res = sync_full_with_runtime_for_pool(&pool, &runtime).await;
                if let Err(e) = res {
                    log::error!("Scheduled sync failed: {}", e);
                }

                let secs = (minutes.max(1) as u64) * 60;
                sleep(Duration::from_secs(secs)).await;
            }
        }));
    }
}

impl Default for SyncRuntime {
    fn default() -> Self {
        Self::new()
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

#[derive(Debug, Deserialize)]
pub struct SyncTestConnectionReq {
    pub bucket: Option<String>,
    pub endpoint: Option<String>,
    pub access_key: Option<String>,
    pub secret_key: Option<String>,
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

        if let Some(ref endpoint) = req.endpoint {
            // On Android, reject http:// endpoints at the Rust layer.
            // This is the authoritative guard; the frontend also validates.
            #[cfg(target_os = "android")]
            validate_endpoint_https(&Some(endpoint.clone()))?;

            set_config_value(&conn, "s3_endpoint", endpoint)?;
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
pub async fn cmd_sync_test_connection(
    pool: State<'_, DbPool>,
    req: Option<SyncTestConnectionReq>,
) -> Result<String, AppError> {
    let pool_ref = pool.inner();
    let req = req.unwrap_or(SyncTestConnectionReq {
        bucket: None,
        endpoint: None,
        access_key: None,
        secret_key: None,
    });

    // Get config
    let (saved_bucket, saved_endpoint, saved_access_key, saved_secret_key) = {
        let conn = pool_ref
            .0
            .lock()
            .map_err(|e: std::sync::PoisonError<_>| AppError::Db(e.to_string()))?;
        (
            get_config_value(&conn, "s3_bucket").ok(),
            get_config_value(&conn, "s3_endpoint").ok(),
            get_config_value(&conn, "s3_access_key").ok(),
            get_config_value(&conn, "s3_secret_key").ok(),
        )
    };

    // Prefer request draft values (for unsaved form testing), fallback to persisted values.
    let bucket = req
        .bucket
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(ToString::to_string)
        .or_else(|| {
            saved_bucket
                .as_deref()
                .map(str::trim)
                .filter(|v| !v.is_empty())
                .map(ToString::to_string)
        })
        .unwrap_or_default();

    let endpoint = req
        .endpoint
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(ToString::to_string)
        .or_else(|| {
            saved_endpoint
                .as_deref()
                .map(str::trim)
                .filter(|v| !v.is_empty())
                .map(ToString::to_string)
        });

    let access_key = req
        .access_key
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(ToString::to_string)
        .or_else(|| {
            saved_access_key
                .as_deref()
                .map(str::trim)
                .filter(|v| !v.is_empty())
                .map(ToString::to_string)
        })
        .unwrap_or_default();

    let secret_key = req
        .secret_key
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(ToString::to_string)
        .or_else(|| {
            saved_secret_key
                .as_deref()
                .map(str::trim)
                .filter(|v| !v.is_empty())
                .map(ToString::to_string)
        })
        .unwrap_or_default();

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
    sync_full_with_runtime_for_pool(pool.inner(), runtime.inner()).await
}

/// Execute full sync pipeline with runtime lock protection.
/// This ensures scheduled/manual sync calls never run concurrently.
pub async fn sync_full_with_runtime_for_pool(
    pool_ref: &DbPool,
    runtime: &SyncRuntime,
) -> Result<String, AppError> {
    let _lock = runtime.inner.sync_lock.lock().await;
    runtime.inner.is_syncing.store(true, Ordering::Relaxed);
    let res = sync_full_impl(pool_ref).await;
    runtime.inner.is_syncing.store(false, Ordering::Relaxed);
    res
}

/// Execute full sync pipeline for a database pool.
/// This entry is used by command runtime and integration tests.
pub async fn sync_full_for_pool(pool_ref: &DbPool) -> Result<String, AppError> {
    sync_full_impl(pool_ref).await
}

/// Test helper: hold the runtime sync lock for a fixed duration.
/// Used to verify scheduler/manual contention behavior in integration tests.
pub async fn sync_hold_lock_for_test(runtime: &SyncRuntime, hold_for: Duration) {
    let _lock = runtime.inner.sync_lock.lock().await;
    runtime.inner.is_syncing.store(true, Ordering::Relaxed);
    sleep(hold_for).await;
    runtime.inner.is_syncing.store(false, Ordering::Relaxed);
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

        sync_full_pipeline(
            pool_ref, device_id, bucket, endpoint, access_key, secret_key,
        )
        .await
    })
    .await;

    if let Err(e) = &res {
        if let Ok(conn) = pool_ref.0.lock() {
            let _ = set_config_value(&conn, "last_sync_error", &e.to_string());
        }
    }

    res
}

/// Full sync with externally supplied credentials (Android: from Keystore).
#[cfg(target_os = "android")]
async fn sync_full_impl_with_creds(
    pool_ref: &DbPool,
    device_id: String,
    bucket: String,
    endpoint: Option<String>,
    creds: SyncCredentials,
) -> Result<String, AppError> {
    let res = sync_full_pipeline(
        pool_ref,
        device_id,
        bucket,
        endpoint,
        creds.access_key,
        creds.secret_key,
    )
    .await;

    if let Err(e) = &res {
        if let Ok(conn) = pool_ref.0.lock() {
            let _ = set_config_value(&conn, "last_sync_error", &e.to_string());
        }
    }

    res
}

/// Core sync pipeline: upload local delta, bootstrap snapshot, download & apply remote deltas.
/// Called by both the desktop path (credentials from SQLite) and the Android path (credentials injected).
async fn sync_full_pipeline(
    pool_ref: &DbPool,
    device_id: String,
    bucket: String,
    endpoint: Option<String>,
    access_key: String,
    secret_key: String,
) -> Result<String, AppError> {
    let res: Result<String, AppError> = (async {
        log::info!("Starting full sync...");

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
                "deltas/{}/delta-{}-{}.gz",
                device_id,
                chrono::Utc::now()
                    .timestamp_nanos_opt()
                    .unwrap_or_else(|| chrono::Utc::now().timestamp_micros() * 1_000),
                Uuid::new_v4()
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

        // Step 2: Download and apply remote deltas
        let remote_delta_keys = s3_client.list("deltas/").await.map_err(|e| {
            log::error!("S3 list error: {:?}", e);
            map_s3_error("list", e)
        })?;

        let mut remote_delta_candidates = Vec::new();
        {
            let conn = pool_ref
                .0
                .lock()
                .map_err(|e: std::sync::PoisonError<_>| AppError::Db(e.to_string()))?;

            for key in remote_delta_keys {
                match parse_remote_delta_object(&key) {
                    Some(remote_delta) => {
                        if remote_delta.source_device_id == device_id {
                            continue;
                        }

                        let cursor_ts = get_remote_delta_cursor_timestamp(
                            &conn,
                            &remote_delta.source_device_id,
                        )?
                        .unwrap_or(0);
                        if remote_delta.timestamp > cursor_ts {
                            remote_delta_candidates.push(remote_delta);
                        }
                    }
                    None => {
                        log::warn!("Skip unsupported delta key format: {}", key);
                    }
                }
            }
        }

        remote_delta_candidates.sort_by(|a, b| {
            a.source_device_id
                .cmp(&b.source_device_id)
                .then(a.timestamp.cmp(&b.timestamp))
                .then(a.key.cmp(&b.key))
        });

        log::info!(
            "Remote delta files pending apply: {}",
            remote_delta_candidates.len()
        );

        let mut applied_remote_delta_count = 0usize;
        for remote in remote_delta_candidates {
            let delta_data = s3_client.download(&remote.key).await.map_err(|e| {
                log::error!("S3 download error for {}: {:?}", remote.key, e);
                map_s3_error("download", e)
            })?;

            let delta = Delta::decompress(&delta_data)?;
            let calculated_checksum = Delta::calculate_checksum(&delta.operations);
            if calculated_checksum != delta.checksum {
                return Err(AppError::Sync(format!(
                    "Checksum mismatch for remote delta {}",
                    remote.key
                )));
            }

            let before_apply_sync_meta_id = delta_engine.current_max_sync_metadata_id()?;
            delta_engine.apply_delta(&delta)?;
            let marked = delta_engine.mark_remote_applied_operations_synced(
                before_apply_sync_meta_id,
                &delta.operations,
            )?;

            {
                let conn = pool_ref
                    .0
                    .lock()
                    .map_err(|e: std::sync::PoisonError<_>| AppError::Db(e.to_string()))?;
                set_remote_delta_cursor_timestamp(
                    &conn,
                    &remote.source_device_id,
                    remote.timestamp,
                )?;
            }

            applied_remote_delta_count += 1;
            log::info!(
                "Applied remote delta {} from {}, marked {} local metadata rows as synced",
                remote.key,
                remote.source_device_id,
                marked
            );
        }

        log::info!("Applied {} remote delta files", applied_remote_delta_count);

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

    res
}

/// Create and upload snapshot
#[tauri::command]
pub async fn cmd_sync_create_snapshot(pool: State<'_, DbPool>) -> Result<String, AppError> {
    sync_create_snapshot_for_pool(pool.inner()).await
}

/// Execute snapshot creation/upload pipeline for a database pool.
/// This entry is used by command runtime and integration tests.
pub async fn sync_create_snapshot_for_pool(pool_ref: &DbPool) -> Result<String, AppError> {
    sync_create_snapshot_impl(pool_ref).await
}

async fn sync_create_snapshot_impl(pool_ref: &DbPool) -> Result<String, AppError> {
    log::info!("Creating snapshot...");

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
    sync_restore_snapshot_for_pool(pool.inner()).await
}

/// Execute snapshot restore pipeline for a database pool.
/// This entry is used by command runtime and integration tests.
pub async fn sync_restore_snapshot_for_pool(pool_ref: &DbPool) -> Result<String, AppError> {
    sync_restore_snapshot_impl(pool_ref).await
}

async fn sync_restore_snapshot_impl(pool_ref: &DbPool) -> Result<String, AppError> {
    log::info!("Restoring from snapshot...");

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

    // List snapshots with metadata and choose latest explicitly.
    let snapshots = s3_client
        .list_with_metadata("snapshots/")
        .await
        .map_err(|e| {
            log::error!("S3 list error: {:?}", e);
            map_s3_error("list", e)
        })?;

    if snapshots.is_empty() {
        return Err(AppError::Db("No snapshots found".to_string()));
    }

    let latest = select_latest_snapshot(&snapshots)
        .ok_or_else(|| AppError::Db("No valid snapshots found".to_string()))?;
    let latest_key = latest.key.as_str();
    log::info!(
        "Downloading latest snapshot: {} (last_modified_unix={:?})",
        latest_key,
        latest.last_modified_unix
    );

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

#[derive(Debug, Clone)]
struct RemoteDeltaObject {
    key: String,
    source_device_id: String,
    timestamp: i64,
}

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

fn get_optional_config_value(conn: &Connection, key: &str) -> Result<Option<String>, AppError> {
    match conn.query_row(
        "SELECT value FROM sync_config WHERE key = ?1",
        [key],
        |row: &rusqlite::Row<'_>| row.get(0),
    ) {
        Ok(value) => Ok(Some(value)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(AppError::Db(e.to_string())),
    }
}

fn parse_remote_delta_object(key: &str) -> Option<RemoteDeltaObject> {
    let rest = key.strip_prefix("deltas/")?;
    let (source_device_id, file_name) = rest.split_once('/')?;
    let core = file_name.strip_prefix("delta-")?.strip_suffix(".gz")?;
    // Backward compatible:
    // - old: delta-<unix_ts>.gz
    // - new: delta-<unix_ts>-<uuid>.gz
    let ts_str = core.split('-').next()?;
    let timestamp = ts_str.parse::<i64>().ok()?;

    Some(RemoteDeltaObject {
        key: key.to_string(),
        source_device_id: source_device_id.to_string(),
        timestamp,
    })
}

fn select_latest_snapshot(snapshots: &[S3ObjectSummary]) -> Option<&S3ObjectSummary> {
    snapshots.iter().max_by(|a, b| {
        a.last_modified_unix
            .unwrap_or(i64::MIN)
            .cmp(&b.last_modified_unix.unwrap_or(i64::MIN))
            .then(a.key.cmp(&b.key))
    })
}

fn remote_delta_cursor_key(source_device_id: &str) -> String {
    format!("last_remote_delta_ts::{}", source_device_id)
}

fn get_remote_delta_cursor_timestamp(
    conn: &Connection,
    source_device_id: &str,
) -> Result<Option<i64>, AppError> {
    let key = remote_delta_cursor_key(source_device_id);
    let value = get_optional_config_value(conn, &key)?;
    Ok(value.and_then(|v| v.trim().parse::<i64>().ok()))
}

fn set_remote_delta_cursor_timestamp(
    conn: &Connection,
    source_device_id: &str,
    timestamp: i64,
) -> Result<(), AppError> {
    let key = remote_delta_cursor_key(source_device_id);
    set_config_value(conn, &key, &timestamp.to_string())
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

/// Export sync configuration (credentials included, device-specific state excluded).
/// The exported JSON can be imported on another device to quickly set up sync.
///
/// 导出内容：bucket / endpoint / access_key / secret_key / auto_sync_interval_minutes
/// 不导出：device_id / sync_enabled / last_sync / local_version（这些是设备运行时状态）
#[tauri::command]
pub fn cmd_sync_export_config(pool: State<DbPool>) -> Result<String, AppError> {
    let conn = pool
        .inner()
        .0
        .lock()
        .map_err(|e: std::sync::PoisonError<_>| AppError::Db(e.to_string()))?;

    let bucket = get_optional_config_value(&conn, "s3_bucket")?;
    let endpoint = get_optional_config_value(&conn, "s3_endpoint")?;
    let access_key = get_optional_config_value(&conn, "s3_access_key")?;
    let secret_key = get_optional_config_value(&conn, "s3_secret_key")?;
    let auto_sync_interval_minutes = get_config_value(&conn, "auto_sync_interval_minutes")
        .ok()
        .and_then(|v| v.trim().parse::<i64>().ok())
        .filter(|v| *v >= 1)
        .unwrap_or(1);

    let exported_at = chrono::Utc::now().to_rfc3339();

    let payload = serde_json::json!({
        "version": 1,
        "exported_at": exported_at,
        "sync_config": {
            "bucket": bucket.unwrap_or_default(),
            "endpoint": endpoint.unwrap_or_default(),
            "access_key": access_key.unwrap_or_default(),
            "secret_key": secret_key.unwrap_or_default(),
            "auto_sync_interval_minutes": auto_sync_interval_minutes,
        }
    });

    serde_json::to_string_pretty(&payload)
        .map_err(|e| AppError::Validation(format!("Failed to serialize config: {}", e)))
}

#[derive(Debug, Deserialize)]
pub struct SyncImportConfigReq {
    pub json: String,
}

/// Import sync configuration from a previously exported JSON file.
/// Only overwrites non-empty values; sync_enabled and device_id are never touched.
///
/// 导入逻辑：
/// - 仅覆盖非空字段（空字符串不覆盖已有值）
/// - 不修改 sync_enabled / device_id / last_sync / local_version
/// - 导入后不自动启用同步，由用户手动开启
#[tauri::command]
pub async fn cmd_sync_import_config(
    pool: State<'_, DbPool>,
    runtime: State<'_, SyncRuntime>,
    req: SyncImportConfigReq,
) -> Result<SyncConfigResp, AppError> {
    let parsed: serde_json::Value = serde_json::from_str(&req.json)
        .map_err(|e| AppError::Validation(format!("INVALID_JSON: {}", e)))?;

    let version = parsed.get("version").and_then(|v| v.as_i64()).unwrap_or(0);
    if version != 1 {
        return Err(AppError::Validation(format!(
            "UNSUPPORTED_VERSION: expected version 1, got {}",
            version
        )));
    }

    let cfg = parsed
        .get("sync_config")
        .ok_or_else(|| AppError::Validation("MISSING_FIELD: sync_config".to_string()))?;

    {
        let conn = pool
            .inner()
            .0
            .lock()
            .map_err(|e: std::sync::PoisonError<_>| AppError::Db(e.to_string()))?;

        if let Some(bucket) = cfg
            .get("bucket")
            .and_then(|v| v.as_str())
            .filter(|v| !v.trim().is_empty())
        {
            set_config_value(&conn, "s3_bucket", bucket.trim())?;
        }
        if let Some(endpoint) = cfg
            .get("endpoint")
            .and_then(|v| v.as_str())
            .filter(|v| !v.trim().is_empty())
        {
            #[cfg(target_os = "android")]
            validate_endpoint_https(&Some(endpoint.trim().to_string()))?;
            set_config_value(&conn, "s3_endpoint", endpoint.trim())?;
        }
        if let Some(access_key) = cfg
            .get("access_key")
            .and_then(|v| v.as_str())
            .filter(|v| !v.trim().is_empty())
        {
            set_config_value(&conn, "s3_access_key", access_key.trim())?;
        }
        if let Some(secret_key) = cfg
            .get("secret_key")
            .and_then(|v| v.as_str())
            .filter(|v| !v.trim().is_empty())
        {
            set_config_value(&conn, "s3_secret_key", secret_key.trim())?;
        }
        if let Some(interval) = cfg
            .get("auto_sync_interval_minutes")
            .and_then(|v| v.as_i64())
            .filter(|v| *v >= 1)
        {
            set_config_value(&conn, "auto_sync_interval_minutes", &interval.to_string())?;
        }
    }

    // Refresh scheduler in case interval changed (sync_enabled state unchanged).
    runtime.refresh_scheduler(pool.inner().clone()).await;

    // Return updated config so the frontend can refresh its state.
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

#[cfg(test)]
mod tests {
    use super::{parse_remote_delta_object, select_latest_snapshot};
    use crate::sync::S3ObjectSummary;

    #[test]
    fn parse_remote_delta_object_supports_legacy_key() {
        let key = "deltas/device-a/delta-1700000000.gz";
        let parsed = parse_remote_delta_object(key).expect("should parse legacy key");
        assert_eq!(parsed.source_device_id, "device-a");
        assert_eq!(parsed.timestamp, 1_700_000_000);
        assert_eq!(parsed.key, key);
    }

    #[test]
    fn parse_remote_delta_object_supports_new_key_format() {
        let key =
            "deltas/device-a/delta-1700000000123456789-550e8400-e29b-41d4-a716-446655440000.gz";
        let parsed = parse_remote_delta_object(key).expect("should parse new key format");
        assert_eq!(parsed.source_device_id, "device-a");
        assert_eq!(parsed.timestamp, 1_700_000_000_123_456_789);
        assert_eq!(parsed.key, key);
    }

    #[test]
    fn parse_remote_delta_object_rejects_invalid_key() {
        assert!(parse_remote_delta_object("deltas/device-a/not-a-delta.gz").is_none());
        assert!(parse_remote_delta_object("delta-1700000000.gz").is_none());
    }

    #[test]
    fn select_latest_snapshot_picks_highest_last_modified_then_key() {
        let snapshots = vec![
            S3ObjectSummary {
                key: "snapshots/latest-b.gz".to_string(),
                last_modified_unix: Some(100),
            },
            S3ObjectSummary {
                key: "snapshots/latest-a.gz".to_string(),
                last_modified_unix: Some(100),
            },
            S3ObjectSummary {
                key: "snapshots/latest-c.gz".to_string(),
                last_modified_unix: Some(101),
            },
        ];

        let latest = select_latest_snapshot(&snapshots).expect("should pick one snapshot");
        assert_eq!(latest.key, "snapshots/latest-c.gz");
        assert_eq!(latest.last_modified_unix, Some(101));
    }
}

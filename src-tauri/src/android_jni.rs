//! Android JNI bridge for background sync triggered by WorkManager.
//!
//! This module is only compiled for the Android target. It exposes a single
//! JNI entry point that Kotlin's SyncWorker calls to execute one full sync cycle.
//!
//! Calling convention:
//!   Java_com_nickdu_projex_SyncWorker_nativeRunSyncOnce(
//!       env: JNIEnv, _class: JClass,
//!       access_key: JString, secret_key: JString
//!   ) -> jstring  (JSON: {"status":"ok|skipped|failed","message":"..."})

#![cfg(target_os = "android")]

use jni::objects::JClass;
use jni::sys::jstring;
use jni::JNIEnv;

use crate::commands::sync::android_run_sync_once;
use crate::infra::DbPool;

use std::sync::Mutex;

/// Lazily initialised database pool shared between the Tauri runtime and the
/// background Worker. Initialised the first time either path opens the DB.
static ANDROID_POOL: std::sync::OnceLock<Mutex<Option<DbPool>>> = std::sync::OnceLock::new();

fn get_or_init_pool() -> Option<DbPool> {
    let guard = ANDROID_POOL.get_or_init(|| Mutex::new(None)).lock().ok()?;

    if let Some(ref pool) = *guard {
        return Some(pool.clone());
    }

    // Pool not yet initialised by the Tauri app (e.g. Worker woke up before
    // the UI). Open the database directly.
    drop(guard);
    init_pool_for_android()
}

fn init_pool_for_android() -> Option<DbPool> {
    use crate::infra::init_db;

    let base = dirs::data_dir()?;
    let data_dir = base.join("com.nickdu.projex").join("default");
    std::fs::create_dir_all(&data_dir).ok()?;
    let db_path = data_dir.join("projex.db");

    let pool = init_db(&db_path).ok()?;
    let mut guard = ANDROID_POOL.get_or_init(|| Mutex::new(None)).lock().ok()?;
    *guard = Some(pool.clone());
    Some(pool)
}

/// Register the pool created by the Tauri runtime so the background Worker
/// reuses the same connection pool and migration state.
pub fn register_pool(pool: DbPool) {
    let cell = ANDROID_POOL.get_or_init(|| Mutex::new(None));
    if let Ok(mut guard) = cell.lock() {
        *guard = Some(pool);
    }
}

/// JNI entry point called from `SyncWorker.kt`.
///
/// Credentials are read from SQLite sync_config (same as desktop).
/// Returns a JSON string:
///   {"status":"ok","message":"Sync completed"}
///   {"status":"skipped","message":"sync disabled"}
///   {"status":"failed","message":"..."}
#[no_mangle]
pub extern "C" fn Java_com_nickdu_projex_SyncWorker_nativeRunSyncOnce(
    env: JNIEnv,
    _class: JClass,
) -> jstring {
    let result = match get_or_init_pool() {
        Some(pool) => {
            // Run the async sync function on a new Tokio runtime.
            // WorkManager runs the worker on its own thread pool; we spin up a
            // single-threaded runtime here to avoid depending on the Tauri
            // runtime being alive.
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build();

            match rt {
                Ok(rt) => rt.block_on(android_run_sync_once(&pool)),
                Err(e) => crate::commands::sync::AndroidSyncResult {
                    status: "failed".to_string(),
                    message: format!("tokio runtime error: {}", e),
                },
            }
        }
        None => crate::commands::sync::AndroidSyncResult {
            status: "failed".to_string(),
            message: "db pool unavailable".to_string(),
        },
    };

    let json = format!(
        r#"{{"status":"{}","message":"{}"}}"#,
        result.status,
        result.message.replace('"', "\\\"")
    );

    env.new_string(&json)
        .map(|s| s.into_raw())
        .unwrap_or(std::ptr::null_mut())
}

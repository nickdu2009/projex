package com.nickdu.projex

import android.content.Context
import android.util.Log
import androidx.work.CoroutineWorker
import androidx.work.WorkerParameters
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext

/**
 * WorkManager periodic worker that triggers one full S3 sync cycle.
 *
 * Design:
 * - Credentials are read from SQLite sync_config (same as desktop).
 * - The actual sync logic runs in Rust via JNI (nativeRunSyncOnce).
 * - If credentials are absent or sync is disabled, the worker returns SUCCESS
 *   (no retry) and waits for the next scheduled period.
 * - Concurrency (Worker vs. foreground app) is handled inside Rust via a file lock.
 */
class SyncWorker(appContext: Context, params: WorkerParameters) :
    CoroutineWorker(appContext, params) {

    companion object {
        private const val TAG = "ProjexSyncWorker"

        // Called from Rust JNI — do NOT rename without updating android_jni.rs
        @JvmStatic
        external fun nativeRunSyncOnce(): String
    }

    override suspend fun doWork(): Result = withContext(Dispatchers.IO) {
        Log.i(TAG, "SyncWorker started")

        return@withContext try {
            val json = nativeRunSyncOnce()
            Log.i(TAG, "nativeRunSyncOnce result: $json")

            // Parse minimal JSON: {"status":"ok|skipped|failed","message":"..."}
            val status = Regex(""""status"\s*:\s*"([^"]+)"""")
                .find(json)?.groupValues?.getOrNull(1) ?: "failed"

            if (status == "failed") {
                // Don't retry immediately — wait for next scheduled period.
                // Errors are already written to sync_config.last_sync_error by Rust.
                Log.w(TAG, "Sync failed: $json")
                Result.success()
            } else {
                Log.i(TAG, "Sync $status")
                Result.success()
            }
        } catch (e: Exception) {
            Log.e(TAG, "SyncWorker exception: ${e.message}", e)
            Result.success() // Don't let WorkManager retry aggressively
        }
    }
}

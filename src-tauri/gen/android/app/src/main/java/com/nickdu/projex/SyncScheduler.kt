package com.nickdu.projex

import android.content.Context
import android.util.Log
import androidx.work.Constraints
import androidx.work.ExistingPeriodicWorkPolicy
import androidx.work.NetworkType
import androidx.work.PeriodicWorkRequestBuilder
import androidx.work.WorkManager
import java.util.concurrent.TimeUnit

/**
 * Manages the WorkManager periodic sync schedule for Projex.
 *
 * Unique work name ensures only one PeriodicWork is enqueued at a time.
 * Calling [schedule] is idempotent: UPDATE policy replaces any stale config.
 */
object SyncScheduler {

    private const val TAG = "ProjexSyncScheduler"

    /** Unique WorkManager task name (per default profile). */
    const val WORK_NAME = "projex.sync.periodic::default"

    /** Minimum allowed period (WorkManager platform floor is 15 min). */
    private const val PERIOD_MINUTES = 15L

    /**
     * Enqueue (or update) the periodic sync work.
     * Safe to call multiple times — UPDATE policy replaces the existing config.
     */
    fun schedule(context: Context) {
        val constraints = Constraints.Builder()
            .setRequiredNetworkType(NetworkType.CONNECTED) // allow metered (cellular)
            .build()

        val request = PeriodicWorkRequestBuilder<SyncWorker>(PERIOD_MINUTES, TimeUnit.MINUTES)
            .setConstraints(constraints)
            .build()

        WorkManager.getInstance(context).enqueueUniquePeriodicWork(
            WORK_NAME,
            ExistingPeriodicWorkPolicy.UPDATE,
            request,
        )

        Log.i(TAG, "Periodic sync scheduled (period=${PERIOD_MINUTES}min)")
    }

    /**
     * Cancel the periodic sync work.
     * Call this when the user disables sync in Settings.
     */
    fun cancel(context: Context) {
        WorkManager.getInstance(context).cancelUniqueWork(WORK_NAME)
        Log.i(TAG, "Periodic sync cancelled")
    }
}

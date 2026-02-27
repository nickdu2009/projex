package com.nickdu.projex

import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.util.Log

/**
 * Listens for BOOT_COMPLETED and MY_PACKAGE_REPLACED broadcasts and
 * re-schedules the periodic sync work.
 *
 * WorkManager typically survives reboots automatically, but some OEM ROMs
 * kill scheduled work on reboot. Explicitly re-scheduling here ensures
 * the periodic sync is restored even on those devices.
 *
 * The schedule call is idempotent (ExistingPeriodicWorkPolicy.UPDATE),
 * so calling it redundantly on normal WorkManager-aware devices is harmless.
 */
class BootReceiver : BroadcastReceiver() {

    override fun onReceive(context: Context, intent: Intent) {
        val action = intent.action
        if (action != Intent.ACTION_BOOT_COMPLETED &&
            action != Intent.ACTION_MY_PACKAGE_REPLACED
        ) return

        Log.i("ProjexBootReceiver", "Received $action — re-scheduling sync")

        // Always re-schedule. Worker will skip if sync disabled or credentials not configured.
        SyncScheduler.schedule(context)
    }
}

package com.nickdu.projex

import android.os.Bundle
import androidx.activity.enableEdgeToEdge

class MainActivity : TauriActivity() {
  override fun onCreate(savedInstanceState: Bundle?) {
    enableEdgeToEdge()
    super.onCreate(savedInstanceState)

    // Ensure periodic sync is scheduled whenever the app starts (idempotent).
    // Worker will skip if sync disabled or credentials not configured.
    SyncScheduler.schedule(applicationContext)
  }
}

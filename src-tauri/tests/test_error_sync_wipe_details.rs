//! Contract tests for `AppError::SyncWipeConfirmRequired` serialization.
//!
//! Ensures that the `details` field in `AppErrorDto` contains camelCase keys
//! that the frontend depends on for displaying the pending wipe dialog.

use app_lib::error::{AppError, PendingWipeInfo};

fn make_pending() -> PendingWipeInfo {
    PendingWipeInfo {
        wipe_id: "wipe-test-123".to_string(),
        source_device_id: "device-remote-xyz".to_string(),
        delta_key: "deltas/device-remote-xyz/delta-9999.gz".to_string(),
        source_timestamp: 1_700_000_000,
        created_at: "2026-03-03T12:00:00Z".to_string(),
    }
}

// ── code ──────────────────────────────────────────────────────────────────────

#[test]
fn sync_wipe_confirm_required_has_correct_code() {
    let err = AppError::SyncWipeConfirmRequired(make_pending());
    assert_eq!(err.code(), "SYNC_WIPE_CONFIRM_REQUIRED");
}

// ── to_serde / details ────────────────────────────────────────────────────────

#[test]
fn sync_wipe_confirm_required_details_contains_all_fields() {
    let pending = make_pending();
    let err = AppError::SyncWipeConfirmRequired(pending.clone());
    let dto = err.to_serde();

    assert_eq!(dto.code, "SYNC_WIPE_CONFIRM_REQUIRED");
    assert!(!dto.message.is_empty());

    let details = dto
        .details
        .expect("details must be present for SyncWipeConfirmRequired");

    // Frontend depends on these camelCase keys.
    assert_eq!(details["wipeId"], pending.wipe_id, "wipeId must match");
    assert_eq!(
        details["sourceDeviceId"], pending.source_device_id,
        "sourceDeviceId must match"
    );
    assert_eq!(
        details["deltaKey"], pending.delta_key,
        "deltaKey must match"
    );
    assert_eq!(
        details["sourceTimestamp"], pending.source_timestamp,
        "sourceTimestamp must match"
    );
    assert_eq!(
        details["createdAt"], pending.created_at,
        "createdAt must match"
    );
}

// ── Serialize impl (used by Tauri IPC) ───────────────────────────────────────

#[test]
fn app_error_serialize_produces_dto_shape() {
    let err = AppError::SyncWipeConfirmRequired(make_pending());
    let json = serde_json::to_value(&err).unwrap();

    assert_eq!(json["code"], "SYNC_WIPE_CONFIRM_REQUIRED");
    assert!(json["message"].is_string());
    let details = &json["details"];
    assert!(details.is_object(), "details must be an object");
    assert!(details["wipeId"].is_string());
    assert!(details["sourceDeviceId"].is_string());
    assert!(details["deltaKey"].is_string());
    assert!(details["sourceTimestamp"].is_number());
    assert!(details["createdAt"].is_string());
}

// ── other error variants must NOT have details ────────────────────────────────

#[test]
fn non_wipe_errors_have_no_details() {
    let cases: Vec<AppError> = vec![
        AppError::Db("db error".to_string()),
        AppError::Validation("invalid".to_string()),
        AppError::NotFound("not found".to_string()),
        AppError::Sync("sync error".to_string()),
        AppError::PartnerImmutable,
    ];
    for err in cases {
        let dto = err.to_serde();
        assert!(
            dto.details.is_none(),
            "error {:?} should have no details, but got: {:?}",
            dto.code,
            dto.details
        );
    }
}

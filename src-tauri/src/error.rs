//! Stable error codes for frontend.

use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("{0}")]
    Db(String),

    #[error("Validation failed: {0}")]
    Validation(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Conflict: {0}")]
    Conflict(String),

    #[error("Partner is immutable after project creation")]
    PartnerImmutable,

    #[error("Invalid status transition: {0}")]
    InvalidStatusTransition(String),

    #[error("Note is required for this transition")]
    NoteRequired,

    #[error("Assignment already active for this person on this project")]
    AssignmentAlreadyActive,

    #[error("No active assignment to end")]
    AssignmentNotActive,

    #[error("Sync config incomplete")]
    SyncConfigIncomplete,

    #[error("Access denied: bucket does not belong to current credentials")]
    SyncBucketNotOwned,

    #[error("Sync error: {0}")]
    Sync(String),

    #[error("Log file error: {0}")]
    LogFile(String),

    #[error("Log I/O error: {0}")]
    LogIo(String),
}

impl AppError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Db(_) => "DB_ERROR",
            Self::Validation(_) => "VALIDATION_ERROR",
            Self::NotFound(_) => "NOT_FOUND",
            Self::Conflict(_) => "CONFLICT",
            Self::PartnerImmutable => "PARTNER_IMMUTABLE",
            Self::InvalidStatusTransition(_) => "INVALID_STATUS_TRANSITION",
            Self::NoteRequired => "NOTE_REQUIRED",
            Self::AssignmentAlreadyActive => "ASSIGNMENT_ALREADY_ACTIVE",
            Self::AssignmentNotActive => "ASSIGNMENT_NOT_ACTIVE",
            Self::SyncConfigIncomplete => "SYNC_CONFIG_INCOMPLETE",
            Self::SyncBucketNotOwned => "SYNC_BUCKET_NOT_OWNED",
            Self::Sync(_) => "SYNC_ERROR",
            Self::LogFile(_) => "LOG_INVALID_FILE",
            Self::LogIo(_) => "LOG_IO_ERROR",
        }
    }

    pub fn to_serde(&self) -> AppErrorDto {
        AppErrorDto {
            code: self.code().to_string(),
            message: self.to_string(),
            details: None,
        }
    }
}

impl From<rusqlite::Error> for AppError {
    fn from(e: rusqlite::Error) -> Self {
        AppError::Db(e.to_string())
    }
}

impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        AppError::LogIo(e.to_string())
    }
}

impl serde::Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.to_serde().serialize(serializer)
    }
}

#[derive(Debug, Serialize)]
pub struct AppErrorDto {
    pub code: String,
    pub message: String,
    pub details: Option<serde_json::Value>,
}

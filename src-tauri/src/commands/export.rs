//! Export command handlers.

use crate::app::export_json_string;
use crate::error::AppError;
use crate::infra::DbPool;
use serde::Deserialize;
use tauri::State;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportJsonReq {
    pub schema_version: Option<i32>,
}

#[tauri::command]
pub fn cmd_export_json(
    pool: State<DbPool>,
    req: Option<ExportJsonReq>,
) -> Result<String, AppError> {
    let schema_version = req.and_then(|r| r.schema_version);
    export_json_string(&pool, schema_version)
}

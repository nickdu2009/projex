//! Export / Import command handlers.

use crate::app::{
    export_json_string, export_persons_csv, import_json_string, import_persons_csv, ImportResult,
    PersonImportResult,
};
use crate::error::AppError;
use crate::infra::DbPool;
use serde::Deserialize;
use tauri::State;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportJsonReq {
    pub schema_version: Option<i32>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportJsonReq {
    pub json: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportPersonsCsvReq {
    pub csv: String,
}

#[tauri::command]
pub fn cmd_export_json(
    pool: State<DbPool>,
    req: Option<ExportJsonReq>,
) -> Result<String, AppError> {
    let schema_version = req.and_then(|r| r.schema_version);
    export_json_string(&pool, schema_version)
}

#[tauri::command]
pub fn cmd_import_json(pool: State<DbPool>, req: ImportJsonReq) -> Result<ImportResult, AppError> {
    import_json_string(&pool, &req.json)
}

#[tauri::command]
pub fn cmd_export_persons_csv(pool: State<DbPool>) -> Result<String, AppError> {
    export_persons_csv(&pool)
}

#[tauri::command]
pub fn cmd_import_persons_csv(
    pool: State<DbPool>,
    req: ImportPersonsCsvReq,
) -> Result<PersonImportResult, AppError> {
    import_persons_csv(&pool, &req.csv)
}

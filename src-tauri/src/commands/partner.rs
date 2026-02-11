use crate::app::{
    partner_create, partner_deactivate, partner_get, partner_list, partner_projects,
    partner_update, PartnerCreateReq, PartnerDto, PartnerProjectItemDto, PartnerUpdateReq,
};
use crate::error::AppError;
use crate::infra::DbPool;
use serde::Deserialize;
use tauri::State;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PartnerListReq {
    pub only_active: Option<bool>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PartnerGetReq {
    pub id: String,
}

#[tauri::command]
pub fn cmd_partner_create(
    pool: State<DbPool>,
    req: PartnerCreateReq,
) -> Result<PartnerDto, AppError> {
    partner_create(&pool, req)
}

#[tauri::command]
pub fn cmd_partner_get(pool: State<DbPool>, req: PartnerGetReq) -> Result<PartnerDto, AppError> {
    partner_get(&pool, &req.id)
}

#[tauri::command]
pub fn cmd_partner_update(
    pool: State<DbPool>,
    req: PartnerUpdateReq,
) -> Result<PartnerDto, AppError> {
    partner_update(&pool, req)
}

#[tauri::command]
pub fn cmd_partner_deactivate(
    pool: State<DbPool>,
    req: PartnerGetReq,
) -> Result<PartnerDto, AppError> {
    partner_deactivate(&pool, &req.id)
}

#[tauri::command]
pub fn cmd_partner_list(
    pool: State<DbPool>,
    req: Option<PartnerListReq>,
) -> Result<Vec<PartnerDto>, AppError> {
    partner_list(&pool, req.and_then(|r| r.only_active).unwrap_or(true))
}

#[tauri::command]
pub fn cmd_partner_projects(
    pool: State<DbPool>,
    req: PartnerGetReq,
) -> Result<Vec<PartnerProjectItemDto>, AppError> {
    partner_projects(&pool, &req.id)
}

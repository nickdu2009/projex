use crate::app::{
    person_all_projects, person_create, person_current_projects, person_deactivate, person_get,
    person_list, person_update, PersonCreateReq, PersonDto, PersonProjectItemDto, PersonUpdateReq,
};
use crate::error::AppError;
use crate::infra::DbPool;
use serde::Deserialize;
use tauri::State;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PersonListReq {
    pub only_active: Option<bool>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PersonGetReq {
    pub id: String,
}

#[tauri::command]
pub fn cmd_person_create(pool: State<DbPool>, req: PersonCreateReq) -> Result<PersonDto, AppError> {
    person_create(&pool, req)
}

#[tauri::command]
pub fn cmd_person_get(pool: State<DbPool>, req: PersonGetReq) -> Result<PersonDto, AppError> {
    person_get(&pool, &req.id)
}

#[tauri::command]
pub fn cmd_person_update(pool: State<DbPool>, req: PersonUpdateReq) -> Result<PersonDto, AppError> {
    person_update(&pool, req)
}

#[tauri::command]
pub fn cmd_person_deactivate(
    pool: State<DbPool>,
    req: PersonGetReq,
) -> Result<PersonDto, AppError> {
    person_deactivate(&pool, &req.id)
}

#[tauri::command]
pub fn cmd_person_list(
    pool: State<DbPool>,
    req: Option<PersonListReq>,
) -> Result<Vec<PersonDto>, AppError> {
    person_list(&pool, req.and_then(|r| r.only_active).unwrap_or(true))
}

#[tauri::command]
pub fn cmd_person_current_projects(
    pool: State<DbPool>,
    req: PersonGetReq,
) -> Result<Vec<PersonProjectItemDto>, AppError> {
    person_current_projects(&pool, &req.id)
}

#[tauri::command]
pub fn cmd_person_all_projects(
    pool: State<DbPool>,
    req: PersonGetReq,
) -> Result<Vec<PersonProjectItemDto>, AppError> {
    person_all_projects(&pool, &req.id)
}

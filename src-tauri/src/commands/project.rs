use crate::app::{
    project_change_status, project_create, project_get, project_list, project_update,
    ProjectChangeStatusReq, ProjectCreateReq, ProjectDetailDto, ProjectListPage, ProjectListReq,
    ProjectUpdateReq,
};
use crate::error::AppError;
use crate::infra::DbPool;
use serde::Deserialize;
use tauri::State;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectGetReq {
    pub id: String,
}

#[tauri::command]
pub fn cmd_project_create(pool: State<DbPool>, req: ProjectCreateReq) -> Result<ProjectDetailDto, AppError> {
    project_create(&pool, req)
}

#[tauri::command]
pub fn cmd_project_get(pool: State<DbPool>, req: ProjectGetReq) -> Result<ProjectDetailDto, AppError> {
    project_get(&pool, &req.id)
}

#[tauri::command]
pub fn cmd_project_update(pool: State<DbPool>, req: ProjectUpdateReq) -> Result<ProjectDetailDto, AppError> {
    project_update(&pool, req)
}

#[tauri::command]
pub fn cmd_project_list(pool: State<DbPool>, req: Option<ProjectListReq>) -> Result<ProjectListPage, AppError> {
    project_list(&pool, req.unwrap_or_default())
}

#[tauri::command]
pub fn cmd_project_change_status(
    pool: State<DbPool>,
    req: ProjectChangeStatusReq,
) -> Result<ProjectDetailDto, AppError> {
    project_change_status(&pool, req)
}

use crate::app::{
    comment_create, comment_delete, comment_list_by_project, comment_update, CommentCreateReq,
    CommentDto, CommentUpdateReq,
};
use crate::error::AppError;
use crate::infra::DbPool;
use serde::Deserialize;
use tauri::State;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommentListReq {
    pub project_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommentDeleteReq {
    pub id: String,
}

#[tauri::command]
pub fn cmd_comment_create(
    pool: State<DbPool>,
    req: CommentCreateReq,
) -> Result<CommentDto, AppError> {
    comment_create(&pool, req)
}

#[tauri::command]
pub fn cmd_comment_update(
    pool: State<DbPool>,
    req: CommentUpdateReq,
) -> Result<CommentDto, AppError> {
    comment_update(&pool, req)
}

#[tauri::command]
pub fn cmd_comment_delete(pool: State<DbPool>, req: CommentDeleteReq) -> Result<(), AppError> {
    comment_delete(&pool, req.id)
}

#[tauri::command]
pub fn cmd_comment_list(
    pool: State<DbPool>,
    req: CommentListReq,
) -> Result<Vec<CommentDto>, AppError> {
    comment_list_by_project(&pool, req.project_id)
}

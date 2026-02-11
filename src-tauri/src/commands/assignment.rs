use crate::app::{
    assignment_add_member, assignment_end_member, assignment_list_by_project, AssignmentAddReq,
    AssignmentEndReq, AssignmentItemDto,
};
use crate::error::AppError;
use crate::infra::DbPool;
use serde::Deserialize;
use tauri::State;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssignmentListReq {
    pub project_id: String,
}

#[tauri::command]
pub fn cmd_assignment_add_member(
    pool: State<DbPool>,
    req: AssignmentAddReq,
) -> Result<(), AppError> {
    assignment_add_member(&pool, req)
}

#[tauri::command]
pub fn cmd_assignment_end_member(
    pool: State<DbPool>,
    req: AssignmentEndReq,
) -> Result<(), AppError> {
    assignment_end_member(&pool, req)
}

#[tauri::command]
pub fn cmd_assignment_list_by_project(
    pool: State<DbPool>,
    req: AssignmentListReq,
) -> Result<Vec<AssignmentItemDto>, AppError> {
    assignment_list_by_project(&pool, &req.project_id)
}

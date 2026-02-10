use crate::app::{
    assignment_add_member, assignment_end_member, AssignmentAddReq, AssignmentEndReq,
};
use crate::error::AppError;
use crate::infra::DbPool;
use tauri::State;

#[tauri::command]
pub fn cmd_assignment_add_member(pool: State<DbPool>, req: AssignmentAddReq) -> Result<(), AppError> {
    assignment_add_member(&pool, req)
}

#[tauri::command]
pub fn cmd_assignment_end_member(pool: State<DbPool>, req: AssignmentEndReq) -> Result<(), AppError> {
    assignment_end_member(&pool, req)
}

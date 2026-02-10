//! Assignment use cases: add member, end member, list by project.

use crate::error::AppError;
use crate::infra::get_connection;
use crate::infra::DbPool;
use chrono::Utc;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize)]
pub struct AssignmentItemDto {
    pub id: String,
    pub project_id: String,
    pub person_id: String,
    pub person_name: String,
    pub role: String,
    pub start_at: String,
    pub end_at: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssignmentAddReq {
    pub project_id: String,
    pub person_id: String,
    pub role: Option<String>,
    pub start_at: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssignmentEndReq {
    pub project_id: String,
    pub person_id: String,
    pub end_at: Option<String>,
}

pub fn assignment_add_member(pool: &DbPool, req: AssignmentAddReq) -> Result<(), AppError> {
    let role = req.role.as_deref().unwrap_or("member").to_string();
    let now = Utc::now().to_rfc3339();
    let start_at = req
        .start_at
        .as_deref()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or(&now)
        .to_string();

    let conn = get_connection(pool);
    let has_active: i32 = conn
        .query_row(
            "SELECT COUNT(1) FROM assignments WHERE project_id = ?1 AND person_id = ?2 AND end_at IS NULL",
            params![&req.project_id, &req.person_id],
            |r| r.get(0),
        )
        .map_err(|e| AppError::Db(e.to_string()))?;
    if has_active > 0 {
        return Err(AppError::AssignmentAlreadyActive);
    }

    let id = Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO assignments (id, project_id, person_id, role, start_at, end_at, created_at) VALUES (?1, ?2, ?3, ?4, ?5, NULL, ?5)",
        params![id, &req.project_id, &req.person_id, role, &start_at],
    )
    .map_err(|e| AppError::Db(e.to_string()))?;
    Ok(())
}

pub fn assignment_end_member(pool: &DbPool, req: AssignmentEndReq) -> Result<(), AppError> {
    let now = Utc::now().to_rfc3339();
    let end_at = req
        .end_at
        .as_deref()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or(&now);

    let conn = get_connection(pool);
    let changed = conn
        .execute(
            "UPDATE assignments SET end_at = ?1 WHERE project_id = ?2 AND person_id = ?3 AND end_at IS NULL",
            params![end_at, &req.project_id, &req.person_id],
        )
        .map_err(|e| AppError::Db(e.to_string()))?;
    if changed == 0 {
        return Err(AppError::AssignmentNotActive);
    }
    Ok(())
}

pub fn assignment_list_by_project(
    pool: &DbPool,
    project_id: &str,
) -> Result<Vec<AssignmentItemDto>, AppError> {
    let conn = get_connection(pool);
    let mut stmt = conn
        .prepare(
            "SELECT a.id, a.project_id, a.person_id, COALESCE(p.display_name, '?') AS person_name, \
             a.role, a.start_at, a.end_at, a.created_at \
             FROM assignments a \
             LEFT JOIN persons p ON p.id = a.person_id \
             WHERE a.project_id = ?1 \
             ORDER BY a.end_at IS NOT NULL, a.start_at DESC",
        )
        .map_err(|e| AppError::Db(e.to_string()))?;
    let rows = stmt
        .query_map([project_id], |r| {
            Ok(AssignmentItemDto {
                id: r.get(0)?,
                project_id: r.get(1)?,
                person_id: r.get(2)?,
                person_name: r.get(3)?,
                role: r.get(4)?,
                start_at: r.get(5)?,
                end_at: r.get(6)?,
                created_at: r.get(7)?,
            })
        })
        .map_err(|e| AppError::Db(e.to_string()))?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r.map_err(|e| AppError::Db(e.to_string()))?);
    }
    Ok(out)
}

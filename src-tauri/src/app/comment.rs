//! Comment use cases: create, update, delete, list by project.

use crate::error::AppError;
use crate::infra::{get_connection, DbPool};
use chrono::Utc;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommentDto {
    pub id: String,
    pub project_id: String,
    pub person_id: Option<String>,
    pub person_name: Option<String>,
    pub content: String,
    pub is_pinned: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommentCreateReq {
    pub project_id: String,
    pub person_id: Option<String>,
    pub content: String,
    pub is_pinned: Option<bool>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommentUpdateReq {
    pub id: String,
    pub content: Option<String>,
    pub person_id: Option<String>,
    pub is_pinned: Option<bool>,
}

/// Create a new comment
pub fn comment_create(pool: &DbPool, req: CommentCreateReq) -> Result<CommentDto, AppError> {
    // Validate: project exists
    let conn = get_connection(pool);
    let project_exists: bool = conn.query_row(
        "SELECT 1 FROM projects WHERE id = ?",
        params![&req.project_id],
        |_| Ok(true),
    ).unwrap_or(false);
    
    if !project_exists {
        return Err(AppError::NotFound("Project not found".into()));
    }

    // Validate: person exists if provided
    if let Some(ref person_id) = req.person_id {
        let person_exists: bool = conn.query_row(
            "SELECT 1 FROM persons WHERE id = ?",
            params![person_id],
            |_| Ok(true),
        ).unwrap_or(false);
        
        if !person_exists {
            return Err(AppError::NotFound("Person not found".into()));
        }
    }

    let now = Utc::now().to_rfc3339();
    let id = Uuid::new_v4().to_string();
    let is_pinned = req.is_pinned.unwrap_or(false);

    conn.execute(
        "INSERT INTO project_comments (id, project_id, person_id, content, is_pinned, created_at, updated_at, _version)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 1)",
        params![
            &id,
            &req.project_id,
            &req.person_id,
            &req.content,
            is_pinned as i32,
            &now,
            &now,
        ],
    )?;

    comment_get(&conn, &id)
}

/// Update an existing comment
pub fn comment_update(pool: &DbPool, req: CommentUpdateReq) -> Result<CommentDto, AppError> {
    let conn = get_connection(pool);
    
    // Check if comment exists
    let exists: bool = conn.query_row(
        "SELECT 1 FROM project_comments WHERE id = ?",
        params![&req.id],
        |_| Ok(true),
    ).unwrap_or(false);
    
    if !exists {
        return Err(AppError::NotFound("Comment not found".into()));
    }

    // Validate person_id if provided
    if let Some(ref person_id) = req.person_id {
        let person_exists: bool = conn.query_row(
            "SELECT 1 FROM persons WHERE id = ?",
            params![person_id],
            |_| Ok(true),
        ).unwrap_or(false);
        
        if !person_exists {
            return Err(AppError::NotFound("Person not found".into()));
        }
    }

    let now = Utc::now().to_rfc3339();
    
    // Fetch current values to determine what to update
    let (current_content, current_person_id, current_pinned): (String, Option<String>, i32) = conn.query_row(
        "SELECT content, person_id, is_pinned FROM project_comments WHERE id = ?",
        params![&req.id],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    )?;
    
    let final_content = req.content.unwrap_or(current_content);
    let final_person_id = if req.person_id.is_some() {
        req.person_id
    } else {
        current_person_id
    };
    let final_is_pinned = req.is_pinned.unwrap_or(current_pinned != 0);
    
    conn.execute(
        "UPDATE project_comments SET content = ?1, person_id = ?2, is_pinned = ?3, updated_at = ?4, _version = _version + 1 WHERE id = ?5",
        params![final_content, final_person_id, if final_is_pinned { 1 } else { 0 }, &now, &req.id],
    )?;
    
    comment_get(&conn, &req.id)
}

/// Delete a comment
pub fn comment_delete(pool: &DbPool, id: String) -> Result<(), AppError> {
    let conn = get_connection(pool);
    
    let rows = conn.execute(
        "DELETE FROM project_comments WHERE id = ?",
        params![&id],
    )?;
    
    if rows == 0 {
        return Err(AppError::NotFound("Comment not found".into()));
    }
    
    Ok(())
}

/// List all comments for a project (pinned first, then by created_at DESC)
pub fn comment_list_by_project(pool: &DbPool, project_id: String) -> Result<Vec<CommentDto>, AppError> {
    let conn = get_connection(pool);
    
    let mut stmt = conn.prepare(
        "SELECT c.id, c.project_id, c.person_id, c.content, c.is_pinned, c.created_at, c.updated_at,
                p.display_name as person_name
         FROM project_comments c
         LEFT JOIN persons p ON c.person_id = p.id
         WHERE c.project_id = ?
         ORDER BY c.is_pinned DESC, c.created_at DESC"
    ).map_err(|e| AppError::Db(e.to_string()))?;
    
    let rows = stmt.query_map(params![&project_id], |row| {
        Ok(CommentDto {
            id: row.get(0)?,
            project_id: row.get(1)?,
            person_id: row.get(2)?,
            content: row.get(3)?,
            is_pinned: row.get::<_, i32>(4)? != 0,
            created_at: row.get(5)?,
            updated_at: row.get(6)?,
            person_name: row.get(7)?,
        })
    })?;
    
    let mut comments = Vec::new();
    for comment in rows {
        comments.push(comment?);
    }
    
    Ok(comments)
}

/// Internal helper to get a single comment
fn comment_get(conn: &rusqlite::Connection, id: &str) -> Result<CommentDto, AppError> {
    let mut stmt = conn.prepare(
        "SELECT c.id, c.project_id, c.person_id, c.content, c.is_pinned, c.created_at, c.updated_at,
                p.display_name as person_name
         FROM project_comments c
         LEFT JOIN persons p ON c.person_id = p.id
         WHERE c.id = ?"
    ).map_err(|e| AppError::Db(e.to_string()))?;
    
    let comment = stmt.query_row(params![id], |row| {
        Ok(CommentDto {
            id: row.get(0)?,
            project_id: row.get(1)?,
            person_id: row.get(2)?,
            content: row.get(3)?,
            is_pinned: row.get::<_, i32>(4)? != 0,
            created_at: row.get(5)?,
            updated_at: row.get(6)?,
            person_name: row.get(7)?,
        })
    })?;
    
    Ok(comment)
}

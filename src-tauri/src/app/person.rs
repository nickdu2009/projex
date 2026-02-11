//! Person use cases.

use crate::error::AppError;
use crate::infra::get_connection;
use crate::infra::DbPool;
use chrono::Utc;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PersonCreateReq {
    pub display_name: String,
    pub email: Option<String>,
    pub role: Option<String>,
    pub note: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct PersonDto {
    pub id: String,
    pub display_name: String,
    pub email: String,
    pub role: String,
    pub note: String,
    pub is_active: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PersonUpdateReq {
    pub id: String,
    pub display_name: Option<String>,
    pub email: Option<String>,
    pub role: Option<String>,
    pub note: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct PersonProjectItemDto {
    pub id: String,
    pub name: String,
    pub current_status: String,
    pub updated_at: String,
    pub last_involved_at: Option<String>,
}

pub fn person_create(pool: &DbPool, req: PersonCreateReq) -> Result<PersonDto, AppError> {
    let display_name = req.display_name.trim();
    if display_name.is_empty() {
        return Err(AppError::Validation("display_name is required".into()));
    }
    let id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();
    let email = req.email.unwrap_or_default();
    let role = req.role.unwrap_or_default();
    let note = req.note.unwrap_or_default();

    let conn = get_connection(pool);
    conn.execute(
        "INSERT INTO persons (id, display_name, email, role, note, is_active, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, 1, ?6, ?6)",
        params![id, display_name, email, role, note, &now],
    )
    .map_err(|e| AppError::Db(e.to_string()))?;

    Ok(PersonDto {
        id: id.clone(),
        display_name: display_name.to_string(),
        email,
        role,
        note,
        is_active: true,
        created_at: now.clone(),
        updated_at: now,
    })
}

pub fn person_list(pool: &DbPool, only_active: bool) -> Result<Vec<PersonDto>, AppError> {
    let conn = get_connection(pool);
    let sql = if only_active {
        "SELECT id, display_name, email, role, note, is_active, created_at, updated_at FROM persons WHERE is_active = 1 ORDER BY display_name COLLATE NOCASE"
    } else {
        "SELECT id, display_name, email, role, note, is_active, created_at, updated_at FROM persons ORDER BY display_name COLLATE NOCASE"
    };
    let mut stmt = conn.prepare(sql).map_err(|e| AppError::Db(e.to_string()))?;
    let rows = stmt.query_map([], |row| {
        Ok(PersonDto {
            id: row.get(0)?,
            display_name: row.get(1)?,
            email: row.get(2)?,
            role: row.get(3)?,
            note: row.get(4)?,
            is_active: row.get::<_, i32>(5)? != 0,
            created_at: row.get(6)?,
            updated_at: row.get(7)?,
        })
    })?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r.map_err(|e| AppError::Db(e.to_string()))?);
    }
    Ok(out)
}

pub fn person_get(pool: &DbPool, id: &str) -> Result<PersonDto, AppError> {
    let conn = get_connection(pool);
    conn.query_row(
        "SELECT id, display_name, email, role, note, is_active, created_at, updated_at FROM persons WHERE id = ?1",
        [id],
        |row| {
            Ok(PersonDto {
                id: row.get(0)?,
                display_name: row.get(1)?,
                email: row.get(2)?,
                role: row.get(3)?,
                note: row.get(4)?,
                is_active: row.get::<_, i32>(5)? != 0,
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
            })
        },
    )
    .map_err(|e| AppError::NotFound(e.to_string()))
}

pub fn person_update(pool: &DbPool, req: PersonUpdateReq) -> Result<PersonDto, AppError> {
    let now = Utc::now().to_rfc3339();

    {
        let conn = get_connection(pool);

        let (display_name, email, role, note): (String, String, String, String) = conn
            .query_row(
                "SELECT display_name, email, role, note FROM persons WHERE id = ?1",
                [&req.id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            )
            .map_err(|_| AppError::NotFound(format!("person {}", req.id)))?;

        let display_name = req
            .display_name
            .as_deref()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or(display_name);
        let email = req.email.unwrap_or(email);
        let role = req.role.unwrap_or(role);
        let note = req.note.unwrap_or(note);

        if display_name.is_empty() {
            return Err(AppError::Validation("display_name is required".into()));
        }

        conn.execute(
            "UPDATE persons SET display_name = ?1, email = ?2, role = ?3, note = ?4, updated_at = ?5 WHERE id = ?6",
            params![&display_name, &email, &role, &note, &now, &req.id],
        )
        .map_err(|e| AppError::Db(e.to_string()))?;
    } // release conn before calling person_get to avoid deadlock

    person_get(pool, &req.id)
}

pub fn person_deactivate(pool: &DbPool, id: &str) -> Result<PersonDto, AppError> {
    let now = Utc::now().to_rfc3339();
    {
        let conn = get_connection(pool);
        conn.execute(
            "UPDATE persons SET is_active = 0, updated_at = ?1 WHERE id = ?2",
            params![&now, id],
        )
        .map_err(|e| AppError::Db(e.to_string()))?;
    } // release conn before calling person_get to avoid deadlock
    person_get(pool, id)
}

pub fn person_current_projects(
    pool: &DbPool,
    person_id: &str,
) -> Result<Vec<PersonProjectItemDto>, AppError> {
    let conn = get_connection(pool);
    let mut stmt = conn
        .prepare(
            "SELECT p.id, p.name, p.current_status, p.updated_at
             FROM assignments a
             JOIN projects p ON p.id = a.project_id
             WHERE a.person_id = ?1 AND a.end_at IS NULL AND p.current_status <> 'ARCHIVED'
             ORDER BY p.updated_at DESC",
        )
        .map_err(|e| AppError::Db(e.to_string()))?;
    let rows = stmt.query_map([person_id], |r| {
        Ok(PersonProjectItemDto {
            id: r.get(0)?,
            name: r.get(1)?,
            current_status: r.get(2)?,
            updated_at: r.get(3)?,
            last_involved_at: None,
        })
    })?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r.map_err(|e| AppError::Db(e.to_string()))?);
    }
    Ok(out)
}

pub fn person_all_projects(
    pool: &DbPool,
    person_id: &str,
) -> Result<Vec<PersonProjectItemDto>, AppError> {
    let conn = get_connection(pool);
    let mut stmt = conn
        .prepare(
            "SELECT p.id, p.name, p.current_status, p.updated_at,
                    MAX(COALESCE(a.end_at, a.start_at)) AS last_involved_at
             FROM assignments a
             JOIN projects p ON p.id = a.project_id
             WHERE a.person_id = ?1
             GROUP BY p.id
             ORDER BY last_involved_at DESC",
        )
        .map_err(|e| AppError::Db(e.to_string()))?;
    let rows = stmt.query_map([person_id], |r| {
        Ok(PersonProjectItemDto {
            id: r.get(0)?,
            name: r.get(1)?,
            current_status: r.get(2)?,
            updated_at: r.get(3)?,
            last_involved_at: r.get(4)?,
        })
    })?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r.map_err(|e| AppError::Db(e.to_string()))?);
    }
    Ok(out)
}

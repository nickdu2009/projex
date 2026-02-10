//! Partner use cases.

use crate::error::AppError;
use crate::infra::get_connection;
use crate::infra::DbPool;
use chrono::Utc;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PartnerCreateReq {
    pub name: String,
    pub note: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct PartnerDto {
    pub id: String,
    pub name: String,
    pub note: String,
    pub is_active: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PartnerUpdateReq {
    pub id: String,
    pub name: Option<String>,
    pub note: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct PartnerProjectItemDto {
    pub id: String,
    pub name: String,
    pub current_status: String,
    pub updated_at: String,
}

pub fn partner_create(pool: &DbPool, req: PartnerCreateReq) -> Result<PartnerDto, AppError> {
    let name = req.name.trim();
    if name.is_empty() {
        return Err(AppError::Validation("name is required".into()));
    }
    let id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();
    let note = req.note.unwrap_or_default();

    let conn = get_connection(pool);
    conn.execute(
        "INSERT INTO partners (id, name, note, is_active, created_at, updated_at) VALUES (?1, ?2, ?3, 1, ?4, ?4)",
        params![id, name, note, &now],
    )
    .map_err(|e| AppError::Db(e.to_string()))?;

    Ok(PartnerDto {
        id: id.clone(),
        name: name.to_string(),
        note,
        is_active: true,
        created_at: now.clone(),
        updated_at: now,
    })
}

pub fn partner_list(pool: &DbPool, only_active: bool) -> Result<Vec<PartnerDto>, AppError> {
    let conn = get_connection(pool);
    let sql = if only_active {
        "SELECT id, name, note, is_active, created_at, updated_at FROM partners WHERE is_active = 1 ORDER BY name COLLATE NOCASE"
    } else {
        "SELECT id, name, note, is_active, created_at, updated_at FROM partners ORDER BY name COLLATE NOCASE"
    };
    let mut stmt = conn.prepare(sql).map_err(|e| AppError::Db(e.to_string()))?;
    let rows = stmt.query_map([], |row| {
        Ok(PartnerDto {
            id: row.get(0)?,
            name: row.get(1)?,
            note: row.get(2)?,
            is_active: row.get::<_, i32>(3)? != 0,
            created_at: row.get(4)?,
            updated_at: row.get(5)?,
        })
    })?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r.map_err(|e| AppError::Db(e.to_string()))?);
    }
    Ok(out)
}

pub fn partner_get(pool: &DbPool, id: &str) -> Result<PartnerDto, AppError> {
    let conn = get_connection(pool);
    conn.query_row(
        "SELECT id, name, note, is_active, created_at, updated_at FROM partners WHERE id = ?1",
        [id],
        |row| {
            Ok(PartnerDto {
                id: row.get(0)?,
                name: row.get(1)?,
                note: row.get(2)?,
                is_active: row.get::<_, i32>(3)? != 0,
                created_at: row.get(4)?,
                updated_at: row.get(5)?,
            })
        },
    )
    .map_err(|e| AppError::NotFound(e.to_string()))
}

pub fn partner_update(pool: &DbPool, req: PartnerUpdateReq) -> Result<PartnerDto, AppError> {
    let now = Utc::now().to_rfc3339();
    let conn = get_connection(pool);

    let (name, note): (String, String) = conn
        .query_row(
            "SELECT name, note FROM partners WHERE id = ?1",
            [&req.id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .map_err(|_| AppError::NotFound(format!("partner {}", req.id)))?;

    let name = req
        .name
        .as_deref()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or(name);
    let note = req.note.unwrap_or(note);

    if name.is_empty() {
        return Err(AppError::Validation("name is required".into()));
    }

    conn.execute(
        "UPDATE partners SET name = ?1, note = ?2, updated_at = ?3 WHERE id = ?4",
        params![&name, &note, &now, &req.id],
    )
    .map_err(|e| AppError::Db(e.to_string()))?;

    partner_get(pool, &req.id)
}

pub fn partner_deactivate(pool: &DbPool, id: &str) -> Result<PartnerDto, AppError> {
    let now = Utc::now().to_rfc3339();
    let conn = get_connection(pool);
    conn.execute("UPDATE partners SET is_active = 0, updated_at = ?1 WHERE id = ?2", params![&now, id])
        .map_err(|e| AppError::Db(e.to_string()))?;
    partner_get(pool, id)
}

pub fn partner_projects(
    pool: &DbPool,
    partner_id: &str,
) -> Result<Vec<PartnerProjectItemDto>, AppError> {
    let conn = get_connection(pool);
    let mut stmt = conn
        .prepare(
            "SELECT id, name, current_status, updated_at FROM projects WHERE partner_id = ?1 ORDER BY updated_at DESC",
        )
        .map_err(|e| AppError::Db(e.to_string()))?;
    let rows = stmt.query_map([partner_id], |r| {
        Ok(PartnerProjectItemDto {
            id: r.get(0)?,
            name: r.get(1)?,
            current_status: r.get(2)?,
            updated_at: r.get(3)?,
        })
    })?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r.map_err(|e| AppError::Db(e.to_string()))?);
    }
    Ok(out)
}

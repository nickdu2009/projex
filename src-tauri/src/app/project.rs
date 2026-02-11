//! Project use cases: create, list, get, change_status.

use crate::domain::{ProjectStatus, StatusMachine};
use crate::error::AppError;
use crate::infra::get_connection;
use crate::infra::DbPool;
use chrono::Utc;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Type alias to reduce complexity of the raw project query tuple.
type ProjectRawRow = (
    String,         // id
    String,         // name
    String,         // description
    i32,            // priority
    String,         // current_status
    String,         // country_code
    String,         // partner_id
    String,         // owner_person_id
    Option<String>, // start_date
    Option<String>, // due_date
    String,         // created_at
    String,         // updated_at
    Option<String>, // archived_at
);

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectCreateReq {
    pub name: String,
    pub description: Option<String>,
    pub priority: Option<i32>,
    pub country_code: String,
    pub partner_id: String,
    pub owner_person_id: String,
    pub start_date: Option<String>,
    pub due_date: Option<String>,
    pub tags: Option<Vec<String>>,
    pub created_by_person_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ProjectDetailDto {
    pub id: String,
    pub name: String,
    pub description: String,
    pub priority: i32,
    pub current_status: String,
    pub country_code: String,
    pub partner_id: String,
    pub owner_person_id: String,
    pub start_date: Option<String>,
    pub due_date: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub archived_at: Option<String>,
    pub tags: Vec<String>,
    pub owner_name: String,
    pub partner_name: String,
    pub assignments: Vec<AssignmentDto>,
    pub status_history: Vec<StatusHistoryDto>,
}

#[derive(Debug, Serialize)]
pub struct AssignmentDto {
    pub id: String,
    pub project_id: String,
    pub person_id: String,
    pub person_name: String,
    pub role: String,
    pub start_at: String,
    pub end_at: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct StatusHistoryDto {
    pub id: String,
    pub project_id: String,
    pub from_status: Option<String>,
    pub to_status: String,
    pub changed_at: String,
    pub changed_by_person_id: Option<String>,
    pub changed_by_name: Option<String>,
    pub note: String,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectListReq {
    pub only_unarchived: Option<bool>,
    pub statuses: Option<Vec<String>>,
    pub country_codes: Option<Vec<String>>,
    pub partner_ids: Option<Vec<String>>,
    pub owner_person_ids: Option<Vec<String>>,
    pub participant_person_ids: Option<Vec<String>>,
    pub tags: Option<Vec<String>>,
    pub sort_by: Option<String>, // "updatedAt" | "priority" | "dueDate"
    pub sort_order: Option<String>, // "asc" | "desc"
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}

#[derive(Debug, Serialize)]
pub struct ProjectListItemDto {
    pub id: String,
    pub name: String,
    pub current_status: String,
    pub priority: i32,
    pub country_code: String,
    pub partner_name: String,
    pub owner_name: String,
    pub due_date: Option<String>,
    pub updated_at: String,
    pub tags: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct ProjectListPage {
    pub items: Vec<ProjectListItemDto>,
    pub total: i64,
    pub limit: i32,
    pub offset: i32,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectUpdateReq {
    pub id: String,
    pub name: Option<String>,
    pub description: Option<String>,
    pub priority: Option<i32>,
    pub country_code: Option<String>,
    pub owner_person_id: Option<String>,
    pub start_date: Option<String>,
    pub due_date: Option<String>,
    pub tags: Option<Vec<String>>,
    #[serde(default)]
    pub partner_id: Option<String>, // if present -> PARTNER_IMMUTABLE
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectChangeStatusReq {
    pub project_id: String,
    pub to_status: String,
    pub note: Option<String>,
    pub changed_by_person_id: Option<String>,
    pub if_match_updated_at: Option<String>,
}

fn parse_status(s: &str) -> Option<ProjectStatus> {
    s.parse::<ProjectStatus>().ok()
}

pub fn project_create(pool: &DbPool, req: ProjectCreateReq) -> Result<ProjectDetailDto, AppError> {
    let name = req.name.trim();
    if name.is_empty() {
        return Err(AppError::Validation("name is required".into()));
    }
    if req.country_code.trim().is_empty() {
        return Err(AppError::Validation("country_code is required".into()));
    }
    if req.partner_id.trim().is_empty() {
        return Err(AppError::Validation("partner_id is required".into()));
    }
    if req.owner_person_id.trim().is_empty() {
        return Err(AppError::Validation("owner_person_id is required".into()));
    }

    let id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();
    let desc = req.description.unwrap_or_default();
    let priority = req.priority.unwrap_or(3).clamp(1, 5);
    let partner_id = req.partner_id.trim().to_string();
    let owner_person_id = req.owner_person_id.trim().to_string();
    let country_code = req.country_code.trim().to_uppercase();
    let start_date = req.start_date.filter(|s| !s.trim().is_empty());
    let due_date = req.due_date.filter(|s| !s.trim().is_empty());
    let tags = req.tags.unwrap_or_default();
    let created_by = req.created_by_person_id.filter(|s| !s.trim().is_empty());

    {
        let conn = get_connection(pool);
        let tx = conn
            .unchecked_transaction()
            .map_err(|e| AppError::Db(e.to_string()))?;

        tx.execute(
            "INSERT INTO projects (id, name, description, priority, current_status, country_code, partner_id, owner_person_id, start_date, due_date, created_at, updated_at, archived_at) VALUES (?1, ?2, ?3, ?4, 'BACKLOG', ?5, ?6, ?7, ?8, ?9, ?10, ?10, NULL)",
            params![
                id,
                name,
                desc,
                priority,
                country_code,
                partner_id,
                owner_person_id,
                start_date,
                due_date,
                &now
            ],
        )
        .map_err(|e| AppError::Db(e.to_string()))?;

        let assign_id = Uuid::new_v4().to_string();
        tx.execute(
            "INSERT INTO assignments (id, project_id, person_id, role, start_at, end_at, created_at) VALUES (?1, ?2, ?3, 'owner', ?4, NULL, ?4)",
            params![assign_id, &id, &owner_person_id, &now],
        )
        .map_err(|e| AppError::Db(e.to_string()))?;

        let hist_id = Uuid::new_v4().to_string();
        tx.execute(
            "INSERT INTO status_history (id, project_id, from_status, to_status, changed_at, changed_by_person_id, note) VALUES (?1, ?2, NULL, 'BACKLOG', ?3, ?4, '')",
            params![hist_id, &id, &now, created_by],
        )
        .map_err(|e| AppError::Db(e.to_string()))?;

        for tag in &tags {
            let tag = tag.trim();
            if !tag.is_empty() {
                tx.execute(
                    "INSERT INTO project_tags (project_id, tag, created_at) VALUES (?1, ?2, ?3)",
                    params![&id, tag, &now],
                )
                .map_err(|e| AppError::Db(e.to_string()))?;
            }
        }

        tx.commit().map_err(|e| AppError::Db(e.to_string()))?;
    } // release conn before calling project_get to avoid deadlock

    project_get(pool, &id)
}

pub fn project_get(pool: &DbPool, project_id: &str) -> Result<ProjectDetailDto, AppError> {
    let conn = get_connection(pool);

    let proj: ProjectRawRow = conn
        .query_row(
            "SELECT id, name, description, priority, current_status, country_code, partner_id, owner_person_id, start_date, due_date, created_at, updated_at, archived_at FROM projects WHERE id = ?1",
            [project_id],
            |r| {
                Ok((
                    r.get(0)?,
                    r.get(1)?,
                    r.get(2)?,
                    r.get(3)?,
                    r.get(4)?,
                    r.get(5)?,
                    r.get(6)?,
                    r.get(7)?,
                    r.get(8)?,
                    r.get(9)?,
                    r.get(10)?,
                    r.get(11)?,
                    r.get(12)?,
                ))
            },
        )
        .map_err(|e| AppError::NotFound(e.to_string()))?;

    let owner_name: String = conn
        .query_row(
            "SELECT display_name FROM persons WHERE id = ?1",
            [&proj.7],
            |r| r.get(0),
        )
        .unwrap_or_else(|_| "?".to_string());

    let partner_name: String = conn
        .query_row("SELECT name FROM partners WHERE id = ?1", [&proj.6], |r| {
            r.get(0)
        })
        .unwrap_or_else(|_| "?".to_string());

    let mut assignments = Vec::new();
    let mut stmt = conn
        .prepare(
            "SELECT a.id, a.project_id, a.person_id, p.display_name, a.role, a.start_at, a.end_at, a.created_at FROM assignments a LEFT JOIN persons p ON p.id = a.person_id WHERE a.project_id = ?1 ORDER BY a.start_at DESC",
        )
        .map_err(|e| AppError::Db(e.to_string()))?;
    let rows = stmt.query_map([project_id], |r| {
        Ok(AssignmentDto {
            id: r.get(0)?,
            project_id: r.get(1)?,
            person_id: r.get(2)?,
            person_name: r.get(3)?,
            role: r.get(4)?,
            start_at: r.get(5)?,
            end_at: r.get(6)?,
            created_at: r.get(7)?,
        })
    })?;
    for r in rows {
        assignments.push(r.map_err(|e| AppError::Db(e.to_string()))?);
    }

    let mut status_history = Vec::new();
    let mut stmt = conn
        .prepare(
            "SELECT h.id, h.project_id, h.from_status, h.to_status, h.changed_at, h.changed_by_person_id, p.display_name, h.note FROM status_history h LEFT JOIN persons p ON p.id = h.changed_by_person_id WHERE h.project_id = ?1 ORDER BY h.changed_at DESC",
        )
        .map_err(|e| AppError::Db(e.to_string()))?;
    let rows = stmt.query_map([project_id], |r| {
        Ok(StatusHistoryDto {
            id: r.get(0)?,
            project_id: r.get(1)?,
            from_status: r.get(2)?,
            to_status: r.get(3)?,
            changed_at: r.get(4)?,
            changed_by_person_id: r.get(5)?,
            changed_by_name: r.get(6)?,
            note: r.get(7)?,
        })
    })?;
    for r in rows {
        status_history.push(r.map_err(|e| AppError::Db(e.to_string()))?);
    }

    let mut tags = Vec::new();
    let mut stmt = conn
        .prepare("SELECT tag FROM project_tags WHERE project_id = ?1")
        .map_err(|e| AppError::Db(e.to_string()))?;
    let rows = stmt.query_map([project_id], |r| r.get::<_, String>(0))?;
    for r in rows {
        tags.push(r.map_err(|e| AppError::Db(e.to_string()))?);
    }

    Ok(ProjectDetailDto {
        id: proj.0,
        name: proj.1,
        description: proj.2,
        priority: proj.3,
        current_status: proj.4,
        country_code: proj.5,
        partner_id: proj.6.clone(),
        owner_person_id: proj.7.clone(),
        start_date: proj.8,
        due_date: proj.9,
        created_at: proj.10,
        updated_at: proj.11,
        archived_at: proj.12,
        tags,
        owner_name,
        partner_name,
        assignments,
        status_history,
    })
}

pub fn project_update(pool: &DbPool, req: ProjectUpdateReq) -> Result<ProjectDetailDto, AppError> {
    if req.partner_id.is_some() {
        return Err(AppError::PartnerImmutable);
    }
    let now = Utc::now().to_rfc3339();

    {
        let conn = get_connection(pool);
        let tx = conn
            .unchecked_transaction()
            .map_err(|e| AppError::Db(e.to_string()))?;

        let (name, desc, priority, country_code, owner_id, start_date, due_date): (
            String,
            String,
            i32,
            String,
            String,
            Option<String>,
            Option<String>,
        ) = tx
            .query_row(
                "SELECT name, description, priority, country_code, owner_person_id, start_date, due_date FROM projects WHERE id = ?1",
                [&req.id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?, r.get(6)?)),
            )
            .map_err(|_| AppError::NotFound(format!("project {}", req.id)))?;

        let name = req.name.as_deref().unwrap_or(&name).trim().to_string();
        let desc = req.description.as_deref().unwrap_or(&desc).to_string();
        let priority = req.priority.unwrap_or(priority).clamp(1, 5);
        let country_code = req
            .country_code
            .as_deref()
            .map(|s| s.trim().to_uppercase())
            .unwrap_or(country_code);
        let owner_person_id = req
            .owner_person_id
            .as_deref()
            .map(|s| s.trim().to_string())
            .unwrap_or_else(|| owner_id.clone());
        let start_date = req
            .start_date
            .as_ref()
            .or(start_date.as_ref())
            .filter(|s| !s.trim().is_empty())
            .cloned();
        let due_date = req
            .due_date
            .as_ref()
            .or(due_date.as_ref())
            .filter(|s| !s.trim().is_empty())
            .cloned();

        if name.is_empty() {
            return Err(AppError::Validation("name is required".into()));
        }

        // If owner changed: demote old owner to member, then ensure new owner has active assignment
        if owner_person_id != owner_id {
            tx.execute(
                "UPDATE assignments SET role = 'member' WHERE project_id = ?1 AND person_id = ?2 AND end_at IS NULL",
                params![&req.id, &owner_id],
            )
            .map_err(|e| AppError::Db(e.to_string()))?;
        }

        // Ensure new owner has active assignment with role owner
        let has_active: i32 = tx
            .query_row(
                "SELECT COUNT(1) FROM assignments WHERE project_id = ?1 AND person_id = ?2 AND end_at IS NULL",
                params![&req.id, &owner_person_id],
                |r| r.get(0),
            )
            .unwrap_or(0);
        if has_active == 0 {
            let assign_id = Uuid::new_v4().to_string();
            tx.execute(
                "INSERT INTO assignments (id, project_id, person_id, role, start_at, end_at, created_at) VALUES (?1, ?2, ?3, 'owner', ?4, NULL, ?4)",
                params![assign_id, &req.id, &owner_person_id, &now],
            )
            .map_err(|e| AppError::Db(e.to_string()))?;
        } else {
            tx.execute(
                "UPDATE assignments SET role = 'owner' WHERE project_id = ?1 AND person_id = ?2 AND end_at IS NULL",
                params![&req.id, &owner_person_id],
            )
            .map_err(|e| AppError::Db(e.to_string()))?;
        }

        tx.execute(
            "UPDATE projects SET name=?1, description=?2, priority=?3, country_code=?4, owner_person_id=?5, start_date=?6, due_date=?7, updated_at=?8 WHERE id=?9",
            params![
                name,
                desc,
                priority,
                country_code,
                owner_person_id,
                start_date,
                due_date,
                &now,
                &req.id
            ],
        )
        .map_err(|e| AppError::Db(e.to_string()))?;

        if let Some(ref tags) = req.tags {
            tx.execute("DELETE FROM project_tags WHERE project_id = ?1", [&req.id])
                .map_err(|e| AppError::Db(e.to_string()))?;
            for tag in tags {
                let tag = tag.trim();
                if !tag.is_empty() {
                    tx.execute(
                        "INSERT INTO project_tags (project_id, tag, created_at) VALUES (?1, ?2, ?3)",
                        params![&req.id, tag, &now],
                    )
                    .map_err(|e| AppError::Db(e.to_string()))?;
                }
            }
        }

        tx.commit().map_err(|e| AppError::Db(e.to_string()))?;
    }
    project_get(pool, &req.id)
}

pub fn project_list(pool: &DbPool, req: ProjectListReq) -> Result<ProjectListPage, AppError> {
    use rusqlite::types::Value;

    let only_unarchived = req.only_unarchived.unwrap_or(true);
    let limit = req.limit.unwrap_or(50).clamp(1, 200);
    let offset = req.offset.unwrap_or(0).max(0);

    let conn = get_connection(pool);

    // --- build dynamic WHERE clauses ---
    let mut conditions: Vec<String> = Vec::new();
    let mut bind_values: Vec<Value> = Vec::new();

    if only_unarchived {
        conditions.push("p.current_status <> 'ARCHIVED'".to_string());
    }

    if let Some(ref statuses) = req.statuses {
        let v: Vec<&String> = statuses.iter().filter(|s| !s.is_empty()).collect();
        if !v.is_empty() {
            let ph: Vec<String> = v.iter().enumerate().map(|_| "?".to_string()).collect();
            conditions.push(format!("p.current_status IN ({})", ph.join(",")));
            for s in v {
                bind_values.push(Value::Text(s.clone()));
            }
        }
    }

    if let Some(ref codes) = req.country_codes {
        let v: Vec<&String> = codes.iter().filter(|s| !s.is_empty()).collect();
        if !v.is_empty() {
            let ph: Vec<String> = v.iter().enumerate().map(|_| "?".to_string()).collect();
            conditions.push(format!("p.country_code IN ({})", ph.join(",")));
            for s in v {
                bind_values.push(Value::Text(s.clone()));
            }
        }
    }

    if let Some(ref pids) = req.partner_ids {
        let v: Vec<&String> = pids.iter().filter(|s| !s.is_empty()).collect();
        if !v.is_empty() {
            let ph: Vec<String> = v.iter().enumerate().map(|_| "?".to_string()).collect();
            conditions.push(format!("p.partner_id IN ({})", ph.join(",")));
            for s in v {
                bind_values.push(Value::Text(s.clone()));
            }
        }
    }

    if let Some(ref oids) = req.owner_person_ids {
        let v: Vec<&String> = oids.iter().filter(|s| !s.is_empty()).collect();
        if !v.is_empty() {
            let ph: Vec<String> = v.iter().enumerate().map(|_| "?".to_string()).collect();
            conditions.push(format!("p.owner_person_id IN ({})", ph.join(",")));
            for s in v {
                bind_values.push(Value::Text(s.clone()));
            }
        }
    }

    if let Some(ref ppids) = req.participant_person_ids {
        let v: Vec<&String> = ppids.iter().filter(|s| !s.is_empty()).collect();
        if !v.is_empty() {
            let ph: Vec<String> = v.iter().enumerate().map(|_| "?".to_string()).collect();
            conditions.push(format!(
                "p.id IN (SELECT DISTINCT project_id FROM assignments WHERE person_id IN ({}))",
                ph.join(",")
            ));
            for s in v {
                bind_values.push(Value::Text(s.clone()));
            }
        }
    }

    if let Some(ref tag_list) = req.tags {
        let v: Vec<&String> = tag_list.iter().filter(|s| !s.is_empty()).collect();
        if !v.is_empty() {
            let ph: Vec<String> = v.iter().enumerate().map(|_| "?".to_string()).collect();
            conditions.push(format!(
                "p.id IN (SELECT DISTINCT project_id FROM project_tags WHERE tag IN ({}))",
                ph.join(",")
            ));
            for s in v {
                bind_values.push(Value::Text(s.clone()));
            }
        }
    }

    let where_clause = if conditions.is_empty() {
        String::new()
    } else {
        format!(" WHERE {}", conditions.join(" AND "))
    };

    // --- COUNT total ---
    let count_sql = format!("SELECT COUNT(*) FROM projects p{}", where_clause);
    let count_params: Vec<&dyn rusqlite::types::ToSql> = bind_values
        .iter()
        .map(|v| v as &dyn rusqlite::types::ToSql)
        .collect();
    let total: i64 = conn
        .query_row(&count_sql, count_params.as_slice(), |r| r.get(0))
        .map_err(|e| AppError::Db(e.to_string()))?;

    // --- ORDER BY ---
    let order_clause = match req.sort_by.as_deref() {
        Some("priority") => {
            let dir = match req.sort_order.as_deref() {
                Some("desc") => "DESC",
                _ => "ASC",
            };
            format!(" ORDER BY p.priority {}, p.updated_at DESC", dir)
        }
        Some("dueDate") => {
            let dir = match req.sort_order.as_deref() {
                Some("desc") => "DESC",
                _ => "ASC",
            };
            // NULL due_dates sort last regardless of direction
            format!(" ORDER BY CASE WHEN p.due_date IS NULL THEN 1 ELSE 0 END, p.due_date {}, p.updated_at DESC", dir)
        }
        _ => {
            // default: updatedAt DESC
            let dir = match req.sort_order.as_deref() {
                Some("asc") => "ASC",
                _ => "DESC",
            };
            format!(" ORDER BY p.updated_at {}", dir)
        }
    };

    // --- main query ---
    let data_sql = format!(
        "SELECT p.id, p.name, p.current_status, p.priority, p.country_code, \
         COALESCE(pt.name, '?') AS partner_name, COALESCE(pe.display_name, '?') AS owner_name, \
         p.due_date, p.updated_at \
         FROM projects p \
         LEFT JOIN partners pt ON pt.id = p.partner_id \
         LEFT JOIN persons pe ON pe.id = p.owner_person_id\
         {}{} LIMIT ? OFFSET ?",
        where_clause, order_clause
    );

    let mut all_params = bind_values.clone();
    all_params.push(Value::Integer(limit as i64));
    all_params.push(Value::Integer(offset as i64));

    let all_refs: Vec<&dyn rusqlite::types::ToSql> = all_params
        .iter()
        .map(|v| v as &dyn rusqlite::types::ToSql)
        .collect();

    let mut stmt = conn
        .prepare(&data_sql)
        .map_err(|e| AppError::Db(e.to_string()))?;
    let mut rows = stmt
        .query(all_refs.as_slice())
        .map_err(|e| AppError::Db(e.to_string()))?;
    let mut items = Vec::new();
    while let Some(row) = rows.next().map_err(|e| AppError::Db(e.to_string()))? {
        let id: String = row.get(0)?;
        let mut tags = Vec::new();
        {
            let mut tag_stmt =
                conn.prepare("SELECT tag FROM project_tags WHERE project_id = ?1")?;
            let tag_rows = tag_stmt.query_map([&id], |r| r.get::<_, String>(0))?;
            for t in tag_rows.flatten() {
                tags.push(t);
            }
        }
        items.push(ProjectListItemDto {
            id,
            name: row.get(1)?,
            current_status: row.get(2)?,
            priority: row.get(3)?,
            country_code: row.get(4)?,
            partner_name: row.get(5)?,
            owner_name: row.get(6)?,
            due_date: row.get(7)?,
            updated_at: row.get(8)?,
            tags,
        });
    }

    Ok(ProjectListPage {
        items,
        total,
        limit,
        offset,
    })
}

pub fn project_change_status(
    pool: &DbPool,
    req: ProjectChangeStatusReq,
) -> Result<ProjectDetailDto, AppError> {
    let to_status = parse_status(&req.to_status).ok_or_else(|| {
        AppError::InvalidStatusTransition(format!("unknown status: {}", req.to_status))
    })?;

    {
        let conn = get_connection(pool);
        let tx = conn
            .unchecked_transaction()
            .map_err(|e| AppError::Db(e.to_string()))?;

        let (current_status, updated_at): (String, String) = tx
            .query_row(
                "SELECT current_status, updated_at FROM projects WHERE id = ?1",
                [&req.project_id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .map_err(|_| AppError::NotFound(format!("project {}", req.project_id)))?;

        if let Some(ref if_match) = req.if_match_updated_at {
            if if_match != &updated_at {
                return Err(AppError::Conflict("project was modified".into()));
            }
        }

        let from_status = parse_status(&current_status);

        if !StatusMachine::can_transition(from_status, to_status) {
            return Err(AppError::InvalidStatusTransition(format!(
                "{} -> {}",
                current_status,
                to_status.as_str()
            )));
        }

        if StatusMachine::note_required(from_status, to_status) {
            let note = req.note.as_deref().unwrap_or("").trim();
            if note.is_empty() {
                return Err(AppError::NoteRequired);
            }
        }

        let now = Utc::now().to_rfc3339();
        let hist_id = Uuid::new_v4().to_string();
        let note = req.note.unwrap_or_default();
        let changed_by = req.changed_by_person_id;

        tx.execute(
            "INSERT INTO status_history (id, project_id, from_status, to_status, changed_at, changed_by_person_id, note) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                hist_id,
                &req.project_id,
                current_status,
                to_status.as_str(),
                &now,
                changed_by,
                note
            ],
        )
        .map_err(|e| AppError::Db(e.to_string()))?;

        let archived_at: Option<&str> = if to_status == ProjectStatus::Archived {
            Some(&now)
        } else {
            None
        };

        tx.execute(
            "UPDATE projects SET current_status = ?1, updated_at = ?2, archived_at = ?3 WHERE id = ?4",
            params![
                to_status.as_str(),
                &now,
                archived_at,
                &req.project_id
            ],
        )
        .map_err(|e| AppError::Db(e.to_string()))?;

        tx.commit().map_err(|e| AppError::Db(e.to_string()))?;
    } // release conn before project_get to avoid deadlock
    project_get(pool, &req.project_id)
}

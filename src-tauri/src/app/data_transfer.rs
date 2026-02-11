//! Export / Import use cases: export all data to JSON, import from JSON.

use crate::error::AppError;
use crate::infra::{get_connection, DbPool};
use chrono::Utc;
use rusqlite::params;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportRoot {
    pub schema_version: i32,
    pub exported_at: String,
    pub persons: Vec<ExportPerson>,
    pub partners: Vec<ExportPartner>,
    pub projects: Vec<ExportProject>,
    pub assignments: Vec<ExportAssignment>,
    pub status_history: Vec<ExportStatusHistory>,
    pub comments: Vec<ExportComment>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportPerson {
    pub id: String,
    pub display_name: String,
    pub email: String,
    pub role: String,
    pub note: String,
    pub is_active: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportPartner {
    pub id: String,
    pub name: String,
    pub note: String,
    pub is_active: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportProject {
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
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportAssignment {
    pub id: String,
    pub project_id: String,
    pub person_id: String,
    pub role: String,
    pub start_at: String,
    pub end_at: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportStatusHistory {
    pub id: String,
    pub project_id: String,
    pub from_status: Option<String>,
    pub to_status: String,
    pub changed_at: String,
    pub changed_by_person_id: Option<String>,
    pub note: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportComment {
    pub id: String,
    pub project_id: String,
    pub person_id: Option<String>,
    pub content: String,
    pub is_pinned: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportResult {
    pub persons: usize,
    pub partners: usize,
    pub projects: usize,
    pub assignments: usize,
    pub status_history: usize,
    pub comments: usize,
    pub skipped_duplicates: usize,
}

/// Export all data as JSON string
pub fn export_json_string(pool: &DbPool, _schema_version: Option<i32>) -> Result<String, AppError> {
    let schema_version = 2; // Current schema version (updated for comments support)
    let exported_at = Utc::now().to_rfc3339();

    let conn = get_connection(pool);

    // 1. Export persons
    let mut persons = Vec::new();
    let mut stmt = conn
        .prepare("SELECT id, display_name, email, role, note, is_active, created_at, updated_at FROM persons ORDER BY display_name")
        .map_err(|e| AppError::Db(e.to_string()))?;
    let mut rows = stmt
        .query([])
        .map_err(|e| AppError::Db(e.to_string()))?;
    while let Some(row) = rows.next().map_err(|e| AppError::Db(e.to_string()))? {
        persons.push(ExportPerson {
            id: row.get(0)?,
            display_name: row.get(1)?,
            email: row.get(2)?,
            role: row.get(3)?,
            note: row.get(4)?,
            is_active: row.get::<_, i32>(5)? != 0,
            created_at: row.get(6)?,
            updated_at: row.get(7)?,
        });
    }

    // 2. Export partners
    let mut partners = Vec::new();
    let mut stmt = conn
        .prepare("SELECT id, name, note, is_active, created_at, updated_at FROM partners ORDER BY name")
        .map_err(|e| AppError::Db(e.to_string()))?;
    let mut rows = stmt
        .query([])
        .map_err(|e| AppError::Db(e.to_string()))?;
    while let Some(row) = rows.next().map_err(|e| AppError::Db(e.to_string()))? {
        partners.push(ExportPartner {
            id: row.get(0)?,
            name: row.get(1)?,
            note: row.get(2)?,
            is_active: row.get::<_, i32>(3)? != 0,
            created_at: row.get(4)?,
            updated_at: row.get(5)?,
        });
    }

    // 3. Export projects (with tags)
    let mut projects = Vec::new();
    let mut stmt = conn
        .prepare("SELECT id, name, description, priority, current_status, country_code, partner_id, owner_person_id, start_date, due_date, created_at, updated_at, archived_at FROM projects ORDER BY created_at DESC")
        .map_err(|e| AppError::Db(e.to_string()))?;
    let mut rows = stmt
        .query([])
        .map_err(|e| AppError::Db(e.to_string()))?;
    while let Some(row) = rows.next().map_err(|e| AppError::Db(e.to_string()))? {
        let project_id: String = row.get(0)?;
        
        // Get tags for this project
        let mut tags = Vec::new();
        let mut tag_stmt = conn
            .prepare("SELECT tag FROM project_tags WHERE project_id = ?1 ORDER BY tag")
            .map_err(|e| AppError::Db(e.to_string()))?;
        let tag_rows = tag_stmt
            .query_map([&project_id], |r| r.get::<_, String>(0))
            .map_err(|e| AppError::Db(e.to_string()))?;
        for tag_result in tag_rows {
            if let Ok(tag) = tag_result {
                tags.push(tag);
            }
        }

        projects.push(ExportProject {
            id: project_id,
            name: row.get(1)?,
            description: row.get(2)?,
            priority: row.get(3)?,
            current_status: row.get(4)?,
            country_code: row.get(5)?,
            partner_id: row.get(6)?,
            owner_person_id: row.get(7)?,
            start_date: row.get(8)?,
            due_date: row.get(9)?,
            created_at: row.get(10)?,
            updated_at: row.get(11)?,
            archived_at: row.get(12)?,
            tags,
        });
    }

    // 4. Export assignments
    let mut assignments = Vec::new();
    let mut stmt = conn
        .prepare("SELECT id, project_id, person_id, role, start_at, end_at, created_at FROM assignments ORDER BY start_at DESC")
        .map_err(|e| AppError::Db(e.to_string()))?;
    let mut rows = stmt
        .query([])
        .map_err(|e| AppError::Db(e.to_string()))?;
    while let Some(row) = rows.next().map_err(|e| AppError::Db(e.to_string()))? {
        assignments.push(ExportAssignment {
            id: row.get(0)?,
            project_id: row.get(1)?,
            person_id: row.get(2)?,
            role: row.get(3)?,
            start_at: row.get(4)?,
            end_at: row.get(5)?,
            created_at: row.get(6)?,
        });
    }

    // 5. Export status history
    let mut status_history = Vec::new();
    let mut stmt = conn
        .prepare("SELECT id, project_id, from_status, to_status, changed_at, changed_by_person_id, note FROM status_history ORDER BY changed_at DESC")
        .map_err(|e| AppError::Db(e.to_string()))?;
    let mut rows = stmt
        .query([])
        .map_err(|e| AppError::Db(e.to_string()))?;
    while let Some(row) = rows.next().map_err(|e| AppError::Db(e.to_string()))? {
        status_history.push(ExportStatusHistory {
            id: row.get(0)?,
            project_id: row.get(1)?,
            from_status: row.get(2)?,
            to_status: row.get(3)?,
            changed_at: row.get(4)?,
            changed_by_person_id: row.get(5)?,
            note: row.get(6)?,
        });
    }

    // 6. Export comments
    let mut comments = Vec::new();
    let mut stmt = conn
        .prepare("SELECT id, project_id, person_id, content, is_pinned, created_at, updated_at FROM project_comments ORDER BY created_at DESC")
        .map_err(|e| AppError::Db(e.to_string()))?;
    let mut rows = stmt
        .query([])
        .map_err(|e| AppError::Db(e.to_string()))?;
    while let Some(row) = rows.next().map_err(|e| AppError::Db(e.to_string()))? {
        comments.push(ExportComment {
            id: row.get(0)?,
            project_id: row.get(1)?,
            person_id: row.get(2)?,
            content: row.get(3)?,
            is_pinned: row.get::<_, i32>(4)? != 0,
            created_at: row.get(5)?,
            updated_at: row.get(6)?,
        });
    }

    let export_root = ExportRoot {
        schema_version,
        exported_at,
        persons,
        partners,
        projects,
        assignments,
        status_history,
        comments,
    };

    serde_json::to_string_pretty(&export_root).map_err(|e| AppError::Db(format!("JSON serialization failed: {}", e)))
}

/// Import data from JSON string. Uses INSERT OR IGNORE for idempotency (duplicate IDs are skipped).
pub fn import_json_string(pool: &DbPool, json: &str) -> Result<ImportResult, AppError> {
    let root: ExportRoot = serde_json::from_str(json)
        .map_err(|e| AppError::Validation(format!("Invalid JSON: {}", e)))?;

    // Support both schema version 1 (without comments) and 2 (with comments)
    if root.schema_version < 1 || root.schema_version > 2 {
        return Err(AppError::Validation(format!(
            "Unsupported schema version: {} (expected 1 or 2)",
            root.schema_version
        )));
    }

    let conn = get_connection(pool);
    let tx = conn
        .unchecked_transaction()
        .map_err(|e| AppError::Db(e.to_string()))?;

    let mut skipped = 0usize;

    // 1. Import persons (must come before projects/assignments due to FK)
    let mut persons_count = 0usize;
    for p in &root.persons {
        let changed = tx.execute(
            "INSERT OR IGNORE INTO persons (id, display_name, email, role, note, is_active, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![p.id, p.display_name, p.email, p.role, p.note, p.is_active as i32, p.created_at, p.updated_at],
        ).map_err(|e| AppError::Db(e.to_string()))?;
        if changed > 0 { persons_count += 1; } else { skipped += 1; }
    }

    // 2. Import partners (must come before projects due to FK)
    let mut partners_count = 0usize;
    for p in &root.partners {
        let changed = tx.execute(
            "INSERT OR IGNORE INTO partners (id, name, note, is_active, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![p.id, p.name, p.note, p.is_active as i32, p.created_at, p.updated_at],
        ).map_err(|e| AppError::Db(e.to_string()))?;
        if changed > 0 { partners_count += 1; } else { skipped += 1; }
    }

    // 3. Import projects
    let mut projects_count = 0usize;
    for p in &root.projects {
        let changed = tx.execute(
            "INSERT OR IGNORE INTO projects (id, name, description, priority, current_status, country_code, partner_id, owner_person_id, start_date, due_date, created_at, updated_at, archived_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            params![p.id, p.name, p.description, p.priority, p.current_status, p.country_code, p.partner_id, p.owner_person_id, p.start_date, p.due_date, p.created_at, p.updated_at, p.archived_at],
        ).map_err(|e| AppError::Db(e.to_string()))?;
        if changed > 0 {
            projects_count += 1;
            // Import tags for this project
            for tag in &p.tags {
                tx.execute(
                    "INSERT OR IGNORE INTO project_tags (project_id, tag, created_at) VALUES (?1, ?2, ?3)",
                    params![p.id, tag, p.created_at],
                ).map_err(|e| AppError::Db(e.to_string()))?;
            }
        } else {
            skipped += 1;
        }
    }

    // 4. Import assignments
    let mut assignments_count = 0usize;
    for a in &root.assignments {
        let changed = tx.execute(
            "INSERT OR IGNORE INTO assignments (id, project_id, person_id, role, start_at, end_at, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![a.id, a.project_id, a.person_id, a.role, a.start_at, a.end_at, a.created_at],
        ).map_err(|e| AppError::Db(e.to_string()))?;
        if changed > 0 { assignments_count += 1; } else { skipped += 1; }
    }

    // 5. Import status_history
    let mut history_count = 0usize;
    for h in &root.status_history {
        let changed = tx.execute(
            "INSERT OR IGNORE INTO status_history (id, project_id, from_status, to_status, changed_at, changed_by_person_id, note) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![h.id, h.project_id, h.from_status, h.to_status, h.changed_at, h.changed_by_person_id, h.note],
        ).map_err(|e| AppError::Db(e.to_string()))?;
        if changed > 0 { history_count += 1; } else { skipped += 1; }
    }

    // 6. Import comments (schema version 2 only)
    let mut comments_count = 0usize;
    for c in &root.comments {
        let changed = tx.execute(
            "INSERT OR IGNORE INTO project_comments (id, project_id, person_id, content, is_pinned, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![c.id, c.project_id, c.person_id, c.content, c.is_pinned as i32, c.created_at, c.updated_at],
        ).map_err(|e| AppError::Db(e.to_string()))?;
        if changed > 0 { comments_count += 1; } else { skipped += 1; }
    }

    tx.commit().map_err(|e| AppError::Db(e.to_string()))?;

    Ok(ImportResult {
        persons: persons_count,
        partners: partners_count,
        projects: projects_count,
        assignments: assignments_count,
        status_history: history_count,
        comments: comments_count,
        skipped_duplicates: skipped,
    })
}

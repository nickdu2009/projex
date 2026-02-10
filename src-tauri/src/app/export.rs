//! Export use case: export all data to JSON.

use crate::error::AppError;
use crate::infra::{get_connection, DbPool};
use chrono::Utc;
use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportRoot {
    pub schema_version: i32,
    pub exported_at: String,
    pub persons: Vec<ExportPerson>,
    pub partners: Vec<ExportPartner>,
    pub projects: Vec<ExportProject>,
    pub assignments: Vec<ExportAssignment>,
    pub status_history: Vec<ExportStatusHistory>,
}

#[derive(Debug, Serialize)]
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

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportPartner {
    pub id: String,
    pub name: String,
    pub note: String,
    pub is_active: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize)]
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

#[derive(Debug, Serialize)]
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

#[derive(Debug, Serialize)]
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

/// Export all data as JSON string
pub fn export_json_string(pool: &DbPool, _schema_version: Option<i32>) -> Result<String, AppError> {
    let schema_version = 1; // Current schema version
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

    let export_root = ExportRoot {
        schema_version,
        exported_at,
        persons,
        partners,
        projects,
        assignments,
        status_history,
    };

    serde_json::to_string_pretty(&export_root).map_err(|e| AppError::Db(format!("JSON serialization failed: {}", e)))
}

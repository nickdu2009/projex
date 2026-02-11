//! Delta sync engine with conflict resolution

use super::vector_clock::VectorClock;
use crate::error::AppError;
use crate::infra::DbPool;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::io::{Read, Write};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OperationType {
    Insert,
    Update,
    Delete,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Operation {
    pub table_name: String,
    pub record_id: String,
    pub op_type: OperationType,
    pub data: Option<serde_json::Value>,
    pub version: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Delta {
    pub id: i64,
    pub operations: Vec<Operation>,
    pub device_id: String,
    pub vector_clock: VectorClock,
    pub created_at: String,
    pub checksum: String,
}

/// Local delta collected from `sync_metadata`.
/// `max_sync_meta_id` is used to mark those rows as synced after successful upload.
pub struct CollectedLocalDelta {
    pub delta: Delta,
    pub max_sync_meta_id: Option<i64>,
}

impl Delta {
    /// Calculate checksum of operations
    pub fn calculate_checksum(operations: &[Operation]) -> String {
        let json = serde_json::to_string(operations).unwrap();
        let mut hasher = Sha256::new();
        hasher.update(json.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Compress delta to bytes (gzip)
    pub fn compress(&self) -> Result<Vec<u8>, AppError> {
        let json = serde_json::to_string(self)
            .map_err(|e| AppError::Db(format!("Serialize delta failed: {}", e)))?;

        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder
            .write_all(json.as_bytes())
            .map_err(|e| AppError::Db(format!("Compress failed: {}", e)))?;

        encoder
            .finish()
            .map_err(|e| AppError::Db(format!("Compress finish failed: {}", e)))
    }

    /// Decompress delta from bytes
    pub fn decompress(data: &[u8]) -> Result<Self, AppError> {
        let mut decoder = GzDecoder::new(data);
        let mut json = String::new();
        decoder
            .read_to_string(&mut json)
            .map_err(|e| AppError::Db(format!("Decompress failed: {}", e)))?;

        serde_json::from_str(&json)
            .map_err(|e| AppError::Db(format!("Deserialize delta failed: {}", e)))
    }
}

pub struct DeltaSyncEngine<'a> {
    pool: &'a DbPool,
    device_id: String,
}

impl<'a> DeltaSyncEngine<'a> {
    pub fn new(pool: &'a DbPool, device_id: String) -> Self {
        Self { pool, device_id }
    }

    /// Get device ID from database
    pub fn get_device_id(conn: &Connection) -> Result<String, AppError> {
        conn.query_row(
            "SELECT value FROM sync_config WHERE key = 'device_id'",
            [],
            |row| row.get(0),
        )
        .map_err(|e| AppError::Db(e.to_string()))
    }

    /// Collect local changes into delta
    pub fn collect_local_delta(&self) -> Result<CollectedLocalDelta, AppError> {
        let conn = self
            .pool
            .0
            .lock()
            .map_err(|e: std::sync::PoisonError<_>| AppError::Db(e.to_string()))?;

        // Get unsynced metadata
        let mut stmt = conn
            .prepare(
                "SELECT id, table_name, record_id, operation, data_snapshot, version, created_at 
                 FROM sync_metadata 
                 WHERE synced = 0 
                 ORDER BY id ASC",
            )
            .map_err(|e: rusqlite::Error| AppError::Db(e.to_string()))?;

        let mut max_sync_meta_id: Option<i64> = None;
        let operations: Vec<Operation> = stmt
            .query_map([], |row: &rusqlite::Row<'_>| {
                let meta_id: i64 = row.get(0)?;
                max_sync_meta_id = Some(max_sync_meta_id.map_or(meta_id, |m| m.max(meta_id)));
                let op_type = match row.get::<_, String>(3)?.as_str() {
                    "INSERT" => OperationType::Insert,
                    "UPDATE" => OperationType::Update,
                    "DELETE" => OperationType::Delete,
                    _ => OperationType::Update,
                };

                let data_json: Option<String> = row.get(4)?;
                let data = data_json.and_then(|s: String| serde_json::from_str(&s).ok());

                Ok(Operation {
                    table_name: row.get(1)?,
                    record_id: row.get(2)?,
                    op_type,
                    data,
                    version: row.get(5)?,
                })
            })
            .map_err(|e: rusqlite::Error| AppError::Db(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e: rusqlite::Error| AppError::Db(e.to_string()))?;

        // Get current vector clock
        let vector_clock = self.get_vector_clock(&conn)?;

        let checksum = Delta::calculate_checksum(&operations);

        Ok(CollectedLocalDelta {
            delta: Delta {
                id: 0, // Will be assigned by sync manager
                operations,
                device_id: self.device_id.clone(),
                vector_clock,
                created_at: chrono::Utc::now().to_rfc3339(),
                checksum,
            },
            max_sync_meta_id,
        })
    }

    /// Get current vector clock from database
    fn get_vector_clock(&self, conn: &Connection) -> Result<VectorClock, AppError> {
        let mut stmt = conn
            .prepare("SELECT device_id, clock_value FROM vector_clocks")
            .map_err(|e: rusqlite::Error| AppError::Db(e.to_string()))?;

        let clocks: std::collections::HashMap<String, i64> = stmt
            .query_map([], |row: &rusqlite::Row<'_>| Ok((row.get(0)?, row.get(1)?)))
            .map_err(|e: rusqlite::Error| AppError::Db(e.to_string()))?
            .collect::<Result<_, _>>()
            .map_err(|e: rusqlite::Error| AppError::Db(e.to_string()))?;

        Ok(VectorClock { clocks })
    }

    /// Apply remote delta to local database
    pub fn apply_delta(&self, delta: &Delta) -> Result<(), AppError> {
        let mut conn = self
            .pool
            .0
            .lock()
            .map_err(|e: std::sync::PoisonError<_>| AppError::Db(e.to_string()))?;

        let tx = conn
            .transaction()
            .map_err(|e| AppError::Db(e.to_string()))?;

        for op in &delta.operations {
            match op.op_type {
                OperationType::Insert | OperationType::Update => {
                    if let Some(data) = &op.data {
                        self.apply_upsert(&tx, &op.table_name, &op.record_id, data, op.version)?;
                    }
                }
                OperationType::Delete => {
                    self.apply_delete(&tx, &op.table_name, &op.record_id)?;
                }
            }
        }

        // Update vector clock
        self.update_vector_clock(&tx, &delta.vector_clock)?;

        tx.commit()
            .map_err(|e: rusqlite::Error| AppError::Db(e.to_string()))?;

        Ok(())
    }

    /// Apply upsert operation
    fn apply_upsert(
        &self,
        tx: &rusqlite::Transaction,
        table: &str,
        record_id: &str,
        data: &serde_json::Value,
        version: i64,
    ) -> Result<(), AppError> {
        // Check for conflicts using vector clock
        let local_vc = self.get_record_vector_clock(tx, table, record_id)?;
        let remote_vc = VectorClock::empty(); // TODO: get from delta

        if local_vc.conflicts_with(&remote_vc) {
            // Conflict! Use LWW resolution
            log::warn!("Conflict detected for {}:{}, using LWW", table, record_id);
            // For now, remote wins (can be improved with timestamp comparison)
        }

        // Build SQL dynamically based on table
        match table {
            "projects" => self.upsert_project(tx, data, version)?,
            "persons" => self.upsert_person(tx, data, version)?,
            "partners" => self.upsert_partner(tx, data, version)?,
            "assignments" => self.upsert_assignment(tx, data, version)?,
            "status_history" => self.upsert_status_history(tx, data, version)?,
            _ => {
                log::warn!("Unknown table for upsert: {}", table);
            }
        }

        Ok(())
    }

    fn upsert_project(
        &self,
        tx: &rusqlite::Transaction,
        data: &serde_json::Value,
        version: i64,
    ) -> Result<(), AppError> {
        tx.execute(
            "INSERT OR REPLACE INTO projects (
                id, name, description, priority, current_status, country_code, 
                partner_id, owner_person_id, start_date, due_date, 
                created_at, updated_at, archived_at, _version
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
            params![
                data["id"].as_str(),
                data["name"].as_str(),
                data["description"].as_str(),
                data["priority"].as_i64(),
                data["current_status"].as_str(),
                data["country_code"].as_str(),
                data["partner_id"].as_str(),
                data["owner_person_id"].as_str(),
                data["start_date"].as_str(),
                data["due_date"].as_str(),
                data["created_at"].as_str(),
                data["updated_at"].as_str(),
                data["archived_at"].as_str(),
                version,
            ],
        )
        .map_err(|e| AppError::Db(e.to_string()))?;

        Ok(())
    }

    fn upsert_person(
        &self,
        tx: &rusqlite::Transaction,
        data: &serde_json::Value,
        version: i64,
    ) -> Result<(), AppError> {
        tx.execute(
            "INSERT OR REPLACE INTO persons (
                id, display_name, email, role, note, is_active, 
                created_at, updated_at, _version
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                data["id"].as_str(),
                data["display_name"].as_str(),
                data["email"].as_str(),
                data["role"].as_str(),
                data["note"].as_str(),
                data["is_active"].as_i64(),
                data["created_at"].as_str(),
                data["updated_at"].as_str(),
                version,
            ],
        )
        .map_err(|e| AppError::Db(e.to_string()))?;

        Ok(())
    }

    fn upsert_partner(
        &self,
        tx: &rusqlite::Transaction,
        data: &serde_json::Value,
        version: i64,
    ) -> Result<(), AppError> {
        tx.execute(
            "INSERT OR REPLACE INTO partners (
                id, name, note, is_active, created_at, updated_at, _version
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                data["id"].as_str(),
                data["name"].as_str(),
                data["note"].as_str(),
                data["is_active"].as_i64(),
                data["created_at"].as_str(),
                data["updated_at"].as_str(),
                version,
            ],
        )
        .map_err(|e| AppError::Db(e.to_string()))?;

        Ok(())
    }

    fn upsert_assignment(
        &self,
        tx: &rusqlite::Transaction,
        data: &serde_json::Value,
        version: i64,
    ) -> Result<(), AppError> {
        tx.execute(
            "INSERT OR REPLACE INTO assignments (
                id, project_id, person_id, role, start_at, end_at, created_at, _version
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                data["id"].as_str(),
                data["project_id"].as_str(),
                data["person_id"].as_str(),
                data["role"].as_str(),
                data["start_at"].as_str(),
                data["end_at"].as_str(),
                data["created_at"].as_str(),
                version,
            ],
        )
        .map_err(|e| AppError::Db(e.to_string()))?;

        Ok(())
    }

    fn upsert_status_history(
        &self,
        tx: &rusqlite::Transaction,
        data: &serde_json::Value,
        version: i64,
    ) -> Result<(), AppError> {
        tx.execute(
            "INSERT OR REPLACE INTO status_history (
                id, project_id, from_status, to_status, changed_at, 
                changed_by_person_id, note, _version
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                data["id"].as_str(),
                data["project_id"].as_str(),
                data["from_status"].as_str(),
                data["to_status"].as_str(),
                data["changed_at"].as_str(),
                data["changed_by_person_id"].as_str(),
                data["note"].as_str(),
                version,
            ],
        )
        .map_err(|e| AppError::Db(e.to_string()))?;

        Ok(())
    }

    /// Apply delete operation
    fn apply_delete(
        &self,
        tx: &rusqlite::Transaction,
        table: &str,
        record_id: &str,
    ) -> Result<(), AppError> {
        let sql = format!("DELETE FROM {} WHERE id = ?1", table);
        tx.execute(&sql, params![record_id])
            .map_err(|e| AppError::Db(e.to_string()))?;

        Ok(())
    }

    /// Get vector clock for a specific record
    fn get_record_vector_clock(
        &self,
        tx: &rusqlite::Transaction,
        table: &str,
        record_id: &str,
    ) -> Result<VectorClock, AppError> {
        let mut stmt = tx
            .prepare("SELECT device_id, clock_value FROM vector_clocks WHERE table_name = ?1 AND record_id = ?2")
            .map_err(|e: rusqlite::Error| AppError::Db(e.to_string()))?;

        let clocks: std::collections::HashMap<String, i64> = stmt
            .query_map(params![table, record_id], |row: &rusqlite::Row<'_>| {
                Ok((row.get(0)?, row.get(1)?))
            })
            .map_err(|e: rusqlite::Error| AppError::Db(e.to_string()))?
            .collect::<Result<_, _>>()
            .map_err(|e: rusqlite::Error| AppError::Db(e.to_string()))?;

        Ok(VectorClock { clocks })
    }

    /// Update global vector clock after applying delta
    fn update_vector_clock(
        &self,
        tx: &rusqlite::Transaction,
        remote_vc: &VectorClock,
    ) -> Result<(), AppError> {
        for (device_id, clock_value) in &remote_vc.clocks {
            tx.execute(
                "INSERT OR REPLACE INTO vector_clocks (table_name, record_id, device_id, clock_value, updated_at)
                 VALUES ('_global', '_global', ?1, ?2, datetime('now'))",
                params![device_id, clock_value],
            )
            .map_err(|e: rusqlite::Error| AppError::Db(e.to_string()))?;
        }

        Ok(())
    }

    /// Mark local changes as synced
    pub fn mark_synced(&self, up_to_id: i64) -> Result<(), AppError> {
        let conn = self
            .pool
            .0
            .lock()
            .map_err(|e: std::sync::PoisonError<_>| AppError::Db(e.to_string()))?;

        conn.execute(
            "UPDATE sync_metadata SET synced = 1 WHERE id <= ?1 AND synced = 0",
            params![up_to_id],
        )
        .map_err(|e: rusqlite::Error| AppError::Db(e.to_string()))?;

        Ok(())
    }
}

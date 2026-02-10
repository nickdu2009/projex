//! Snapshot manager for full sync

use crate::app::export_json_string;
use crate::error::AppError;
use crate::infra::DbPool;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::io::{Read, Write};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    pub version: i32,
    pub created_at: String,
    pub device_id: String,
    pub data: String, // JSON string from export
    pub checksum: String,
}

impl Snapshot {
    /// Create a new snapshot from current database
    pub fn create(pool: &DbPool, device_id: String) -> Result<Self, AppError> {
        let data = export_json_string(pool, None)?;
        let checksum = Self::calculate_checksum(&data);
        
        Ok(Self {
            version: 1,
            created_at: chrono::Utc::now().to_rfc3339(),
            device_id,
            data,
            checksum,
        })
    }
    
    /// Calculate checksum of snapshot data
    pub fn calculate_checksum(data: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(data.as_bytes());
        format!("{:x}", hasher.finalize())
    }
    
    /// Verify snapshot integrity
    pub fn verify(&self) -> bool {
        let calculated = Self::calculate_checksum(&self.data);
        calculated == self.checksum
    }
    
    /// Compress snapshot to bytes (gzip)
    pub fn compress(&self) -> Result<Vec<u8>, AppError> {
        let json = serde_json::to_string(self)
            .map_err(|e| AppError::Db(format!("Serialize snapshot failed: {}", e)))?;
        
        let mut encoder = GzEncoder::new(Vec::new(), Compression::best());
        encoder
            .write_all(json.as_bytes())
            .map_err(|e| AppError::Db(format!("Compress failed: {}", e)))?;
        
        encoder
            .finish()
            .map_err(|e| AppError::Db(format!("Compress finish failed: {}", e)))
    }
    
    /// Decompress snapshot from bytes
    pub fn decompress(data: &[u8]) -> Result<Self, AppError> {
        let mut decoder = GzDecoder::new(data);
        let mut json = String::new();
        decoder
            .read_to_string(&mut json)
            .map_err(|e| AppError::Db(format!("Decompress failed: {}", e)))?;
        
        serde_json::from_str(&json)
            .map_err(|e| AppError::Db(format!("Deserialize snapshot failed: {}", e)))
    }
}

pub struct SnapshotManager<'a> {
    pool: &'a DbPool,
    device_id: String,
}

impl<'a> SnapshotManager<'a> {
    pub fn new(pool: &'a DbPool, device_id: String) -> Self {
        Self { pool, device_id }
    }
    
    /// Create a new snapshot
    pub fn create_snapshot(&self) -> Result<Snapshot, AppError> {
        log::info!("Creating snapshot for device {}", self.device_id);
        
        let snapshot = Snapshot::create(self.pool, self.device_id.clone())?;
        
        if !snapshot.verify() {
            return Err(AppError::Db("Snapshot verification failed".to_string()));
        }
        
        log::info!(
            "Snapshot created: {} bytes, checksum: {}",
            snapshot.data.len(),
            &snapshot.checksum[..8]
        );
        
        Ok(snapshot)
    }
    
    /// Restore from snapshot (full restore)
    pub fn restore_snapshot(&self, snapshot: &Snapshot) -> Result<(), AppError> {
        log::info!("Restoring snapshot: {}", &snapshot.checksum[..8]);
        
        // Verify snapshot integrity
        if !snapshot.verify() {
            return Err(AppError::Db("Snapshot integrity check failed".to_string()));
        }
        
        // Parse snapshot data
        let export_data: serde_json::Value = serde_json::from_str(&snapshot.data)
            .map_err(|e| AppError::Db(format!("Invalid snapshot data: {}", e)))?;
        
        // Restore to database
        let mut conn = self.pool.0.lock().map_err(|e: std::sync::PoisonError<_>| AppError::Db(e.to_string()))?;
        
        let tx = conn
            .transaction()
            .map_err(|e: rusqlite::Error| AppError::Db(e.to_string()))?;
        
        // Clear existing data
        tx.execute("DELETE FROM status_history", [])
            .map_err(|e: rusqlite::Error| AppError::Db(e.to_string()))?;
        tx.execute("DELETE FROM assignments", [])
            .map_err(|e: rusqlite::Error| AppError::Db(e.to_string()))?;
        tx.execute("DELETE FROM project_tags", [])
            .map_err(|e: rusqlite::Error| AppError::Db(e.to_string()))?;
        tx.execute("DELETE FROM projects", [])
            .map_err(|e: rusqlite::Error| AppError::Db(e.to_string()))?;
        tx.execute("DELETE FROM persons", [])
            .map_err(|e: rusqlite::Error| AppError::Db(e.to_string()))?;
        tx.execute("DELETE FROM partners", [])
            .map_err(|e: rusqlite::Error| AppError::Db(e.to_string()))?;
        
        // Restore persons
        if let Some(persons) = export_data["persons"].as_array() {
            for person in persons {
                self.restore_person(&tx, person)?;
            }
        }
        
        // Restore partners
        if let Some(partners) = export_data["partners"].as_array() {
            for partner in partners {
                self.restore_partner(&tx, partner)?;
            }
        }
        
        // Restore projects
        if let Some(projects) = export_data["projects"].as_array() {
            for project in projects {
                self.restore_project(&tx, project)?;
            }
        }
        
        // Restore assignments
        if let Some(assignments) = export_data["assignments"].as_array() {
            for assignment in assignments {
                self.restore_assignment(&tx, assignment)?;
            }
        }
        
        // Restore status_history
        if let Some(history) = export_data["statusHistory"].as_array() {
            for entry in history {
                self.restore_status_history(&tx, entry)?;
            }
        }
        
        tx.commit().map_err(|e: rusqlite::Error| AppError::Db(e.to_string()))?;
        
        log::info!("Snapshot restore completed");
        
        Ok(())
    }
    
    fn restore_person(&self, tx: &rusqlite::Transaction, data: &serde_json::Value) -> Result<(), AppError> {
        tx.execute(
            "INSERT INTO persons (id, display_name, email, role, note, is_active, created_at, updated_at, _version)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            rusqlite::params![
                data["id"].as_str(),
                data["displayName"].as_str(),
                data["email"].as_str(),
                data["role"].as_str(),
                data["note"].as_str(),
                if data["isActive"].as_bool().unwrap_or(true) { 1 } else { 0 },
                data["createdAt"].as_str(),
                data["updatedAt"].as_str(),
                data["version"].as_i64().unwrap_or(1),
            ],
        )
        .map_err(|e| AppError::Db(e.to_string()))?;
        
        Ok(())
    }
    
    fn restore_partner(&self, tx: &rusqlite::Transaction, data: &serde_json::Value) -> Result<(), AppError> {
        tx.execute(
            "INSERT INTO partners (id, name, note, is_active, created_at, updated_at, _version)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![
                data["id"].as_str(),
                data["name"].as_str(),
                data["note"].as_str(),
                if data["isActive"].as_bool().unwrap_or(true) { 1 } else { 0 },
                data["createdAt"].as_str(),
                data["updatedAt"].as_str(),
                data["version"].as_i64().unwrap_or(1),
            ],
        )
        .map_err(|e| AppError::Db(e.to_string()))?;
        
        Ok(())
    }
    
    fn restore_project(&self, tx: &rusqlite::Transaction, data: &serde_json::Value) -> Result<(), AppError> {
        tx.execute(
            "INSERT INTO projects (id, name, description, priority, current_status, country_code, 
                                   partner_id, owner_person_id, start_date, due_date, 
                                   created_at, updated_at, archived_at, _version)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
            rusqlite::params![
                data["id"].as_str(),
                data["name"].as_str(),
                data["description"].as_str(),
                data["priority"].as_i64(),
                data["currentStatus"].as_str(),
                data["countryCode"].as_str(),
                data["partnerId"].as_str(),
                data["ownerPersonId"].as_str(),
                data["startDate"].as_str(),
                data["dueDate"].as_str(),
                data["createdAt"].as_str(),
                data["updatedAt"].as_str(),
                data["archivedAt"].as_str(),
                data["version"].as_i64().unwrap_or(1),
            ],
        )
        .map_err(|e| AppError::Db(e.to_string()))?;
        
        // Restore tags
        if let Some(tags) = data["tags"].as_array() {
            for tag in tags {
                if let Some(tag_str) = tag.as_str() {
                    tx.execute(
                        "INSERT INTO project_tags (project_id, tag, created_at) VALUES (?1, ?2, ?3)",
                        rusqlite::params![data["id"].as_str(), tag_str, data["createdAt"].as_str()],
                    )
                    .map_err(|e: rusqlite::Error| AppError::Db(e.to_string()))?;
                }
            }
        }
        
        Ok(())
    }
    
    fn restore_assignment(&self, tx: &rusqlite::Transaction, data: &serde_json::Value) -> Result<(), AppError> {
        tx.execute(
            "INSERT INTO assignments (id, project_id, person_id, role, start_at, end_at, created_at, _version)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            rusqlite::params![
                data["id"].as_str(),
                data["projectId"].as_str(),
                data["personId"].as_str(),
                data["role"].as_str(),
                data["startAt"].as_str(),
                data["endAt"].as_str(),
                data["createdAt"].as_str(),
                data["version"].as_i64().unwrap_or(1),
            ],
        )
        .map_err(|e| AppError::Db(e.to_string()))?;
        
        Ok(())
    }
    
    fn restore_status_history(&self, tx: &rusqlite::Transaction, data: &serde_json::Value) -> Result<(), AppError> {
        tx.execute(
            "INSERT INTO status_history (id, project_id, from_status, to_status, changed_at, 
                                         changed_by_person_id, note, _version)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            rusqlite::params![
                data["id"].as_str(),
                data["projectId"].as_str(),
                data["fromStatus"].as_str(),
                data["toStatus"].as_str(),
                data["changedAt"].as_str(),
                data["changedByPersonId"].as_str(),
                data["note"].as_str(),
                data["version"].as_i64().unwrap_or(1),
            ],
        )
        .map_err(|e| AppError::Db(e.to_string()))?;
        
        Ok(())
    }
}

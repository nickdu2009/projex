-- Add optional product_name to projects (deliverable product name)
-- Also update sync triggers' snapshots to include the new column.

ALTER TABLE projects ADD COLUMN product_name TEXT NULL;

-- Keep an index for quick lookup / uniqueness checks in app layer.
CREATE INDEX IF NOT EXISTS idx_projects_name ON projects(name);

-- Update sync triggers for projects to include product_name in data_snapshot.
DROP TRIGGER IF EXISTS trk_projects_insert;
DROP TRIGGER IF EXISTS trk_projects_update;

CREATE TRIGGER IF NOT EXISTS trk_projects_insert
AFTER INSERT ON projects
WHEN (SELECT value FROM sync_config WHERE key = 'sync_enabled') = '1'
BEGIN
    INSERT INTO sync_metadata (table_name, record_id, operation, data_snapshot, device_id, version, created_at, synced)
    VALUES (
        'projects', NEW.id, 'INSERT',
        json_object(
            'id',NEW.id,
            'name',NEW.name,
            'description',NEW.description,
            'priority',NEW.priority,
            'current_status',NEW.current_status,
            'country_code',NEW.country_code,
            'partner_id',NEW.partner_id,
            'owner_person_id',NEW.owner_person_id,
            'product_name',NEW.product_name,
            'start_date',NEW.start_date,
            'due_date',NEW.due_date,
            'created_at',NEW.created_at,
            'updated_at',NEW.updated_at,
            'archived_at',NEW.archived_at,
            '_version',NEW._version
        ),
        (SELECT value FROM sync_config WHERE key = 'device_id'),
        NEW._version, datetime('now'), 0
    );
END;

CREATE TRIGGER IF NOT EXISTS trk_projects_update
AFTER UPDATE ON projects
WHEN (SELECT value FROM sync_config WHERE key = 'sync_enabled') = '1'
BEGIN
    INSERT INTO sync_metadata (table_name, record_id, operation, data_snapshot, device_id, version, created_at, synced)
    VALUES (
        'projects', NEW.id, 'UPDATE',
        json_object(
            'id',NEW.id,
            'name',NEW.name,
            'description',NEW.description,
            'priority',NEW.priority,
            'current_status',NEW.current_status,
            'country_code',NEW.country_code,
            'partner_id',NEW.partner_id,
            'owner_person_id',NEW.owner_person_id,
            'product_name',NEW.product_name,
            'start_date',NEW.start_date,
            'due_date',NEW.due_date,
            'created_at',NEW.created_at,
            'updated_at',NEW.updated_at,
            'archived_at',NEW.archived_at,
            '_version',NEW._version
        ),
        (SELECT value FROM sync_config WHERE key = 'device_id'),
        NEW._version, datetime('now'), 0
    );
END;


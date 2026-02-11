-- Add project comments support

-- 1. Create project_comments table
CREATE TABLE IF NOT EXISTS project_comments (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL,
    person_id TEXT,
    content TEXT NOT NULL,
    is_pinned INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    _version INTEGER DEFAULT 1,
    FOREIGN KEY(project_id) REFERENCES projects(id),
    FOREIGN KEY(person_id) REFERENCES persons(id)
);

CREATE INDEX IF NOT EXISTS idx_comments_project ON project_comments(project_id);
CREATE INDEX IF NOT EXISTS idx_comments_pinned ON project_comments(is_pinned, created_at);

-- 2. Sync triggers for project_comments

-- INSERT trigger
CREATE TRIGGER IF NOT EXISTS trk_project_comments_insert
AFTER INSERT ON project_comments
WHEN (SELECT value FROM sync_config WHERE key = 'sync_enabled') = '1'
BEGIN
    INSERT INTO sync_metadata (table_name, record_id, operation, data_snapshot, device_id, version, created_at, synced)
    VALUES (
        'project_comments', NEW.id, 'INSERT',
        json_object('id',NEW.id,'project_id',NEW.project_id,'person_id',NEW.person_id,'content',NEW.content,'is_pinned',NEW.is_pinned,'created_at',NEW.created_at,'updated_at',NEW.updated_at,'_version',NEW._version),
        (SELECT value FROM sync_config WHERE key = 'device_id'),
        NEW._version, datetime('now'), 0
    );
END;

-- UPDATE trigger
CREATE TRIGGER IF NOT EXISTS trk_project_comments_update
AFTER UPDATE ON project_comments
WHEN (SELECT value FROM sync_config WHERE key = 'sync_enabled') = '1'
BEGIN
    INSERT INTO sync_metadata (table_name, record_id, operation, data_snapshot, device_id, version, created_at, synced)
    VALUES (
        'project_comments', NEW.id, 'UPDATE',
        json_object('id',NEW.id,'project_id',NEW.project_id,'person_id',NEW.person_id,'content',NEW.content,'is_pinned',NEW.is_pinned,'created_at',NEW.created_at,'updated_at',NEW.updated_at,'_version',NEW._version),
        (SELECT value FROM sync_config WHERE key = 'device_id'),
        NEW._version, datetime('now'), 0
    );
END;

-- DELETE trigger
CREATE TRIGGER IF NOT EXISTS trk_project_comments_delete
AFTER DELETE ON project_comments
WHEN (SELECT value FROM sync_config WHERE key = 'sync_enabled') = '1'
BEGIN
    INSERT INTO sync_metadata (table_name, record_id, operation, data_snapshot, device_id, version, created_at, synced)
    VALUES (
        'project_comments', OLD.id, 'DELETE', NULL,
        (SELECT value FROM sync_config WHERE key = 'device_id'),
        OLD._version, datetime('now'), 0
    );
END;

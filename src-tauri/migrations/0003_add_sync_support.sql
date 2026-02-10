-- 添加同步支持（简化版）

-- 1. 同步元数据表
CREATE TABLE IF NOT EXISTS sync_metadata (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    table_name TEXT NOT NULL,
    record_id TEXT NOT NULL,
    operation TEXT NOT NULL,
    data_snapshot TEXT,
    device_id TEXT NOT NULL,
    version INTEGER NOT NULL,
    created_at TEXT NOT NULL,
    synced BOOLEAN DEFAULT 0
);

CREATE INDEX IF NOT EXISTS idx_sync_meta_synced ON sync_metadata(synced);
CREATE INDEX IF NOT EXISTS idx_sync_meta_version ON sync_metadata(version);
CREATE INDEX IF NOT EXISTS idx_sync_meta_table_record ON sync_metadata(table_name, record_id);

-- 2. 向量时钟表
CREATE TABLE IF NOT EXISTS vector_clocks (
    table_name TEXT NOT NULL,
    record_id TEXT NOT NULL,
    device_id TEXT NOT NULL,
    clock_value INTEGER NOT NULL DEFAULT 0,
    updated_at TEXT NOT NULL,
    PRIMARY KEY (table_name, record_id, device_id)
);

CREATE INDEX IF NOT EXISTS idx_vector_clocks_record ON vector_clocks(table_name, record_id);

-- 3. 同步配置表
CREATE TABLE IF NOT EXISTS sync_config (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

-- 初始化设备ID
INSERT OR IGNORE INTO sync_config (key, value) 
VALUES ('device_id', lower(hex(randomblob(16))));

-- 初始化本地版本号
INSERT OR IGNORE INTO sync_config (key, value) 
VALUES ('local_version', '0');

-- 初始化同步状态
INSERT OR IGNORE INTO sync_config (key, value) 
VALUES ('sync_enabled', '0');

-- 4. 为业务表添加 _version 字段
ALTER TABLE projects ADD COLUMN _version INTEGER DEFAULT 1;
ALTER TABLE persons ADD COLUMN _version INTEGER DEFAULT 1;
ALTER TABLE partners ADD COLUMN _version INTEGER DEFAULT 1;
ALTER TABLE assignments ADD COLUMN _version INTEGER DEFAULT 1;
ALTER TABLE status_history ADD COLUMN _version INTEGER DEFAULT 1;

-- 5. 触发器：persons INSERT
CREATE TRIGGER IF NOT EXISTS trk_persons_insert
AFTER INSERT ON persons
WHEN (SELECT value FROM sync_config WHERE key = 'sync_enabled') = '1'
BEGIN
    INSERT INTO sync_metadata (table_name, record_id, operation, data_snapshot, device_id, version, created_at, synced)
    VALUES (
        'persons', NEW.id, 'INSERT',
        json_object('id',NEW.id,'display_name',NEW.display_name,'email',NEW.email,'role',NEW.role,'note',NEW.note,'is_active',NEW.is_active,'created_at',NEW.created_at,'updated_at',NEW.updated_at,'_version',NEW._version),
        (SELECT value FROM sync_config WHERE key = 'device_id'),
        NEW._version, datetime('now'), 0
    );
END;

-- 6. 触发器：persons UPDATE
CREATE TRIGGER IF NOT EXISTS trk_persons_update
AFTER UPDATE ON persons
WHEN (SELECT value FROM sync_config WHERE key = 'sync_enabled') = '1'
BEGIN
    INSERT INTO sync_metadata (table_name, record_id, operation, data_snapshot, device_id, version, created_at, synced)
    VALUES (
        'persons', NEW.id, 'UPDATE',
        json_object('id',NEW.id,'display_name',NEW.display_name,'email',NEW.email,'role',NEW.role,'note',NEW.note,'is_active',NEW.is_active,'created_at',NEW.created_at,'updated_at',NEW.updated_at,'_version',NEW._version),
        (SELECT value FROM sync_config WHERE key = 'device_id'),
        NEW._version, datetime('now'), 0
    );
END;

-- 7. 触发器：partners INSERT
CREATE TRIGGER IF NOT EXISTS trk_partners_insert
AFTER INSERT ON partners
WHEN (SELECT value FROM sync_config WHERE key = 'sync_enabled') = '1'
BEGIN
    INSERT INTO sync_metadata (table_name, record_id, operation, data_snapshot, device_id, version, created_at, synced)
    VALUES (
        'partners', NEW.id, 'INSERT',
        json_object('id',NEW.id,'name',NEW.name,'note',NEW.note,'is_active',NEW.is_active,'created_at',NEW.created_at,'updated_at',NEW.updated_at,'_version',NEW._version),
        (SELECT value FROM sync_config WHERE key = 'device_id'),
        NEW._version, datetime('now'), 0
    );
END;

-- 8. 触发器：partners UPDATE
CREATE TRIGGER IF NOT EXISTS trk_partners_update
AFTER UPDATE ON partners
WHEN (SELECT value FROM sync_config WHERE key = 'sync_enabled') = '1'
BEGIN
    INSERT INTO sync_metadata (table_name, record_id, operation, data_snapshot, device_id, version, created_at, synced)
    VALUES (
        'partners', NEW.id, 'UPDATE',
        json_object('id',NEW.id,'name',NEW.name,'note',NEW.note,'is_active',NEW.is_active,'created_at',NEW.created_at,'updated_at',NEW.updated_at,'_version',NEW._version),
        (SELECT value FROM sync_config WHERE key = 'device_id'),
        NEW._version, datetime('now'), 0
    );
END;

-- 9. 触发器：projects INSERT
CREATE TRIGGER IF NOT EXISTS trk_projects_insert
AFTER INSERT ON projects
WHEN (SELECT value FROM sync_config WHERE key = 'sync_enabled') = '1'
BEGIN
    INSERT INTO sync_metadata (table_name, record_id, operation, data_snapshot, device_id, version, created_at, synced)
    VALUES (
        'projects', NEW.id, 'INSERT',
        json_object('id',NEW.id,'name',NEW.name,'description',NEW.description,'priority',NEW.priority,'current_status',NEW.current_status,'country_code',NEW.country_code,'partner_id',NEW.partner_id,'owner_person_id',NEW.owner_person_id,'start_date',NEW.start_date,'due_date',NEW.due_date,'created_at',NEW.created_at,'updated_at',NEW.updated_at,'archived_at',NEW.archived_at,'_version',NEW._version),
        (SELECT value FROM sync_config WHERE key = 'device_id'),
        NEW._version, datetime('now'), 0
    );
END;

-- 10. 触发器：projects UPDATE
CREATE TRIGGER IF NOT EXISTS trk_projects_update
AFTER UPDATE ON projects
WHEN (SELECT value FROM sync_config WHERE key = 'sync_enabled') = '1'
BEGIN
    INSERT INTO sync_metadata (table_name, record_id, operation, data_snapshot, device_id, version, created_at, synced)
    VALUES (
        'projects', NEW.id, 'UPDATE',
        json_object('id',NEW.id,'name',NEW.name,'description',NEW.description,'priority',NEW.priority,'current_status',NEW.current_status,'country_code',NEW.country_code,'partner_id',NEW.partner_id,'owner_person_id',NEW.owner_person_id,'start_date',NEW.start_date,'due_date',NEW.due_date,'created_at',NEW.created_at,'updated_at',NEW.updated_at,'archived_at',NEW.archived_at,'_version',NEW._version),
        (SELECT value FROM sync_config WHERE key = 'device_id'),
        NEW._version, datetime('now'), 0
    );
END;

-- 11. 触发器：persons DELETE
CREATE TRIGGER IF NOT EXISTS trk_persons_delete
AFTER DELETE ON persons
WHEN (SELECT value FROM sync_config WHERE key = 'sync_enabled') = '1'
BEGIN
    INSERT INTO sync_metadata (table_name, record_id, operation, data_snapshot, device_id, version, created_at, synced)
    VALUES (
        'persons', OLD.id, 'DELETE', NULL,
        (SELECT value FROM sync_config WHERE key = 'device_id'),
        OLD._version, datetime('now'), 0
    );
END;

-- 12. 触发器：partners DELETE
CREATE TRIGGER IF NOT EXISTS trk_partners_delete
AFTER DELETE ON partners
WHEN (SELECT value FROM sync_config WHERE key = 'sync_enabled') = '1'
BEGIN
    INSERT INTO sync_metadata (table_name, record_id, operation, data_snapshot, device_id, version, created_at, synced)
    VALUES (
        'partners', OLD.id, 'DELETE', NULL,
        (SELECT value FROM sync_config WHERE key = 'device_id'),
        OLD._version, datetime('now'), 0
    );
END;

-- 13. 触发器：projects DELETE
CREATE TRIGGER IF NOT EXISTS trk_projects_delete
AFTER DELETE ON projects
WHEN (SELECT value FROM sync_config WHERE key = 'sync_enabled') = '1'
BEGIN
    INSERT INTO sync_metadata (table_name, record_id, operation, data_snapshot, device_id, version, created_at, synced)
    VALUES (
        'projects', OLD.id, 'DELETE', NULL,
        (SELECT value FROM sync_config WHERE key = 'device_id'),
        OLD._version, datetime('now'), 0
    );
END;

-- 14. 触发器：assignments INSERT
CREATE TRIGGER IF NOT EXISTS trk_assignments_insert
AFTER INSERT ON assignments
WHEN (SELECT value FROM sync_config WHERE key = 'sync_enabled') = '1'
BEGIN
    INSERT INTO sync_metadata (table_name, record_id, operation, data_snapshot, device_id, version, created_at, synced)
    VALUES (
        'assignments', NEW.id, 'INSERT',
        json_object('id',NEW.id,'project_id',NEW.project_id,'person_id',NEW.person_id,'role',NEW.role,'start_at',NEW.start_at,'end_at',NEW.end_at,'created_at',NEW.created_at,'_version',NEW._version),
        (SELECT value FROM sync_config WHERE key = 'device_id'),
        NEW._version, datetime('now'), 0
    );
END;

-- 15. 触发器：assignments UPDATE
CREATE TRIGGER IF NOT EXISTS trk_assignments_update
AFTER UPDATE ON assignments
WHEN (SELECT value FROM sync_config WHERE key = 'sync_enabled') = '1'
BEGIN
    INSERT INTO sync_metadata (table_name, record_id, operation, data_snapshot, device_id, version, created_at, synced)
    VALUES (
        'assignments', NEW.id, 'UPDATE',
        json_object('id',NEW.id,'project_id',NEW.project_id,'person_id',NEW.person_id,'role',NEW.role,'start_at',NEW.start_at,'end_at',NEW.end_at,'created_at',NEW.created_at,'_version',NEW._version),
        (SELECT value FROM sync_config WHERE key = 'device_id'),
        NEW._version, datetime('now'), 0
    );
END;

-- 16. 触发器：assignments DELETE
CREATE TRIGGER IF NOT EXISTS trk_assignments_delete
AFTER DELETE ON assignments
WHEN (SELECT value FROM sync_config WHERE key = 'sync_enabled') = '1'
BEGIN
    INSERT INTO sync_metadata (table_name, record_id, operation, data_snapshot, device_id, version, created_at, synced)
    VALUES (
        'assignments', OLD.id, 'DELETE', NULL,
        (SELECT value FROM sync_config WHERE key = 'device_id'),
        OLD._version, datetime('now'), 0
    );
END;

-- 17. 触发器：status_history INSERT
CREATE TRIGGER IF NOT EXISTS trk_status_history_insert
AFTER INSERT ON status_history
WHEN (SELECT value FROM sync_config WHERE key = 'sync_enabled') = '1'
BEGIN
    INSERT INTO sync_metadata (table_name, record_id, operation, data_snapshot, device_id, version, created_at, synced)
    VALUES (
        'status_history', NEW.id, 'INSERT',
        json_object('id',NEW.id,'project_id',NEW.project_id,'from_status',NEW.from_status,'to_status',NEW.to_status,'changed_at',NEW.changed_at,'changed_by_person_id',NEW.changed_by_person_id,'note',NEW.note,'_version',NEW._version),
        (SELECT value FROM sync_config WHERE key = 'device_id'),
        NEW._version, datetime('now'), 0
    );
END;

-- 18. 触发器：status_history DELETE
CREATE TRIGGER IF NOT EXISTS trk_status_history_delete
AFTER DELETE ON status_history
WHEN (SELECT value FROM sync_config WHERE key = 'sync_enabled') = '1'
BEGIN
    INSERT INTO sync_metadata (table_name, record_id, operation, data_snapshot, device_id, version, created_at, synced)
    VALUES (
        'status_history', OLD.id, 'DELETE', NULL,
        (SELECT value FROM sync_config WHERE key = 'device_id'),
        OLD._version, datetime('now'), 0
    );
END;

-- 19. 触发器：project_tags INSERT
CREATE TRIGGER IF NOT EXISTS trk_project_tags_insert
AFTER INSERT ON project_tags
WHEN (SELECT value FROM sync_config WHERE key = 'sync_enabled') = '1'
BEGIN
    INSERT INTO sync_metadata (table_name, record_id, operation, data_snapshot, device_id, version, created_at, synced)
    VALUES (
        'project_tags', NEW.project_id || ':' || NEW.tag, 'INSERT',
        json_object('project_id',NEW.project_id,'tag',NEW.tag,'created_at',NEW.created_at),
        (SELECT value FROM sync_config WHERE key = 'device_id'),
        1, datetime('now'), 0
    );
END;

-- 20. 触发器：project_tags DELETE
CREATE TRIGGER IF NOT EXISTS trk_project_tags_delete
AFTER DELETE ON project_tags
WHEN (SELECT value FROM sync_config WHERE key = 'sync_enabled') = '1'
BEGIN
    INSERT INTO sync_metadata (table_name, record_id, operation, data_snapshot, device_id, version, created_at, synced)
    VALUES (
        'project_tags', OLD.project_id || ':' || OLD.tag, 'DELETE', NULL,
        (SELECT value FROM sync_config WHERE key = 'device_id'),
        1, datetime('now'), 0
    );
END;

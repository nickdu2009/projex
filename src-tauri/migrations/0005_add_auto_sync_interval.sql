-- Add default auto sync interval (minutes)
-- Note: sync_config is a simple key-value table; this is a data seed migration.

INSERT OR IGNORE INTO sync_config (key, value)
VALUES ('auto_sync_interval_minutes', '1');


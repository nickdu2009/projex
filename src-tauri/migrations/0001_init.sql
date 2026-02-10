-- schema_migrations: track applied migrations
CREATE TABLE IF NOT EXISTS schema_migrations (
  version INTEGER PRIMARY KEY,
  applied_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE persons (
  id TEXT PRIMARY KEY,
  display_name TEXT NOT NULL,
  note TEXT NOT NULL DEFAULT '',
  is_active INTEGER NOT NULL DEFAULT 1,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE partners (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  note TEXT NOT NULL DEFAULT '',
  is_active INTEGER NOT NULL DEFAULT 1,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);
CREATE UNIQUE INDEX idx_partners_name ON partners(name);

CREATE TABLE projects (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  description TEXT NOT NULL DEFAULT '',
  priority INTEGER NOT NULL DEFAULT 3,
  current_status TEXT NOT NULL,
  country_code TEXT NOT NULL,
  partner_id TEXT NOT NULL,
  owner_person_id TEXT NOT NULL,
  start_date TEXT NULL,
  due_date TEXT NULL,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  archived_at TEXT NULL,
  FOREIGN KEY(partner_id) REFERENCES partners(id),
  FOREIGN KEY(owner_person_id) REFERENCES persons(id)
);
CREATE INDEX idx_projects_status ON projects(current_status);
CREATE INDEX idx_projects_partner ON projects(partner_id);
CREATE INDEX idx_projects_country ON projects(country_code);

CREATE TABLE assignments (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL,
  person_id TEXT NOT NULL,
  role TEXT NOT NULL DEFAULT 'member',
  start_at TEXT NOT NULL,
  end_at TEXT NULL,
  created_at TEXT NOT NULL,
  FOREIGN KEY(project_id) REFERENCES projects(id),
  FOREIGN KEY(person_id) REFERENCES persons(id)
);
CREATE INDEX idx_assignments_person ON assignments(person_id, end_at);
CREATE INDEX idx_assignments_project ON assignments(project_id, end_at);

CREATE UNIQUE INDEX uniq_assignment_active ON assignments(project_id, person_id) WHERE end_at IS NULL;

CREATE TABLE status_history (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL,
  from_status TEXT NULL,
  to_status TEXT NOT NULL,
  changed_at TEXT NOT NULL,
  changed_by_person_id TEXT NULL,
  note TEXT NOT NULL DEFAULT '',
  FOREIGN KEY(project_id) REFERENCES projects(id),
  FOREIGN KEY(changed_by_person_id) REFERENCES persons(id)
);
CREATE INDEX idx_status_history_project_time ON status_history(project_id, changed_at DESC);

CREATE TABLE project_tags (
  project_id TEXT NOT NULL,
  tag TEXT NOT NULL,
  created_at TEXT NOT NULL,
  PRIMARY KEY(project_id, tag),
  FOREIGN KEY(project_id) REFERENCES projects(id)
);
CREATE INDEX idx_project_tags_tag ON project_tags(tag);

INSERT INTO schema_migrations (version, applied_at) VALUES (1, datetime('now'));

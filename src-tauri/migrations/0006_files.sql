CREATE TABLE registered_files (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE RESTRICT,
  file_name TEXT NOT NULL,
  file_path TEXT NOT NULL COLLATE NOCASE,
  operation TEXT NOT NULL CHECK (operation IN ('register','copy','move')),
  discipline TEXT,
  drawing_number TEXT,
  title TEXT,
  revision TEXT,
  revision_date TEXT,
  received_date TEXT,
  is_current INTEGER NOT NULL DEFAULT 1 CHECK (is_current IN (0,1)),
  created_at_utc TEXT NOT NULL,
  updated_at_utc TEXT NOT NULL,
  UNIQUE(project_id, file_path)
);
CREATE INDEX registered_files_project_idx ON registered_files(project_id, updated_at_utc DESC);

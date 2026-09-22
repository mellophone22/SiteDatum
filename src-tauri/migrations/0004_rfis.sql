CREATE TABLE rfis (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE RESTRICT,
  number TEXT NOT NULL COLLATE NOCASE,
  subject TEXT NOT NULL,
  question TEXT NOT NULL,
  recipient TEXT,
  status TEXT NOT NULL DEFAULT 'draft' CHECK (status IN ('draft', 'open', 'response_received', 'closed')),
  created_date TEXT NOT NULL,
  submitted_date TEXT,
  response_due_date TEXT,
  response_received_date TEXT,
  response TEXT,
  notes TEXT,
  created_at_utc TEXT NOT NULL,
  updated_at_utc TEXT NOT NULL,
  UNIQUE(project_id, number)
);

CREATE TABLE rfi_task_relationships (
  rfi_id TEXT NOT NULL REFERENCES rfis(id) ON DELETE CASCADE,
  task_id TEXT NOT NULL REFERENCES tasks(id) ON DELETE RESTRICT,
  created_at_utc TEXT NOT NULL,
  PRIMARY KEY (rfi_id, task_id)
);

CREATE TABLE rfi_attachment_references (
  id TEXT PRIMARY KEY,
  rfi_id TEXT NOT NULL REFERENCES rfis(id) ON DELETE CASCADE,
  file_path TEXT NOT NULL,
  display_name TEXT NOT NULL,
  created_at_utc TEXT NOT NULL,
  UNIQUE(rfi_id, file_path)
);

CREATE INDEX rfis_project_status_idx ON rfis (project_id, status, response_due_date);
CREATE INDEX rfis_attention_idx ON rfis (status, response_due_date);

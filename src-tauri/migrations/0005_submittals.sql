CREATE TABLE submittals (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE RESTRICT,
  parent_submittal_id TEXT REFERENCES submittals(id) ON DELETE RESTRICT,
  number TEXT NOT NULL COLLATE NOCASE,
  name TEXT NOT NULL,
  package TEXT,
  revision TEXT NOT NULL DEFAULT '',
  recipient TEXT,
  status TEXT NOT NULL DEFAULT 'draft' CHECK (status IN ('draft', 'preparing', 'submitted', 'under_review', 'approved', 'approved_as_noted', 'revise_and_resubmit', 'rejected', 'closed')),
  disposition TEXT CHECK (disposition IN ('approved', 'approved_as_noted', 'revise_and_resubmit', 'rejected')),
  created_date TEXT NOT NULL,
  submitted_date TEXT,
  response_date TEXT,
  resubmission_required INTEGER NOT NULL DEFAULT 0 CHECK (resubmission_required IN (0, 1)),
  notes TEXT,
  created_at_utc TEXT NOT NULL,
  updated_at_utc TEXT NOT NULL,
  UNIQUE(project_id, number, revision),
  CHECK ((status = 'closed' AND disposition IS NOT NULL) OR status <> 'closed'),
  CHECK ((resubmission_required = 0) OR disposition = 'revise_and_resubmit')
);

CREATE TABLE submittal_task_relationships (
  submittal_id TEXT NOT NULL REFERENCES submittals(id) ON DELETE CASCADE,
  task_id TEXT NOT NULL REFERENCES tasks(id) ON DELETE RESTRICT,
  created_at_utc TEXT NOT NULL,
  PRIMARY KEY (submittal_id, task_id)
);

CREATE TABLE submittal_attachment_references (
  id TEXT PRIMARY KEY,
  submittal_id TEXT NOT NULL REFERENCES submittals(id) ON DELETE CASCADE,
  file_path TEXT NOT NULL,
  display_name TEXT NOT NULL,
  created_at_utc TEXT NOT NULL,
  UNIQUE(submittal_id, file_path)
);

CREATE INDEX submittals_project_status_idx ON submittals (project_id, status, submitted_date);
CREATE INDEX submittals_attention_idx ON submittals (status, submitted_date);

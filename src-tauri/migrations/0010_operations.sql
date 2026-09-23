CREATE TABLE work_items (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE RESTRICT,
  item_type TEXT NOT NULL CHECK (item_type IN ('meeting_minute','procurement','change_event','transmittal','milestone','punch_item','daily_report','startup_check','commissioning_check','commissioning_issue')),
  number TEXT,
  title TEXT NOT NULL,
  description TEXT,
  status TEXT NOT NULL DEFAULT 'open' CHECK (status IN ('draft','open','in_progress','waiting','completed','cancelled')),
  priority TEXT NOT NULL DEFAULT 'medium' CHECK (priority IN ('low','medium','high','urgent')),
  due_date TEXT,
  occurred_date TEXT,
  responsible_party TEXT,
  company TEXT,
  location TEXT,
  amount_cents INTEGER,
  checklist_total INTEGER NOT NULL DEFAULT 0 CHECK (checklist_total >= 0),
  checklist_completed INTEGER NOT NULL DEFAULT 0 CHECK (checklist_completed >= 0 AND checklist_completed <= checklist_total),
  archived_at_utc TEXT,
  created_at_utc TEXT NOT NULL,
  updated_at_utc TEXT NOT NULL
);
CREATE INDEX work_items_project_type_idx ON work_items(project_id,item_type,status,due_date);
CREATE INDEX work_items_due_idx ON work_items(status,due_date);

CREATE TABLE project_templates (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL COLLATE NOCASE UNIQUE,
  task_titles_json TEXT NOT NULL DEFAULT '[]',
  milestone_titles_json TEXT NOT NULL DEFAULT '[]',
  created_at_utc TEXT NOT NULL,
  updated_at_utc TEXT NOT NULL
);

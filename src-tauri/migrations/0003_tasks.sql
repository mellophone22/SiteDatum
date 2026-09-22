CREATE TABLE tasks (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE RESTRICT,
  title TEXT NOT NULL,
  description TEXT,
  priority TEXT NOT NULL DEFAULT 'medium' CHECK (priority IN ('low','medium','high','urgent')),
  status TEXT NOT NULL DEFAULT 'open' CHECK (status IN ('open','in_progress','waiting','blocked','completed','cancelled')),
  category TEXT,
  due_date TEXT,
  follow_up_date TEXT,
  waiting_since_utc TEXT,
  waiting_on TEXT,
  related_contact_id TEXT,
  created_at_utc TEXT NOT NULL,
  updated_at_utc TEXT NOT NULL,
  CHECK ((status = 'waiting' AND waiting_on IS NOT NULL AND length(trim(waiting_on)) > 0 AND waiting_since_utc IS NOT NULL) OR status <> 'waiting')
);
CREATE INDEX tasks_project_idx ON tasks (project_id, status, due_date);
CREATE INDEX tasks_attention_idx ON tasks (status, due_date, follow_up_date);

CREATE TABLE projects (
  id TEXT PRIMARY KEY,
  number TEXT NOT NULL COLLATE NOCASE UNIQUE,
  name TEXT NOT NULL,
  status TEXT NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'on_hold', 'completed', 'cancelled')),
  phase TEXT NOT NULL DEFAULT 'engineering' CHECK (phase IN ('preconstruction', 'engineering', 'submittals', 'procurement', 'construction', 'programming', 'startup', 'commissioning', 'closeout', 'custom')),
  custom_phase_name TEXT,
  customer TEXT,
  general_contractor TEXT,
  engineer TEXT,
  project_manager TEXT,
  superintendent TEXT,
  location TEXT,
  start_date TEXT,
  target_date TEXT,
  description TEXT,
  important_notes TEXT,
  project_path TEXT NOT NULL,
  is_pinned INTEGER NOT NULL DEFAULT 0 CHECK (is_pinned IN (0, 1)),
  archived_at_utc TEXT,
  created_at_utc TEXT NOT NULL,
  updated_at_utc TEXT NOT NULL,
  CHECK ((phase = 'custom' AND custom_phase_name IS NOT NULL AND length(trim(custom_phase_name)) > 0) OR (phase <> 'custom' AND custom_phase_name IS NULL))
);

CREATE INDEX projects_active_list_idx ON projects (archived_at_utc, is_pinned DESC, updated_at_utc DESC);
CREATE INDEX projects_status_idx ON projects (status);
CREATE INDEX projects_phase_idx ON projects (phase);

CREATE TABLE notes (id TEXT PRIMARY KEY, project_id TEXT REFERENCES projects(id) ON DELETE RESTRICT, body TEXT NOT NULL, created_at_utc TEXT NOT NULL, updated_at_utc TEXT NOT NULL);
CREATE TABLE contacts (id TEXT PRIMARY KEY, name TEXT NOT NULL, company TEXT, email TEXT, phone TEXT, role TEXT, created_at_utc TEXT NOT NULL, updated_at_utc TEXT NOT NULL);
CREATE INDEX notes_project_idx ON notes(project_id, updated_at_utc DESC);
CREATE INDEX contacts_name_idx ON contacts(name COLLATE NOCASE);

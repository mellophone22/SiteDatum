CREATE TABLE sync_local_records (
  entity_type TEXT NOT NULL,
  entity_id TEXT NOT NULL,
  content_hash TEXT NOT NULL,
  cloud_version INTEGER NOT NULL DEFAULT 0,
  PRIMARY KEY (entity_type, entity_id)
);

CREATE TABLE sync_conflicts (
  id TEXT PRIMARY KEY,
  entity_type TEXT NOT NULL,
  entity_id TEXT NOT NULL,
  local_payload_json TEXT NOT NULL,
  cloud_payload_json TEXT NOT NULL,
  cloud_version INTEGER NOT NULL,
  created_at_utc TEXT NOT NULL,
  resolved_at_utc TEXT
);

CREATE INDEX sync_conflicts_open_idx ON sync_conflicts (resolved_at_utc, created_at_utc DESC);

-- C10-04A local-only anti-rollback state. This table is deliberately excluded
-- from the replicated record set and stores no project content or key material.
CREATE TABLE sync_v2_checkpoint_anchors (
  workspace_id TEXT PRIMARY KEY,
  protocol_version INTEGER NOT NULL CHECK (protocol_version = 1),
  checkpoint_counter INTEGER NOT NULL CHECK (checkpoint_counter >= 1),
  checkpoint_digest BLOB NOT NULL CHECK (length(checkpoint_digest) = 32),
  through_change_seq INTEGER NOT NULL CHECK (through_change_seq >= 1),
  updated_at_utc TEXT NOT NULL
);

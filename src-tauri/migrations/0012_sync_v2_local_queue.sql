-- Dormant local protocol state. No document bytes, plaintext content, or keys.
CREATE TABLE sync_v2_local_streams (
    workspace_id TEXT PRIMARY KEY,
    owner_id TEXT NOT NULL,
    device_id TEXT NOT NULL,
    pull_cursor INTEGER NOT NULL DEFAULT 0 CHECK (pull_cursor >= 0)
);
CREATE TABLE sync_v2_record_snapshots (
    workspace_id TEXT NOT NULL REFERENCES sync_v2_local_streams(workspace_id),
    record_id TEXT NOT NULL,
    record_kind INTEGER NOT NULL CHECK (record_kind BETWEEN 1 AND 13),
    server_version INTEGER NOT NULL CHECK (server_version >= 0),
    envelope BLOB NOT NULL,
    PRIMARY KEY (workspace_id, record_id)
);
CREATE TABLE sync_v2_outbox (
    sequence INTEGER PRIMARY KEY AUTOINCREMENT,
    workspace_id TEXT NOT NULL REFERENCES sync_v2_local_streams(workspace_id),
    mutation_id TEXT NOT NULL UNIQUE,
    record_id TEXT NOT NULL,
    envelope BLOB NOT NULL,
    UNIQUE (workspace_id, record_id)
);
CREATE TABLE sync_v2_device_acknowledgements (
    workspace_id TEXT PRIMARY KEY REFERENCES sync_v2_local_streams(workspace_id),
    device_id TEXT NOT NULL,
    checkpoint_counter INTEGER NOT NULL CHECK (checkpoint_counter > 0),
    checkpoint_digest BLOB NOT NULL CHECK (length(checkpoint_digest) = 32),
    through_change_seq INTEGER NOT NULL CHECK (through_change_seq > 0)
);

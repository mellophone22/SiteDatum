-- Dormant local protocol state; envelopes only, never keys or document bytes.
CREATE TABLE sync_v2_committed_bases (
 workspace_id TEXT NOT NULL REFERENCES sync_v2_local_streams(workspace_id),
 record_id TEXT NOT NULL, server_version INTEGER NOT NULL,
 envelope BLOB, PRIMARY KEY(workspace_id,record_id)
);
CREATE TABLE sync_v2_pull_history (
 workspace_id TEXT NOT NULL REFERENCES sync_v2_local_streams(workspace_id),
 change_seq INTEGER NOT NULL, server_version INTEGER NOT NULL, envelope BLOB NOT NULL,
 PRIMARY KEY(workspace_id,change_seq)
);
CREATE TABLE sync_v2_pull_batches (
 workspace_id TEXT PRIMARY KEY REFERENCES sync_v2_local_streams(workspace_id),
 start_after INTEGER NOT NULL, checkpoint BLOB NOT NULL
);
CREATE TABLE sync_v2_pull_parts (
 workspace_id TEXT NOT NULL REFERENCES sync_v2_pull_batches(workspace_id),
 change_seq INTEGER NOT NULL, server_version INTEGER NOT NULL, envelope BLOB NOT NULL,
 PRIMARY KEY(workspace_id,change_seq)
);
CREATE TABLE sync_v2_record_conflicts (
 workspace_id TEXT NOT NULL REFERENCES sync_v2_local_streams(workspace_id),
 record_id TEXT NOT NULL, checkpoint_counter INTEGER NOT NULL,
 local_envelope BLOB NOT NULL, remote_envelope BLOB NOT NULL, baseline_envelope BLOB,
 pending_envelope BLOB, resolved_counter INTEGER,
 choice TEXT CHECK(choice IN ('keep_local','accept_remote')), resolution_envelope BLOB,
 PRIMARY KEY(workspace_id,record_id,checkpoint_counter)
);
CREATE TABLE sync_v2_applied_receipts (
 workspace_id TEXT PRIMARY KEY REFERENCES sync_v2_local_streams(workspace_id),
 device_id TEXT NOT NULL, checkpoint_counter INTEGER NOT NULL,
 checkpoint_digest BLOB NOT NULL CHECK(length(checkpoint_digest)=32),
 through_change_seq INTEGER NOT NULL, backup_id TEXT NOT NULL
);
CREATE TABLE sync_v2_record_subtypes (
 workspace_id TEXT NOT NULL REFERENCES sync_v2_local_streams(workspace_id),
 record_id TEXT NOT NULL, subtype TEXT NOT NULL CHECK(subtype IN ('work_item','project_template')),
 PRIMARY KEY(workspace_id,record_id)
);
CREATE TABLE sync_v2_activity_bindings (
 workspace_id TEXT NOT NULL REFERENCES sync_v2_local_streams(workspace_id),
 record_id TEXT NOT NULL UNIQUE, local_id TEXT NOT NULL UNIQUE,
 PRIMARY KEY(workspace_id,record_id)
);

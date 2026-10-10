-- Local opaque envelope identity for composite-key links; retained on deletion.
CREATE TABLE sync_v2_relationship_bindings (
    workspace_id TEXT NOT NULL REFERENCES sync_v2_local_streams(workspace_id),
    record_id TEXT NOT NULL,
    record_kind INTEGER NOT NULL CHECK (record_kind IN (5,6)),
    register_id TEXT NOT NULL,
    task_id TEXT NOT NULL,
    PRIMARY KEY (workspace_id,record_id),
    UNIQUE (record_kind,register_id,task_id)
);

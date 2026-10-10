-- Ciphertext-only retry journal. No keys, code, file paths or project content.
CREATE TABLE sync_v2_rotation_batches (
 workspace_id TEXT PRIMARY KEY REFERENCES sync_v2_local_streams(workspace_id),
 key_version INTEGER NOT NULL CHECK(key_version>0),
 checkpoint_counter INTEGER NOT NULL CHECK(checkpoint_counter>0),
 through_change_seq INTEGER NOT NULL CHECK(through_change_seq>0),
 checkpoint BLOB NOT NULL
);

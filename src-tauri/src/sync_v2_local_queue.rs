//! Encrypted staging only. Project-table apply, backup and conflicts are C10-04C.
#![allow(dead_code)]

use crate::{
    error::{AppError, AppResult},
    persistence::Database,
    sync_v2_record_codec::{
        invalid, open_record, validate_header, RecordHeader, SealedRecord, MAX_SAFE_INTEGER,
    },
    sync_v2_record_protocol::{
        accept_checkpoint_in_transaction, open_workspace_checkpoint, RecordManifestEntry,
        SealedWorkspaceCheckpoint,
    },
};
use rusqlite::{params, OptionalExtension, Transaction};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Copy)]
pub(crate) struct StreamScope {
    pub owner_id: Uuid,
    pub workspace_id: Uuid,
    pub device_id: Uuid,
}

#[derive(Clone)]
pub(crate) struct PulledRecord {
    pub change_seq: u64,
    pub server_version: u64,
    pub record: SealedRecord,
}

#[derive(Clone)]
pub(crate) struct MutationReceipt {
    pub record_id: Uuid,
    pub mutation_id: Uuid,
    pub server_version: u64,
}

// This is LOCAL storage encoding, not a provider request or a plaintext record.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredEnvelope {
    owner: String,
    workspace: String,
    record: String,
    mutation: String,
    kind: u8,
    expected: u64,
    key_version: u32,
    protocol: u16,
    tombstone: bool,
    ciphertext: Vec<u8>,
}
pub(crate) fn encode(record: &SealedRecord) -> AppResult<Vec<u8>> {
    validate_header(&record.header)?;
    let h = &record.header;
    serde_json::to_vec(&StoredEnvelope {
        owner: h.owner_id.to_string(),
        workspace: h.workspace_id.to_string(),
        record: h.record_id.to_string(),
        mutation: h.mutation_id.to_string(),
        kind: h.record_kind,
        expected: h.expected_server_version,
        key_version: h.workspace_key_version,
        protocol: h.protocol_version,
        tombstone: h.tombstone,
        ciphertext: record.ciphertext.clone(),
    })
    .map_err(|_| invalid())
}
pub(crate) fn decode(bytes: &[u8]) -> AppResult<SealedRecord> {
    let s: StoredEnvelope = serde_json::from_slice(bytes).map_err(|_| invalid())?;
    let id = |text: &str| Uuid::parse_str(text).map_err(|_| invalid());
    let record = SealedRecord {
        header: RecordHeader {
            owner_id: id(&s.owner)?,
            workspace_id: id(&s.workspace)?,
            record_id: id(&s.record)?,
            mutation_id: id(&s.mutation)?,
            record_kind: s.kind,
            expected_server_version: s.expected,
            workspace_key_version: s.key_version,
            protocol_version: s.protocol,
            tombstone: s.tombstone,
        },
        ciphertext: s.ciphertext,
    };
    validate_header(&record.header)?;
    Ok(record)
}
fn storage(error: rusqlite::Error) -> AppError {
    // Never include SQL values or provider/record content in error detail.
    let _ = error;
    AppError::from_technical(
        "SYNC_V2_QUEUE_STORAGE_FAILED",
        "The local Sync queue could not be updated.",
        "Keep the local workspace and retry.",
        "Local queue transaction failed.",
    )
}
fn blocked() -> AppError {
    AppError::from_technical(
        "SYNC_V2_QUEUE_CONFLICT",
        "Sync staging needs review before it can advance.",
        "Keep both computers' local work and review Sync status.",
        "Scope, revision, pending work, receipt, or cursor did not match.",
    )
}
pub(crate) fn scope(transaction: &Transaction<'_>, s: StreamScope) -> AppResult<()> {
    if [s.owner_id, s.workspace_id, s.device_id]
        .iter()
        .any(Uuid::is_nil)
    {
        return Err(invalid());
    }
    transaction.execute("INSERT INTO sync_v2_local_streams(workspace_id,owner_id,device_id) VALUES(?1,?2,?3) ON CONFLICT(workspace_id) DO NOTHING", params![s.workspace_id.to_string(), s.owner_id.to_string(), s.device_id.to_string()]).map_err(storage)?;
    let existing: (String, String) = transaction
        .query_row(
            "SELECT owner_id,device_id FROM sync_v2_local_streams WHERE workspace_id=?1",
            [s.workspace_id.to_string()],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(storage)?;
    if existing != (s.owner_id.to_string(), s.device_id.to_string()) {
        return Err(blocked());
    }
    Ok(())
}
fn matches_scope(s: StreamScope, record: &SealedRecord) -> AppResult<()> {
    if record.header.owner_id != s.owner_id || record.header.workspace_id != s.workspace_id {
        return Err(blocked());
    }
    Ok(())
}

impl Database {
    /// Persist the encrypted local candidate and retry queue in ONE transaction.
    /// A second edit to a pending record is refused until conflict/coalescing exists.
    pub(crate) fn stage_sync_v2_mutation(
        &mut self,
        s: StreamScope,
        record: &SealedRecord,
        key: &impl crate::sync_v2_key_recovery::WorkspaceKeys,
    ) -> AppResult<()> {
        self.stage_sync_v2_mutation_with_apply(s, record, key, |_| Ok(()))
    }
    pub(crate) fn stage_sync_v2_mutation_with_apply(
        &mut self,
        s: StreamScope,
        record: &SealedRecord,
        key: &impl crate::sync_v2_key_recovery::WorkspaceKeys,
        apply: impl FnOnce(&Transaction<'_>) -> AppResult<()>,
    ) -> AppResult<()> {
        matches_scope(s, record)?;
        open_record(record, key)?;
        let bytes = encode(record)?;
        let h = &record.header;
        if key
            .write_version()
            .is_some_and(|active| active != h.workspace_key_version)
        {
            return Err(blocked());
        }
        let tx = self.connection.transaction().map_err(storage)?;
        scope(&tx, s)?;
        let rotating: bool = tx
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM sync_v2_rotation_batches WHERE workspace_id=?1)",
                [s.workspace_id.to_string()],
                |r| r.get(0),
            )
            .map_err(storage)?;
        if rotating {
            return Err(blocked());
        }
        let queued: Option<Vec<u8>> = tx
            .query_row(
                "SELECT envelope FROM sync_v2_outbox WHERE workspace_id=?1 AND record_id=?2",
                params![s.workspace_id.to_string(), h.record_id.to_string()],
                |row| row.get(0),
            )
            .optional()
            .map_err(storage)?;
        if let Some(queued) = queued {
            if queued != bytes {
                return Err(blocked());
            }
            apply(&tx)?;
            return tx.commit().map_err(storage);
        }
        let existing: Option<(u64,u8)> = tx.query_row("SELECT server_version,record_kind FROM sync_v2_record_snapshots WHERE workspace_id=?1 AND record_id=?2", params![s.workspace_id.to_string(),h.record_id.to_string()], |row| Ok((row.get::<_, i64>(0)? as u64,row.get(1)?))).optional().map_err(storage)?;
        if existing
            .map(|(version, kind)| version != h.expected_server_version || kind != h.record_kind)
            .unwrap_or(h.expected_server_version != 0)
        {
            return Err(blocked());
        }
        tx.execute("INSERT INTO sync_v2_committed_bases(workspace_id,record_id,server_version,envelope) SELECT ?1,?2,COALESCE((SELECT server_version FROM sync_v2_record_snapshots WHERE workspace_id=?1 AND record_id=?2),0),(SELECT envelope FROM sync_v2_record_snapshots WHERE workspace_id=?1 AND record_id=?2)",params![s.workspace_id.to_string(),h.record_id.to_string()]).map_err(storage)?;
        tx.execute("INSERT INTO sync_v2_outbox(workspace_id,mutation_id,record_id,envelope) VALUES(?1,?2,?3,?4)", params![s.workspace_id.to_string(),h.mutation_id.to_string(),h.record_id.to_string(),&bytes]).map_err(storage)?;
        tx.execute("INSERT INTO sync_v2_record_snapshots(workspace_id,record_id,record_kind,server_version,envelope) VALUES(?1,?2,?3,?4,?5) ON CONFLICT(workspace_id,record_id) DO UPDATE SET envelope=excluded.envelope", params![s.workspace_id.to_string(),h.record_id.to_string(),h.record_kind,h.expected_server_version as i64,&bytes]).map_err(storage)?;
        tx.execute(
            "DELETE FROM sync_v2_applied_receipts WHERE workspace_id=?1",
            [s.workspace_id.to_string()],
        )
        .map_err(storage)?;
        apply(&tx)?;
        tx.commit().map_err(storage)
    }

    pub(crate) fn pending_sync_v2_mutations(&self, s: StreamScope) -> AppResult<Vec<SealedRecord>> {
        let mut statement = self.connection.prepare("SELECT q.envelope FROM sync_v2_outbox q JOIN sync_v2_local_streams s USING(workspace_id) WHERE q.workspace_id=?1 AND s.owner_id=?2 AND s.device_id=?3 ORDER BY q.sequence").map_err(storage)?;
        let rows = statement
            .query_map(
                params![
                    s.workspace_id.to_string(),
                    s.owner_id.to_string(),
                    s.device_id.to_string()
                ],
                |row| row.get::<_, Vec<u8>>(0),
            )
            .map_err(storage)?;
        rows.map(|row| decode(&row.map_err(storage)?)).collect()
    }

    /// Call only after authenticating the service response. Exact retries are safe.
    pub(crate) fn accept_sync_v2_receipts(
        &mut self,
        s: StreamScope,
        receipts: &[MutationReceipt],
    ) -> AppResult<()> {
        if receipts.is_empty() || receipts.len() > 50 {
            return Err(invalid());
        }
        let tx = self.connection.transaction().map_err(storage)?;
        scope(&tx, s)?;
        let mut ids = std::collections::HashSet::new();
        for receipt in receipts {
            if !ids.insert(receipt.record_id)
                || receipt.server_version == 0
                || receipt.server_version > MAX_SAFE_INTEGER
            {
                return Err(blocked());
            }
            let (version,bytes): (u64,Vec<u8>) = tx.query_row("SELECT server_version,envelope FROM sync_v2_record_snapshots WHERE workspace_id=?1 AND record_id=?2", params![s.workspace_id.to_string(),receipt.record_id.to_string()], |row| Ok((row.get::<_, i64>(0)? as u64,row.get(1)?))).optional().map_err(storage)?.ok_or_else(blocked)?;
            let record = decode(&bytes)?;
            if record.header.mutation_id != receipt.mutation_id
                || record.header.expected_server_version + 1 != receipt.server_version
            {
                return Err(blocked());
            }
            let removed = tx.execute("DELETE FROM sync_v2_outbox WHERE workspace_id=?1 AND record_id=?2 AND mutation_id=?3 AND envelope=?4", params![s.workspace_id.to_string(),receipt.record_id.to_string(),receipt.mutation_id.to_string(),bytes]).map_err(storage)?;
            if removed == 0 && version != receipt.server_version {
                return Err(blocked());
            }
            tx.execute("UPDATE sync_v2_record_snapshots SET server_version=?3 WHERE workspace_id=?1 AND record_id=?2", params![s.workspace_id.to_string(),receipt.record_id.to_string(),receipt.server_version as i64]).map_err(storage)?;
            tx.execute(
                "DELETE FROM sync_v2_committed_bases WHERE workspace_id=?1 AND record_id=?2",
                params![s.workspace_id.to_string(), receipt.record_id.to_string()],
            )
            .map_err(storage)?;
        }
        tx.commit().map_err(storage)
    }

    /// Stage a complete checkpoint-covered page, atomically advancing the cursor
    /// and local acknowledgement only after every record and manifest verifies.
    /// No live project table is changed. Partial pages are deferred to C10-04C.
    pub(crate) fn stage_sync_v2_pull(
        &mut self,
        s: StreamScope,
        after: u64,
        changes: &[PulledRecord],
        checkpoint: &SealedWorkspaceCheckpoint,
        key: &impl crate::sync_v2_key_recovery::WorkspaceKeys,
    ) -> AppResult<()> {
        self.stage_sync_v2_pull_with_apply(s, after, changes, checkpoint, key, |_| Ok(()))
    }

    /// Internal transaction hook: live adapters must succeed before the staged
    /// cursor/checkpoint acknowledgement commits. Never expose as a command.
    pub(crate) fn stage_sync_v2_pull_with_apply(
        &mut self,
        s: StreamScope,
        after: u64,
        changes: &[PulledRecord],
        checkpoint: &SealedWorkspaceCheckpoint,
        key: &impl crate::sync_v2_key_recovery::WorkspaceKeys,
        apply: impl FnOnce(&Transaction<'_>) -> AppResult<()>,
    ) -> AppResult<()> {
        self.stage_sync_v2_pull_options(s, after, changes, checkpoint, key, false, apply)
    }
    pub(crate) fn stage_sync_v2_pull_options(
        &mut self,
        s: StreamScope,
        after: u64,
        changes: &[PulledRecord],
        checkpoint: &SealedWorkspaceCheckpoint,
        key: &impl crate::sync_v2_key_recovery::WorkspaceKeys,
        allow_pending: bool,
        apply: impl FnOnce(&Transaction<'_>) -> AppResult<()>,
    ) -> AppResult<()> {
        if changes.is_empty() || changes.len() > 10_000 || after > MAX_SAFE_INTEGER {
            return Err(invalid());
        }
        for (index, change) in changes.iter().enumerate() {
            validate_header(&change.record.header)?;
            if change.change_seq != after + index as u64 + 1
                || change.change_seq > MAX_SAFE_INTEGER
                || change.server_version != change.record.header.expected_server_version + 1
            {
                return Err(blocked());
            }
            matches_scope(s, &change.record)?;
            open_record(&change.record, key)?;
        }
        if checkpoint.through_change_seq != changes.last().expect("nonempty").change_seq
            || checkpoint.checkpoint_counter > MAX_SAFE_INTEGER
        {
            return Err(blocked());
        }
        let tx = self.connection.transaction().map_err(storage)?;
        scope(&tx, s)?;
        let cursor: u64 = tx
            .query_row(
                "SELECT pull_cursor FROM sync_v2_local_streams WHERE workspace_id=?1",
                [s.workspace_id.to_string()],
                |row| Ok(row.get::<_, i64>(0)? as u64),
            )
            .map_err(storage)?;
        let replay = cursor == checkpoint.through_change_seq;
        if cursor != after && !replay {
            return Err(blocked());
        }
        let pending: bool = tx
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM sync_v2_outbox WHERE workspace_id=?1)",
                [s.workspace_id.to_string()],
                |row| row.get(0),
            )
            .map_err(storage)?;
        if pending && !allow_pending {
            return Err(blocked());
        }
        if !replay {
            tx.execute(
                "DELETE FROM sync_v2_applied_receipts WHERE workspace_id=?1",
                [s.workspace_id.to_string()],
            )
            .map_err(storage)?;
        }
        for change in changes {
            let h = &change.record.header;
            let bytes = encode(&change.record)?;
            let history:Option<(u64,Vec<u8>)>=tx.query_row("SELECT server_version,envelope FROM sync_v2_pull_history WHERE workspace_id=?1 AND change_seq=?2",params![s.workspace_id.to_string(),change.change_seq as i64],|r|Ok((r.get::<_,i64>(0)? as u64,r.get(1)?))).optional().map_err(storage)?;
            if replay {
                if history != Some((change.server_version, bytes)) {
                    return Err(blocked());
                }
                continue;
            }
            // Pending encrypted candidates never become the committed baseline.
            if allow_pending {
                let base:Option<(u64,Option<Vec<u8>>) >=tx.query_row("SELECT server_version,envelope FROM sync_v2_committed_bases WHERE workspace_id=?1 AND record_id=?2",params![s.workspace_id.to_string(),h.record_id.to_string()],|r|Ok((r.get::<_,i64>(0)? as u64,r.get(1)?))).optional().map_err(storage)?;
                if let Some((version, base)) = base {
                    if let Some(base) = base {
                        let committed = decode(&base)?;
                        if version == 0
                            || committed.header.owner_id != s.owner_id
                            || committed.header.workspace_id != s.workspace_id
                            || committed.header.record_id != h.record_id
                            || committed.header.record_kind != h.record_kind
                            || committed.header.expected_server_version.checked_add(1)
                                != Some(version)
                        {
                            return Err(blocked());
                        }
                        open_record(&committed, key)?;
                        tx.execute("UPDATE sync_v2_record_snapshots SET server_version=?3,envelope=?4 WHERE workspace_id=?1 AND record_id=?2",params![s.workspace_id.to_string(),h.record_id.to_string(),version as i64,base]).map_err(storage)?;
                    } else {
                        tx.execute("DELETE FROM sync_v2_record_snapshots WHERE workspace_id=?1 AND record_id=?2",params![s.workspace_id.to_string(),h.record_id.to_string()]).map_err(storage)?;
                    }
                    tx.execute("DELETE FROM sync_v2_committed_bases WHERE workspace_id=?1 AND record_id=?2",params![s.workspace_id.to_string(),h.record_id.to_string()]).map_err(storage)?;
                }
            }
            let existing: Option<(u64,u8,Vec<u8>)> = tx.query_row("SELECT server_version,record_kind,envelope FROM sync_v2_record_snapshots WHERE workspace_id=?1 AND record_id=?2",params![s.workspace_id.to_string(),h.record_id.to_string()], |row| Ok((row.get::<_, i64>(0)? as u64,row.get(1)?,row.get(2)?))).optional().map_err(storage)?;
            if replay
                && !existing.as_ref().is_some_and(|(version, kind, old)| {
                    *version == change.server_version && *kind == h.record_kind && *old == bytes
                })
            {
                return Err(blocked());
            }
            if let Some((version, kind, old)) = existing {
                // A just-pushed receipt may already have stored this exact version.
                if kind != h.record_kind
                    || !(version == h.expected_server_version
                        || (version == change.server_version && old == bytes))
                {
                    return Err(blocked());
                }
            } else if h.expected_server_version != 0 {
                return Err(blocked());
            }
            tx.execute("INSERT INTO sync_v2_record_snapshots(workspace_id,record_id,record_kind,server_version,envelope) VALUES(?1,?2,?3,?4,?5) ON CONFLICT(workspace_id,record_id) DO UPDATE SET server_version=excluded.server_version,envelope=excluded.envelope",params![s.workspace_id.to_string(),h.record_id.to_string(),h.record_kind,change.server_version as i64,bytes]).map_err(storage)?;
            tx.execute("INSERT INTO sync_v2_pull_history(workspace_id,change_seq,server_version,envelope) VALUES(?1,?2,?3,?4)",params![s.workspace_id.to_string(),change.change_seq as i64,change.server_version as i64,encode(&change.record)?]).map_err(storage)?;
        }
        let manifest = {
            let mut statement=tx.prepare("SELECT r.record_id,COALESCE(b.server_version,r.server_version),CASE WHEN b.record_id IS NOT NULL THEN b.envelope ELSE r.envelope END FROM sync_v2_record_snapshots r LEFT JOIN sync_v2_committed_bases b ON b.workspace_id=r.workspace_id AND b.record_id=r.record_id WHERE r.workspace_id=?1 AND COALESCE(b.server_version,r.server_version)>0 ORDER BY r.record_id").map_err(storage)?;
            let rows = statement
                .query_map([s.workspace_id.to_string()], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, i64>(1)? as u64,
                        row.get::<_, Vec<u8>>(2)?,
                    ))
                })
                .map_err(storage)?;
            let mut entries = Vec::new();
            for row in rows {
                let (id, revision, bytes) = row.map_err(storage)?;
                let record = decode(&bytes)?;
                if record.header.owner_id != s.owner_id
                    || record.header.workspace_id != s.workspace_id
                    || record.header.record_id.to_string() != id
                    || record.header.expected_server_version.checked_add(1) != Some(revision)
                {
                    return Err(blocked());
                }
                open_record(&record, key)?;
                // Pending edits are still local; this page cannot authenticate the
                // entire hosted set until they have been reconciled.
                if revision == 0 {
                    return Err(blocked());
                }
                entries.push(RecordManifestEntry {
                    record_id: Uuid::parse_str(&id).map_err(|_| invalid())?,
                    record_revision: revision,
                    workspace_key_version: record.header.workspace_key_version,
                    tombstone: record.header.tombstone,
                });
            }
            entries
        };
        let verified =
            open_workspace_checkpoint(s.owner_id, s.workspace_id, key, checkpoint, &manifest)?;
        accept_checkpoint_in_transaction(&tx, s.workspace_id, &verified)?;
        tx.execute("DELETE FROM sync_v2_rotation_batches WHERE workspace_id=?1 AND checkpoint_counter<=?2 AND through_change_seq<=?3",params![s.workspace_id.to_string(),verified.checkpoint_counter as i64,verified.through_change_seq as i64]).map_err(storage)?;
        tx.execute(
            "UPDATE sync_v2_local_streams SET pull_cursor=?2 WHERE workspace_id=?1",
            params![
                s.workspace_id.to_string(),
                verified.through_change_seq as i64
            ],
        )
        .map_err(storage)?;
        tx.execute("INSERT INTO sync_v2_device_acknowledgements(workspace_id,device_id,checkpoint_counter,checkpoint_digest,through_change_seq) VALUES(?1,?2,?3,?4,?5) ON CONFLICT(workspace_id) DO UPDATE SET checkpoint_counter=excluded.checkpoint_counter,checkpoint_digest=excluded.checkpoint_digest,through_change_seq=excluded.through_change_seq",params![s.workspace_id.to_string(),s.device_id.to_string(),verified.checkpoint_counter as i64,verified.manifest_digest.as_slice(),verified.through_change_seq as i64]).map_err(storage)?;
        apply(&tx)?;
        tx.commit().map_err(storage)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sync_v2_record_codec::{
        seal_record,
        tests::{header, note},
        RecordContent,
    };
    use crate::sync_v2_record_protocol::seal_workspace_checkpoint;
    const KEY: [u8; 32] = [0x61; 32];
    fn setup() -> (StreamScope, SealedRecord) {
        let h = header(10);
        let s = StreamScope {
            owner_id: h.owner_id,
            workspace_id: h.workspace_id,
            device_id: Uuid::new_v4(),
        };
        let record = seal_record(h.clone(), &KEY, &note(&h, "Fictional private note")).unwrap();
        (s, record)
    }
    fn receipt(record: &SealedRecord) -> MutationReceipt {
        MutationReceipt {
            record_id: record.header.record_id,
            mutation_id: record.header.mutation_id,
            server_version: record.header.expected_server_version + 1,
        }
    }
    fn checkpoint(
        s: StreamScope,
        counter: u64,
        through: u64,
        records: &[SealedRecord],
    ) -> SealedWorkspaceCheckpoint {
        let manifest = records
            .iter()
            .map(|r| RecordManifestEntry {
                record_id: r.header.record_id,
                record_revision: r.header.expected_server_version + 1,
                workspace_key_version: r.header.workspace_key_version,
                tombstone: r.header.tombstone,
            })
            .collect::<Vec<_>>();
        seal_workspace_checkpoint(
            s.owner_id,
            s.workspace_id,
            1,
            &KEY,
            counter,
            through,
            &manifest,
        )
        .unwrap()
    }
    fn change(seq: u64, r: &SealedRecord) -> PulledRecord {
        PulledRecord {
            change_seq: seq,
            server_version: r.header.expected_server_version + 1,
            record: r.clone(),
        }
    }
    fn count(db: &Database, table: &str) -> i64 {
        // Table names come only from test constants, never provider data.
        db.connection
            .query_row(&format!("SELECT count(*) FROM {table}"), [], |row| {
                row.get(0)
            })
            .unwrap()
    }
    #[test]
    fn schema_eleven_upgrade_preserves_anchor_and_fictional_local_note() {
        let path = std::env::temp_dir().join(format!(
            "sitedatum-c10-04b-upgrade-{}.sqlite3",
            Uuid::new_v4()
        ));
        {
            let connection = rusqlite::Connection::open(&path).unwrap();
            connection.execute_batch("CREATE TABLE schema_migrations(version INTEGER PRIMARY KEY, applied_at_utc TEXT NOT NULL);").unwrap();
            for (index, sql) in [
                include_str!("../migrations/0001_foundation.sql"),
                include_str!("../migrations/0002_projects.sql"),
                include_str!("../migrations/0003_tasks.sql"),
                include_str!("../migrations/0004_rfis.sql"),
                include_str!("../migrations/0005_submittals.sql"),
                include_str!("../migrations/0006_files.sql"),
                include_str!("../migrations/0007_notes_contacts.sql"),
                include_str!("../migrations/0008_cloud_sync.sql"),
                include_str!("../migrations/0009_rfi_pdf_fields.sql"),
                include_str!("../migrations/0010_operations.sql"),
                include_str!("../migrations/0011_sync_v2_checkpoint_anchors.sql"),
            ]
            .iter()
            .enumerate()
            {
                connection.execute_batch(sql).unwrap();
                connection
                    .execute(
                        "INSERT INTO schema_migrations VALUES(?1,'2026-10-10T00:00:00Z')",
                        [index as i64 + 1],
                    )
                    .unwrap();
            }
            connection.execute("INSERT INTO notes VALUES('fictional-note',NULL,'Fictional preserved note','2026-10-10T00:00:00Z','2026-10-10T00:00:00Z')",[]).unwrap();
            connection.execute("INSERT INTO sync_v2_checkpoint_anchors VALUES('fictional-workspace',1,7,?1,44,'2026-10-10T00:00:00Z')",[[0x11u8;32].as_slice()]).unwrap();
        }
        {
            let db = Database::open(&path).unwrap();
            assert_eq!(
                db.connection
                    .query_row("SELECT max(version) FROM schema_migrations", [], |row| row
                        .get::<_, i64>(
                        0
                    ))
                    .unwrap(),
                15
            );
            assert_eq!(
                db.connection
                    .query_row("SELECT body FROM notes", [], |row| row.get::<_, String>(0))
                    .unwrap(),
                "Fictional preserved note"
            );
            assert_eq!(
                db.connection
                    .query_row(
                        "SELECT checkpoint_counter FROM sync_v2_checkpoint_anchors",
                        [],
                        |row| row.get::<_, i64>(0)
                    )
                    .unwrap(),
                7
            );
            assert_eq!(count(&db, "sync_v2_outbox"), 0);
        }
        std::fs::remove_file(path).unwrap();
        // The migration's verified backup uses the shared temp backup folder;
        // leave it in place rather than delete any broad directory.
    }
    #[test]
    fn queue_reopens_with_identical_ciphertext_and_receipts_are_atomic_and_idempotent() {
        let (s, r) = setup();
        let path =
            std::env::temp_dir().join(format!("sitedatum-c10-04b-{}.sqlite3", Uuid::new_v4()));
        {
            let mut db = Database::open(&path).unwrap();
            db.stage_sync_v2_mutation(s, &r, &KEY).unwrap();
            db.stage_sync_v2_mutation(s, &r, &KEY).unwrap();
            assert_eq!(count(&db, "sync_v2_outbox"), 1);
        }
        {
            let mut db = Database::open(&path).unwrap();
            assert_eq!(db.pending_sync_v2_mutations(s).unwrap(), vec![r.clone()]);
            let bytes: Vec<u8> = db
                .connection
                .query_row("SELECT envelope FROM sync_v2_outbox", [], |row| row.get(0))
                .unwrap();
            assert!(!bytes.windows(9).any(|w| w == b"Fictional"));
            let mut bad = receipt(&r);
            bad.mutation_id = Uuid::new_v4();
            assert!(db.accept_sync_v2_receipts(s, &[bad]).is_err());
            assert_eq!(count(&db, "sync_v2_outbox"), 1);
            db.accept_sync_v2_receipts(s, &[receipt(&r)]).unwrap();
            db.accept_sync_v2_receipts(s, &[receipt(&r)]).unwrap();
            assert!(db.pending_sync_v2_mutations(s).unwrap().is_empty());
        }
        std::fs::remove_file(path).unwrap();
    }
    #[test]
    fn snapshot_failure_rolls_back_outbox_and_scope_without_touching_project_work() {
        let (s, r) = setup();
        let mut db = Database::open_in_memory().unwrap();
        db.connection.execute_batch("CREATE TRIGGER fail_snapshot BEFORE INSERT ON sync_v2_record_snapshots BEGIN SELECT RAISE(ABORT,'injected'); END;").unwrap();
        assert!(db.stage_sync_v2_mutation(s, &r, &KEY).is_err());
        assert_eq!(count(&db, "sync_v2_outbox"), 0);
        assert_eq!(count(&db, "sync_v2_local_streams"), 0);
        assert_eq!(count(&db, "sync_v2_record_snapshots"), 0);
        assert_eq!(count(&db, "notes"), 0);
    }
    #[test]
    fn bad_second_receipt_rolls_back_the_entire_batch() {
        let (s, r) = setup();
        let mut h = r.header.clone();
        h.record_id = Uuid::new_v4();
        h.mutation_id = Uuid::new_v4();
        let second = seal_record(h.clone(), &KEY, &note(&h, "Second fictional note")).unwrap();
        let mut db = Database::open_in_memory().unwrap();
        for record in [&r, &second] {
            db.stage_sync_v2_mutation(s, record, &KEY).unwrap();
        }
        let mut bad = receipt(&second);
        bad.server_version = 2;
        assert!(db.accept_sync_v2_receipts(s, &[receipt(&r), bad]).is_err());
        assert_eq!(db.pending_sync_v2_mutations(s).unwrap(), vec![r, second]);
        assert_eq!(
            db.connection
                .query_row(
                    "SELECT sum(server_version) FROM sync_v2_record_snapshots",
                    [],
                    |row| row.get::<_, i64>(0)
                )
                .unwrap(),
            0
        );
    }
    #[test]
    fn pull_cursor_ack_and_anchor_advance_together_and_survive_restart() {
        let (s, r) = setup();
        let cp = checkpoint(s, 1, 1, &[r.clone()]);
        let path =
            std::env::temp_dir().join(format!("sitedatum-c10-04b-pull-{}.sqlite3", Uuid::new_v4()));
        {
            let mut db = Database::open(&path).unwrap();
            db.stage_sync_v2_pull(s, 0, &[change(1, &r)], &cp, &KEY)
                .unwrap();
        }
        {
            let mut db = Database::open(&path).unwrap();
            db.stage_sync_v2_pull(s, 0, &[change(1, &r)], &cp, &KEY)
                .unwrap(); // exact redelivery
            assert_eq!(
                db.connection
                    .query_row("SELECT pull_cursor FROM sync_v2_local_streams", [], |row| {
                        row.get::<_, i64>(0)
                    })
                    .unwrap(),
                1
            );
            assert_eq!(count(&db, "sync_v2_device_acknowledgements"), 1);
            assert_eq!(count(&db, "sync_v2_checkpoint_anchors"), 1);
            assert_eq!(count(&db, "notes"), 0); // staging never applies to live project tables
            let mut altered = r.clone();
            altered.header.mutation_id = Uuid::new_v4();
            assert!(db
                .stage_sync_v2_pull(s, 0, &[change(1, &altered)], &cp, &KEY)
                .is_err());
            let wrong = StreamScope {
                device_id: Uuid::new_v4(),
                ..s
            };
            assert!(db
                .stage_sync_v2_pull(wrong, 0, &[change(1, &r)], &cp, &KEY)
                .is_err());
        }
        std::fs::remove_file(path).unwrap();
    }
    #[test]
    fn corrupt_reordered_omitted_and_failed_ack_pages_leave_no_partial_state() {
        let (s, r) = setup();
        let mut h = r.header.clone();
        h.record_id = Uuid::new_v4();
        h.mutation_id = Uuid::new_v4();
        let second = seal_record(h.clone(), &KEY, &note(&h, "Second fictional note")).unwrap();
        let cp = checkpoint(s, 1, 2, &[r.clone(), second.clone()]);
        let mut db = Database::open_in_memory().unwrap();
        let mut corrupt = second.clone();
        corrupt.ciphertext[30] ^= 1;
        assert!(db
            .stage_sync_v2_pull(s, 0, &[change(1, &r), change(2, &corrupt)], &cp, &KEY)
            .is_err());
        assert!(db
            .stage_sync_v2_pull(s, 0, &[change(2, &second), change(1, &r)], &cp, &KEY)
            .is_err());
        let omitted_cp = checkpoint(s, 1, 1, &[r.clone(), second.clone()]);
        assert!(db
            .stage_sync_v2_pull(s, 0, &[change(1, &r)], &omitted_cp, &KEY)
            .is_err());
        db.connection.execute_batch("CREATE TRIGGER fail_ack BEFORE INSERT ON sync_v2_device_acknowledgements BEGIN SELECT RAISE(ABORT,'injected'); END;").unwrap();
        assert!(db
            .stage_sync_v2_pull(s, 0, &[change(1, &r), change(2, &second)], &cp, &KEY)
            .is_err());
        for table in [
            "sync_v2_local_streams",
            "sync_v2_record_snapshots",
            "sync_v2_checkpoint_anchors",
            "sync_v2_device_acknowledgements",
        ] {
            assert_eq!(count(&db, table), 0);
        }
    }
    #[test]
    fn two_offline_devices_preserve_both_candidates_and_tombstones_are_retained() {
        let (a, r) = setup();
        let b = StreamScope {
            device_id: Uuid::new_v4(),
            ..a
        };
        let mut h = r.header.clone();
        h.mutation_id = Uuid::new_v4();
        let other = seal_record(
            h.clone(),
            &KEY,
            &note(&h, "Different fictional offline edit"),
        )
        .unwrap();
        let mut first = Database::open_in_memory().unwrap();
        let mut second = Database::open_in_memory().unwrap();
        first.stage_sync_v2_mutation(a, &r, &KEY).unwrap();
        second.stage_sync_v2_mutation(b, &other, &KEY).unwrap();
        first.accept_sync_v2_receipts(a, &[receipt(&r)]).unwrap();
        let cp = checkpoint(a, 1, 1, &[r.clone()]);
        assert!(second
            .stage_sync_v2_pull(b, 0, &[change(1, &r)], &cp, &KEY)
            .is_err());
        assert_eq!(second.pending_sync_v2_mutations(b).unwrap(), vec![other]);
        first
            .stage_sync_v2_pull(a, 0, &[change(1, &r)], &cp, &KEY)
            .unwrap();
        let mut deleted = r.header.clone();
        deleted.expected_server_version = 1;
        deleted.mutation_id = Uuid::new_v4();
        deleted.tombstone = true;
        let tombstone = seal_record(
            deleted,
            &KEY,
            &RecordContent {
                schema_version: 1,
                fields: None,
            },
        )
        .unwrap();
        first.stage_sync_v2_mutation(a, &tombstone, &KEY).unwrap();
        first
            .accept_sync_v2_receipts(a, &[receipt(&tombstone)])
            .unwrap();
        let cp2 = checkpoint(a, 2, 2, &[tombstone.clone()]);
        first
            .stage_sync_v2_pull(a, 1, &[change(2, &tombstone)], &cp2, &KEY)
            .unwrap();
        assert!(first
            .stage_sync_v2_pull(a, 0, &[change(1, &r)], &cp, &KEY)
            .is_err());
        assert_eq!(count(&first, "sync_v2_record_snapshots"), 1); // no compaction before later gate
    }
    #[test]
    fn pending_replacement_and_extreme_untrusted_revision_fail_without_panicking() {
        let (s, r) = setup();
        let mut db = Database::open_in_memory().unwrap();
        db.stage_sync_v2_mutation(s, &r, &KEY).unwrap();
        let mut h = r.header.clone();
        h.mutation_id = Uuid::new_v4();
        let replacement = seal_record(h.clone(), &KEY, &note(&h, "Newer local edit")).unwrap();
        assert!(db.stage_sync_v2_mutation(s, &replacement, &KEY).is_err());
        let mut invalid_record = r.clone();
        invalid_record.header.expected_server_version = u64::MAX;
        let cp = checkpoint(s, 1, 1, &[r.clone()]);
        assert!(db
            .stage_sync_v2_pull(
                s,
                0,
                &[PulledRecord {
                    change_seq: 1,
                    server_version: 1,
                    record: invalid_record
                }],
                &cp,
                &KEY
            )
            .is_err());
        assert_eq!(db.pending_sync_v2_mutations(s).unwrap(), vec![r]);
    }
}

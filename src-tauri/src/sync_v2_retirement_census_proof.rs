//! D1 fictional, test-only census and authority-contract proof.
//! This module is excluded from production. It cannot authorize retirement.
//! Checkpoint authentication uses a caller-supplied known fictional manifest;
//! discovering historical manifests and hosted/backup dependencies is not proved.

#[path = "../../proofs/sync-v2-retirement-d1/authority_roles.rs"]
mod roles;

#[cfg(test)]
mod tests {
    use super::roles;
    use crate::{
        error::{AppError, AppResult},
        persistence::{apply_migrations, Database},
        sync_v2_key_recovery::{KeyRing, WorkspaceKeys},
        sync_v2_local_queue::{decode, encode, StreamScope},
        sync_v2_record_codec::{open_record, seal_record, RecordContent, RecordHeader},
        sync_v2_record_protocol::{
            open_workspace_checkpoint, seal_workspace_checkpoint, RecordManifestEntry,
            SealedWorkspaceCheckpoint,
        },
    };
    use roles::{ActiveWriter, HistoricalReader, ReadBackend, ReadKeys, WriteBackend, WriteKeys};
    use rusqlite::{params, Connection, OptionalExtension};
    use std::{collections::BTreeMap, process::Command};
    use uuid::Uuid;

    // Deliberately explicit: schema discovery below rejects unlisted BLOB columns.
    // Resolved conflicts are included; no filter is allowed on resolution state.
    const COLUMNS: [(&str, &str, bool, bool); 12] = [
        ("sync_v2_record_snapshots", "envelope", false, false),
        ("sync_v2_outbox", "envelope", false, false),
        ("sync_v2_committed_bases", "envelope", true, false),
        ("sync_v2_pull_history", "envelope", false, false),
        ("sync_v2_pull_parts", "envelope", false, false),
        ("sync_v2_pull_batches", "checkpoint", false, true),
        ("sync_v2_rotation_batches", "checkpoint", false, true),
        ("sync_v2_record_conflicts", "local_envelope", false, false),
        ("sync_v2_record_conflicts", "remote_envelope", false, false),
        ("sync_v2_record_conflicts", "baseline_envelope", true, false),
        ("sync_v2_record_conflicts", "pending_envelope", true, false),
        (
            "sync_v2_record_conflicts",
            "resolution_envelope",
            true,
            false,
        ),
    ];
    // Proof-harness bounds, not approved product budgets.
    const MAX_CELL_BYTES: usize = 1_048_576;
    const MAX_CELLS: usize = 10_000;

    fn refused() -> AppError {
        AppError::from_technical(
            "SYNC_V2_D1_PROOF_REFUSED",
            "The fictional census proof could not be verified.",
            "Keep all keys and local work. This result cannot authorize retirement.",
            "Proof schema, scope, size, encoding, key or authentication mismatch.",
        )
    }
    struct RingRead<'a>(&'a KeyRing);
    struct RingWrite<'a>(&'a KeyRing);
    impl ReadBackend for RingRead<'_> {
        type Error = AppError;
        fn select(&self, o: [u8; 16], w: [u8; 16], v: u32) -> AppResult<&[u8; 32]> {
            self.0.select(Uuid::from_bytes(o), Uuid::from_bytes(w), v)
        }
    }
    impl WriteBackend for RingWrite<'_> {
        type Error = AppError;
        fn current(&self, o: [u8; 16], w: [u8; 16]) -> AppResult<(u32, &[u8; 32])> {
            let v = self.0.active_version();
            Ok((
                v,
                self.0.select(Uuid::from_bytes(o), Uuid::from_bytes(w), v)?,
            ))
        }
    }
    // Compatibility adapter for existing decrypt APIs; NEVER grants write_version.
    impl WorkspaceKeys for HistoricalReader<RingRead<'_>> {
        fn select(&self, o: Uuid, w: Uuid, v: u32) -> AppResult<&[u8; 32]> {
            self.read(*o.as_bytes(), *w.as_bytes(), v)
        }
        fn write_version(&self) -> Option<u32> {
            None
        }
    }

    #[derive(Debug, PartialEq)]
    struct Census {
        counts: [usize; 12],
        nulls: [usize; 12],
        versions: BTreeMap<u32, usize>,
    }

    fn verify_schema(c: &Connection) -> AppResult<()> {
        let mut actual = Vec::new();
        let mut tables = c
            .prepare("SELECT name FROM sqlite_master WHERE type='table' AND name GLOB 'sync_v2_*'")
            .map_err(|_| refused())?;
        let names = tables
            .query_map([], |r| r.get::<_, String>(0))
            .map_err(|_| refused())?;
        for name in names {
            let name = name.map_err(|_| refused())?;
            // A bound table-valued pragma avoids interpolating schema identifiers.
            let mut columns = c
                .prepare("SELECT name, type, \"notnull\" FROM pragma_table_info(?1)")
                .map_err(|_| refused())?;
            let rows = columns
                .query_map([&name], |r| {
                    Ok((
                        r.get::<_, String>(0)?,
                        r.get::<_, String>(1)?,
                        r.get::<_, bool>(2)?,
                    ))
                })
                .map_err(|_| refused())?;
            for row in rows {
                let (column, kind, required) = row.map_err(|_| refused())?;
                let known_digest = column == "checkpoint_digest"
                    && matches!(
                        name.as_str(),
                        "sync_v2_checkpoint_anchors"
                            | "sync_v2_device_acknowledgements"
                            | "sync_v2_applied_receipts"
                    );
                if kind.eq_ignore_ascii_case("BLOB") && !known_digest {
                    actual.push((name.clone(), column, !required));
                }
            }
        }
        let mut expected = COLUMNS
            .iter()
            .map(|(t, c, nullable, _)| (t.to_string(), c.to_string(), *nullable))
            .collect::<Vec<_>>();
        actual.sort();
        expected.sort();
        if actual != expected {
            return Err(refused());
        }
        Ok(())
    }

    fn census(
        c: &Connection,
        scope: StreamScope,
        keys: &impl WorkspaceKeys,
        known_manifest: &[RecordManifestEntry],
    ) -> AppResult<Census> {
        // Require a caller-created query-only fixture. This is not a database opener.
        if !c
            .query_row("PRAGMA query_only", [], |r| r.get::<_, bool>(0))
            .map_err(|_| refused())?
        {
            return Err(refused());
        }
        let tx = c.unchecked_transaction().map_err(|_| refused())?;
        verify_schema(&tx)?;
        let owner_device: Option<(String, String)> = tx
            .query_row(
                "SELECT owner_id, device_id FROM sync_v2_local_streams WHERE workspace_id=?1",
                [scope.workspace_id.to_string()],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()
            .map_err(|_| refused())?;
        if owner_device != Some((scope.owner_id.to_string(), scope.device_id.to_string())) {
            return Err(refused());
        }
        let mut result = Census {
            counts: [0; 12],
            nulls: [0; 12],
            versions: BTreeMap::new(),
        };
        let mut cells = 0;
        for (index, (table, column, nullable, checkpoint)) in COLUMNS.iter().enumerate() {
            // Only compile-time allowlisted identifiers enter SQL. CASE prevents a
            // corrupt oversized value from being copied into the Rust row buffer.
            let sql = format!("SELECT typeof({column}), length({column}), CASE WHEN length({column}) <= {MAX_CELL_BYTES} THEN {column} ELSE NULL END FROM {table} WHERE workspace_id=?1");
            let mut statement = tx.prepare(&sql).map_err(|_| refused())?;
            let mut rows = statement
                .query([scope.workspace_id.to_string()])
                .map_err(|_| refused())?;
            while let Some(row) = rows.next().map_err(|_| refused())? {
                cells += 1;
                if cells > MAX_CELLS {
                    return Err(refused());
                }
                let kind: String = row.get(0).map_err(|_| refused())?;
                if kind == "null" && *nullable {
                    result.nulls[index] += 1;
                    continue;
                }
                let size: i64 = row.get(1).map_err(|_| refused())?;
                if kind != "blob" || size <= 0 || size > MAX_CELL_BYTES as i64 {
                    return Err(refused());
                }
                let bytes: Vec<u8> = row.get(2).map_err(|_| refused())?;
                let version = if *checkpoint {
                    let sealed: SealedWorkspaceCheckpoint =
                        serde_json::from_slice(&bytes).map_err(|_| refused())?;
                    open_workspace_checkpoint(
                        scope.owner_id,
                        scope.workspace_id,
                        keys,
                        &sealed,
                        known_manifest,
                    )
                    .map_err(|_| refused())?;
                    sealed.workspace_key_version
                } else {
                    let record = decode(&bytes).map_err(|_| refused())?;
                    if record.header.owner_id != scope.owner_id
                        || record.header.workspace_id != scope.workspace_id
                    {
                        return Err(refused());
                    }
                    open_record(&record, keys).map_err(|_| refused())?;
                    record.header.workspace_key_version
                };
                result.counts[index] += 1;
                *result.versions.entry(version).or_default() += 1;
            }
        }
        // A read transaction is deliberately rolled back, not committed as evidence.
        tx.rollback().map_err(|_| refused())?;
        Ok(result)
    }

    struct Fixture {
        c: Connection,
        scope: StreamScope,
        ring: KeyRing,
        record: Vec<u8>,
    }
    impl Fixture {
        fn new() -> Self {
            let scope = StreamScope {
                owner_id: Uuid::new_v4(),
                workspace_id: Uuid::new_v4(),
                device_id: Uuid::new_v4(),
            };
            let mut ring = KeyRing::initial(scope.owner_id, scope.workspace_id).unwrap();
            ring.rotate().unwrap();
            let header = RecordHeader {
                owner_id: scope.owner_id,
                workspace_id: scope.workspace_id,
                record_id: Uuid::new_v4(),
                record_kind: 10,
                mutation_id: Uuid::new_v4(),
                expected_server_version: 0,
                workspace_key_version: 1,
                protocol_version: 1,
                tombstone: true,
            };
            let record = encode(
                &seal_record(
                    header.clone(),
                    &ring,
                    &RecordContent {
                        schema_version: 1,
                        fields: None,
                    },
                )
                .unwrap(),
            )
            .unwrap();
            let checkpoint = serde_json::to_vec(
                &seal_workspace_checkpoint(scope.owner_id, scope.workspace_id, 1, &ring, 1, 1, &[])
                    .unwrap(),
            )
            .unwrap();
            let mut c = Connection::open_in_memory().unwrap();
            apply_migrations(&mut c).unwrap();
            let w = scope.workspace_id.to_string();
            let id = header.record_id.to_string();
            c.execute(
                "INSERT INTO sync_v2_local_streams VALUES(?1,?2,?3,0)",
                params![w, scope.owner_id.to_string(), scope.device_id.to_string()],
            )
            .unwrap();
            c.execute(
                "INSERT INTO sync_v2_record_snapshots VALUES(?1,?2,10,1,?3)",
                params![w, id, record],
            )
            .unwrap();
            c.execute("INSERT INTO sync_v2_outbox(workspace_id,mutation_id,record_id,envelope) VALUES(?1,?2,?3,?4)", params![w,header.mutation_id.to_string(),id,record]).unwrap();
            c.execute(
                "INSERT INTO sync_v2_committed_bases VALUES(?1,?2,1,?3)",
                params![w, id, record],
            )
            .unwrap();
            c.execute(
                "INSERT INTO sync_v2_pull_history VALUES(?1,1,1,?2)",
                params![w, record],
            )
            .unwrap();
            c.execute(
                "INSERT INTO sync_v2_pull_batches VALUES(?1,0,?2)",
                params![w, checkpoint],
            )
            .unwrap();
            c.execute(
                "INSERT INTO sync_v2_pull_parts VALUES(?1,1,1,?2)",
                params![w, record],
            )
            .unwrap();
            c.execute(
                "INSERT INTO sync_v2_rotation_batches VALUES(?1,2,1,1,?2)",
                params![w, checkpoint],
            )
            .unwrap();
            for resolved in [false, true] {
                c.execute(
                    "INSERT INTO sync_v2_record_conflicts VALUES(?1,?2,?3,?4,?4,?4,?4,?5,?6,?4)",
                    params![
                        w,
                        id,
                        if resolved { 2 } else { 1 },
                        record,
                        if resolved { Some(2) } else { None },
                        if resolved { Some("keep_local") } else { None }
                    ],
                )
                .unwrap();
            }
            c.execute_batch("PRAGMA query_only=ON").unwrap();
            Self {
                c,
                scope,
                ring,
                record,
            }
        }
        fn scan(&self) -> AppResult<Census> {
            census(
                &self.c,
                self.scope,
                &HistoricalReader::new(RingRead(&self.ring)),
                &[],
            )
        }
        fn update_column(&self, index: usize, bytes: &[u8]) {
            let (table, column, _, _) = COLUMNS[index];
            self.c.execute_batch("PRAGMA query_only=OFF").unwrap();
            self.c
                .execute(&format!("UPDATE {table} SET {column}=?1"), [bytes])
                .unwrap();
            self.c.execute_batch("PRAGMA query_only=ON").unwrap();
        }
    }

    #[test]
    fn twelve_columns_include_nullable_and_resolved_rows_without_writes() {
        let f = Fixture::new();
        let changes = f.c.total_changes();
        let result = f.scan().unwrap();
        assert_eq!(result.counts, [1, 1, 1, 1, 1, 1, 1, 2, 2, 2, 2, 2]);
        assert_eq!(result.nulls, [0; 12]);
        assert_eq!(result.versions.get(&1), Some(&17));
        assert_eq!(f.c.total_changes(), changes);
        let stored: Vec<u8> =
            f.c.query_row("SELECT envelope FROM sync_v2_pull_history", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(stored, f.record);
        assert!(f.c.execute("DELETE FROM sync_v2_outbox", []).is_err());
    }

    #[test]
    fn file_vault_total_loss_recovery_reads_all_twelve_columns_at_every_retained_version() {
        use crate::sync_v2_vault_file_prototype::{read_bounded, write_new, Archive, Secret};
        use zeroize::Zeroizing;
        for count in [1, 8, 100, 1000] {
            let f = Fixture::new();
            let scope = f.scope;
            let archive = Archive::new(
                scope.owner_id,
                scope.workspace_id,
                1,
                (1..=count)
                    .map(|v| (v, Zeroizing::new([((v % 251) + 1) as u8; 32])))
                    .collect(),
            )
            .unwrap();
            // Public fictional keys are deliberate. No real keyring or credentials.
            // Seal before losing the archive, so recovery is not its own oracle.
            let mut captured = Vec::with_capacity(count as usize);
            for version in 1..=count {
                let header = RecordHeader {
                    owner_id: scope.owner_id,
                    workspace_id: scope.workspace_id,
                    record_id: Uuid::new_v4(),
                    record_kind: 10,
                    mutation_id: Uuid::new_v4(),
                    expected_server_version: 0,
                    workspace_key_version: version,
                    protocol_version: 1,
                    tombstone: true,
                };
                let record = encode(
                    &seal_record(
                        header,
                        &archive,
                        &RecordContent {
                            schema_version: 1,
                            fields: None,
                        },
                    )
                    .unwrap(),
                )
                .unwrap();
                let checkpoint = serde_json::to_vec(
                    &seal_workspace_checkpoint(
                        scope.owner_id,
                        scope.workspace_id,
                        version,
                        &archive,
                        version as u64,
                        version as u64,
                        &[],
                    )
                    .unwrap(),
                )
                .unwrap();
                captured.push((record, checkpoint));
            }
            let recovery = Secret::random().unwrap();
            let local = Secret::random().unwrap();
            let (local_file, _) = archive.seal(&local, false).unwrap();
            let (copy, pin) = archive.seal(&recovery, true).unwrap();
            let path = std::env::temp_dir()
                .join(format!("sitedatum-d3-recovery-{}.fixture", Uuid::new_v4()));
            write_new(&path, &copy).unwrap();
            // Destructure to discard even the original unrelated two-key fixture ring.
            let Fixture { c, ring, .. } = f;
            drop(ring);
            drop(archive);
            drop(local);
            drop(local_file);
            drop(copy);
            let restored = Archive::open(
                &read_bounded(&path).unwrap(),
                &recovery,
                true,
                scope.owner_id,
                scope.workspace_id,
                pin,
            )
            .unwrap();
            assert_eq!(restored.write_version(), None);
            for (index, (record, checkpoint)) in captured.iter().enumerate() {
                c.execute_batch("PRAGMA query_only=OFF").unwrap();
                for (table, column, _, is_checkpoint) in COLUMNS {
                    c.execute(
                        &format!("UPDATE {table} SET {column}=?1"),
                        [if is_checkpoint { checkpoint } else { record }],
                    )
                    .unwrap();
                }
                c.execute_batch("PRAGMA query_only=ON").unwrap();
                let before = c.total_changes();
                let result = census(&c, scope, &restored, &[]).unwrap();
                assert_eq!(result.counts, [1, 1, 1, 1, 1, 1, 1, 2, 2, 2, 2, 2]);
                assert_eq!(result.versions.get(&(index as u32 + 1)), Some(&17));
                assert_eq!(c.total_changes(), before);
            }
            // Cleanup only the unique fictional file explicitly created by this drill.
            std::fs::remove_file(&path).unwrap();
        }
    }

    #[test]
    fn all_four_nullable_columns_are_counted_not_omitted() {
        let f = Fixture::new();
        f.c.execute_batch("PRAGMA query_only=OFF; UPDATE sync_v2_committed_bases SET envelope=NULL; UPDATE sync_v2_record_conflicts SET baseline_envelope=NULL,pending_envelope=NULL,resolution_envelope=NULL; PRAGMA query_only=ON;").unwrap();
        let result = f.scan().unwrap();
        for (i, (_, _, nullable, _)) in COLUMNS.iter().enumerate() {
            assert_eq!(
                result.nulls[i],
                if *nullable {
                    if i == 2 {
                        1
                    } else {
                        2
                    }
                } else {
                    0
                }
            );
        }
        assert_eq!(result.versions.get(&1), Some(&10));
    }

    #[test]
    fn every_column_rejects_corruption_and_keeps_original_keys() {
        for i in 0..12 {
            let f = Fixture::new();
            f.update_column(i, b"invalid fictional encoding");
            let changes = f.c.total_changes();
            assert!(f.scan().is_err(), "column {i}");
            assert_eq!(f.c.total_changes(), changes);
            assert_eq!(f.ring.active_version(), 2);
            assert!(f
                .ring
                .select(f.scope.owner_id, f.scope.workspace_id, 1)
                .is_ok());
        }
    }

    #[test]
    fn every_column_rejects_well_framed_ciphertext_tampering() {
        for (index, (_, _, _, checkpoint)) in COLUMNS.iter().enumerate() {
            let f = Fixture::new();
            let bytes = if *checkpoint {
                let mut sealed = seal_workspace_checkpoint(
                    f.scope.owner_id,
                    f.scope.workspace_id,
                    1,
                    &f.ring,
                    1,
                    1,
                    &[],
                )
                .unwrap();
                sealed.ciphertext[25] ^= 1;
                serde_json::to_vec(&sealed).unwrap()
            } else {
                let mut record = decode(&f.record).unwrap();
                record.ciphertext[25] ^= 1;
                encode(&record).unwrap()
            };
            f.update_column(index, &bytes);
            assert!(f.scan().is_err(), "authenticated column {index}");
        }
    }

    #[test]
    fn authenticated_tampering_wrong_scope_missing_key_and_oversize_refuse() {
        let f = Fixture::new();
        let mut record = decode(&f.record).unwrap();
        record.ciphertext[25] ^= 1;
        f.update_column(0, &encode(&record).unwrap());
        assert!(f.scan().is_err());
        let f = Fixture::new();
        let wrong = StreamScope {
            owner_id: Uuid::new_v4(),
            ..f.scope
        };
        assert!(census(&f.c, wrong, &HistoricalReader::new(RingRead(&f.ring)), &[]).is_err());
        let other = KeyRing::initial(f.scope.owner_id, f.scope.workspace_id).unwrap();
        assert!(census(&f.c, f.scope, &HistoricalReader::new(RingRead(&other)), &[]).is_err());
        f.update_column(0, &vec![0; MAX_CELL_BYTES + 1]);
        assert!(f.scan().is_err());
    }

    #[test]
    fn schema_drift_unknown_ciphertext_column_and_wrong_nullability_refuse() {
        let f = Fixture::new();
        f.c.execute_batch("PRAGMA query_only=OFF; ALTER TABLE sync_v2_pull_history ADD COLUMN extra_ciphertext BLOB; PRAGMA query_only=ON;").unwrap();
        assert!(f.scan().is_err());
        let f = Fixture::new();
        f.c.execute_batch("PRAGMA query_only=OFF; ALTER TABLE sync_v2_committed_bases RENAME COLUMN envelope TO renamed; PRAGMA query_only=ON;").unwrap();
        assert!(f.scan().is_err());
        let c = Connection::open_in_memory().unwrap();
        c.execute_batch(include_str!("../migrations/0012_sync_v2_local_queue.sql"))
            .unwrap();
        c.execute_batch(
            &include_str!("../migrations/0014_sync_v2_apply_protocol.sql").replace(
                "envelope BLOB, PRIMARY KEY",
                "envelope BLOB NOT NULL, PRIMARY KEY",
            ),
        )
        .unwrap();
        c.execute_batch(include_str!("../migrations/0015_sync_v2_key_rotation.sql"))
            .unwrap();
        assert!(verify_schema(&c).is_err());
    }

    #[test]
    fn absent_exact_version_and_checkpoint_manifest_mismatch_refuse() {
        let f = Fixture::new();
        let mut header = decode(&f.record).unwrap().header;
        header.workspace_key_version = 2;
        let record = seal_record(
            header,
            &f.ring,
            &RecordContent {
                schema_version: 1,
                fields: None,
            },
        )
        .unwrap();
        f.update_column(0, &encode(&record).unwrap());
        let only_v1 = KeyRing::initial(f.scope.owner_id, f.scope.workspace_id).unwrap();
        assert!(census(
            &f.c,
            f.scope,
            &HistoricalReader::new(RingRead(&only_v1)),
            &[]
        )
        .is_err());
        let f = Fixture::new();
        let wrong_manifest = [RecordManifestEntry {
            record_id: Uuid::new_v4(),
            record_revision: 1,
            workspace_key_version: 1,
            tombstone: false,
        }];
        assert!(census(
            &f.c,
            f.scope,
            &HistoricalReader::new(RingRead(&f.ring)),
            &wrong_manifest
        )
        .is_err());
    }

    #[test]
    fn read_only_mode_and_total_cell_budget_are_required() {
        let f = Fixture::new();
        f.c.execute_batch("PRAGMA query_only=OFF").unwrap();
        assert!(f.scan().is_err());
        f.c.execute("WITH RECURSIVE n(x) AS (SELECT 2 UNION ALL SELECT x+1 FROM n WHERE x<10002) INSERT INTO sync_v2_pull_history(workspace_id,change_seq,server_version,envelope) SELECT ?1,x,1,?2 FROM n",params![f.scope.workspace_id.to_string(),f.record]).unwrap();
        f.c.execute_batch("PRAGMA query_only=ON").unwrap();
        let changes = f.c.total_changes();
        assert!(f.scan().is_err());
        assert_eq!(f.c.total_changes(), changes);
    }

    #[test]
    fn census_is_not_stability_or_retirement_authority() {
        let f = Fixture::new();
        let before = f.scan().unwrap();
        f.c.execute_batch("PRAGMA query_only=OFF").unwrap();
        f.c.execute(
            "INSERT INTO sync_v2_pull_history VALUES(?1,2,1,?2)",
            params![f.scope.workspace_id.to_string(), f.record],
        )
        .unwrap();
        f.c.execute_batch("PRAGMA query_only=ON").unwrap();
        let after = f.scan().unwrap();
        assert_eq!(after.counts[3], before.counts[3] + 1);
        assert_eq!(after.versions.get(&1), Some(&18));
        // Newly acquired dependency is still readable. No key was pruned or floor committed.
        assert_eq!(f.ring.active_version(), 2);
        assert!(f
            .ring
            .select(f.scope.owner_id, f.scope.workspace_id, 1)
            .is_ok());
    }

    #[test]
    fn historical_reader_cannot_authorize_existing_queue_writes() {
        let f = Fixture::new();
        let reader = HistoricalReader::new(RingRead(&f.ring));
        assert_eq!(reader.write_version(), None);
        assert!(reader
            .select(f.scope.owner_id, f.scope.workspace_id, 1)
            .is_ok());
        let writer = ActiveWriter::new(RingWrite(&f.ring));
        assert_eq!(
            writer
                .for_current_write(
                    *f.scope.owner_id.as_bytes(),
                    *f.scope.workspace_id.as_bytes()
                )
                .unwrap()
                .0,
            2
        );
        let mut c = Connection::open_in_memory().unwrap();
        apply_migrations(&mut c).unwrap();
        let mut db = Database { connection: c };
        let changes = db.connection.total_changes();
        assert!(db
            .stage_sync_v2_mutation(f.scope, &decode(&f.record).unwrap(), &reader)
            .is_err());
        assert_eq!(db.connection.total_changes(), changes);
    }

    #[test]
    fn restored_file_archive_has_no_queue_write_authority() {
        use crate::sync_v2_vault_file_prototype::{Archive, Secret};
        use zeroize::Zeroizing;
        let f = Fixture::new();
        let a = Archive::new(
            f.scope.owner_id,
            f.scope.workspace_id,
            1,
            vec![(
                1,
                Zeroizing::new(
                    *f.ring
                        .select(f.scope.owner_id, f.scope.workspace_id, 1)
                        .unwrap(),
                ),
            )],
        )
        .unwrap();
        let secret = Secret::random().unwrap();
        let (bytes, pin) = a.seal(&secret, true).unwrap();
        drop(a);
        let restored = Archive::open(
            &bytes,
            &secret,
            true,
            f.scope.owner_id,
            f.scope.workspace_id,
            pin,
        )
        .unwrap();
        assert!(open_record(&decode(&f.record).unwrap(), &restored).is_ok());
        let mut c = Connection::open_in_memory().unwrap();
        apply_migrations(&mut c).unwrap();
        let mut db = Database { connection: c };
        let before = db.connection.total_changes();
        assert!(db
            .stage_sync_v2_mutation(f.scope, &decode(&f.record).unwrap(), &restored)
            .is_err());
        assert_eq!(db.connection.total_changes(), before);
        assert!(f
            .ring
            .select(f.scope.owner_id, f.scope.workspace_id, 1)
            .is_ok());
    }

    #[test]
    fn role_compile_failures_have_a_successful_positive_control() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../proofs/sync-v2-retirement-d1/compile-fixtures");
        let output_root = std::env::var_os("SITEDATUM_D1_PROOF_OUTPUT")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| std::env::temp_dir().join("sitedatum-d1-fictional-compile-proof"));
        let output = output_root.join(Uuid::new_v4().to_string());
        std::fs::create_dir_all(&output).unwrap();
        for (name, succeeds, role) in [
            ("valid_roles", true, ""),
            ("read_as_writer", false, "WriteKeys"),
            ("write_as_reader", false, "ReadKeys"),
        ] {
            let result = Command::new("rustc")
                .args(["--edition=2021", "--emit=metadata", "--error-format=json"])
                .arg(root.join(format!("{name}.rs")))
                .arg("--out-dir")
                .arg(&output)
                .output()
                .unwrap();
            assert_eq!(result.status.success(), succeeds, "fixture {name}");
            if !succeeds {
                let diagnostics = String::from_utf8(result.stderr).unwrap();
                assert!(
                    diagnostics.contains("E0277") && diagnostics.contains(role),
                    "expected authority trait error, not an unrelated compile error"
                );
            }
        }
        // Compiler metadata remains in disposable task temporary storage, no user-file cleanup.
    }

    #[test]
    fn hosted_lock_then_idempotency_order_is_preserved() {
        let sql = include_str!("../../proofs/sync-v2-identity/supabase/migrations/20261010022507_c10_03g_encrypted_record_operations.sql");
        let (_, push) = sql
            .split_once("create function public.sync_v2_record_bridge_push_batch(")
            .unwrap();
        let lock = push
            .find("perform pg_catalog.pg_advisory_xact_lock(")
            .unwrap();
        let retry = push.find("select b.* into existing_batch").unwrap();
        let row_lock = push.find("for update;").unwrap();
        assert!(lock < retry && retry < row_lock);
    }
}

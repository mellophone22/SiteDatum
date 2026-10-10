//! C10-04C1: dormant, fail-closed notes/contact adapters. No transport or commands.
#![allow(dead_code)]

use crate::{
    error::{AppError, AppResult},
    persistence::Database,
    recovery::{self, BackupInfo},
    sync_v2_local_queue::{decode, PulledRecord, StreamScope},
    sync_v2_record_codec::{invalid, open_record},
    sync_v2_record_protocol::SealedWorkspaceCheckpoint,
};
use rusqlite::{backup::Backup, params, Connection, OptionalExtension, Transaction};
use serde_json::{json, Value};
use std::{path::Path, time::Duration};
use uuid::Uuid;

fn refused() -> AppError {
    AppError::from_technical(
        "SYNC_V2_APPLY_REFUSED",
        "The verified Sync records could not be applied safely.",
        "Keep local work and review the conflicting or unsupported record.",
        "Local baseline, domain, reference, or transaction validation failed.",
    )
}
fn sql(_: rusqlite::Error) -> AppError {
    // SQL/provider values, contact details and note bodies never enter errors.
    refused()
}
// Codec validation already guarantees UTC ISO timestamps. Normalize fractional
// seconds before comparing: lexical order puts "...00Z" after "...00.1Z".
fn timestamp_order(value: &str) -> (&str, String) {
    let fraction = value[19..].trim_start_matches('.').trim_end_matches('Z');
    (&value[..19], format!("{fraction:0<9}"))
}
fn live(connection: &Connection, kind: u8, id: Uuid) -> AppResult<Option<Value>> {
    let id = id.to_string();
    match kind {
        10 => connection.query_row(
            "SELECT id,project_id,body,created_at_utc,updated_at_utc FROM notes WHERE id=?1",
            [id], |r| Ok(json!({"id":r.get::<_,String>(0)?,"project_id":r.get::<_,Option<String>>(1)?,"body":r.get::<_,String>(2)?,"created_at_utc":r.get::<_,String>(3)?,"updated_at_utc":r.get::<_,String>(4)?})),
        ).optional().map_err(sql),
        11 => connection.query_row(
            "SELECT id,name,company,email,phone,role,created_at_utc,updated_at_utc FROM contacts WHERE id=?1",
            [id], |r| Ok(json!({"id":r.get::<_,String>(0)?,"name":r.get::<_,String>(1)?,"company":r.get::<_,Option<String>>(2)?,"email":r.get::<_,Option<String>>(3)?,"phone":r.get::<_,Option<String>>(4)?,"role":r.get::<_,Option<String>>(5)?,"created_at_utc":r.get::<_,String>(6)?,"updated_at_utc":r.get::<_,String>(7)?})),
        ).optional().map_err(sql),
        _ => Err(refused()),
    }
}
struct Prepared {
    id: Uuid,
    kind: u8,
    before: Option<Value>,
    after: Option<Value>,
}
fn prepare(
    db: &Database,
    scope: StreamScope,
    changes: &[PulledRecord],
    key: &[u8; 32],
) -> AppResult<Vec<Prepared>> {
    changes.iter().map(|change| {
        let h = &change.record.header;
        if h.owner_id != scope.owner_id || h.workspace_id != scope.workspace_id || !matches!(h.record_kind,10|11) {
            return Err(refused());
        }
        let after = open_record(&change.record,key)?.fields;
        if let Some(fields) = &after {
            let required = if h.record_kind == 10 { "body" } else { "name" };
            if fields[required].as_str().is_none_or(|s|s.trim().is_empty()) {
                return Err(refused());
            }
            // Never normalize imported values silently, including timestamps.
            if timestamp_order(text(fields, "created_at_utc")?)
                > timestamp_order(text(fields, "updated_at_utc")?)
            {
                return Err(refused());
            }
        }
        let before = live(&db.connection,h.record_kind,h.record_id)?;
        let baseline: Option<(u8,Vec<u8>)> = db.connection.query_row(
            "SELECT record_kind,envelope FROM sync_v2_record_snapshots WHERE workspace_id=?1 AND record_id=?2",
            params![scope.workspace_id.to_string(),h.record_id.to_string()],
            |r|Ok((r.get(0)?,r.get(1)?)),
        ).optional().map_err(sql)?;
        let expected = match baseline {
            Some((kind,bytes)) => {
                let old = decode(&bytes)?;
                if kind != h.record_kind || old.header.owner_id != scope.owner_id || old.header.workspace_id != scope.workspace_id || old.header.record_id != h.record_id {
                    return Err(refused());
                }
                open_record(&old,key)?.fields
            },
            None => None,
        };
        // Exact remote redelivery is harmless; divergent local work is never
        // overwritten, even when the outbox was not populated by an adapter.
        if before != expected && before != after {
            return Err(refused());
        }
        Ok(Prepared {id:h.record_id,kind:h.record_kind,before,after})
    }).collect()
}
fn text<'a>(fields: &'a Value, name: &str) -> AppResult<&'a str> {
    fields[name].as_str().ok_or_else(invalid)
}
fn nullable<'a>(fields: &'a Value, name: &str) -> AppResult<Option<&'a str>> {
    if fields[name].is_null() {
        Ok(None)
    } else {
        text(fields, name).map(Some)
    }
}
fn apply(tx: &Transaction<'_>, records: &[Prepared]) -> AppResult<()> {
    for record in records {
        if live(tx, record.kind, record.id)? != record.before {
            return Err(refused());
        }
        let id = record.id.to_string();
        match (record.kind, &record.after) {
            (10, Some(f)) => {
                tx.execute("INSERT INTO notes(id,project_id,body,created_at_utc,updated_at_utc) VALUES(?1,?2,?3,?4,?5) ON CONFLICT(id) DO UPDATE SET project_id=excluded.project_id,body=excluded.body,created_at_utc=excluded.created_at_utc,updated_at_utc=excluded.updated_at_utc",params![id,nullable(f,"project_id")?,text(f,"body")?,text(f,"created_at_utc")?,text(f,"updated_at_utc")?]).map_err(sql)?;
            }
            (11, Some(f)) => {
                tx.execute("INSERT INTO contacts(id,name,company,email,phone,role,created_at_utc,updated_at_utc) VALUES(?1,?2,?3,?4,?5,?6,?7,?8) ON CONFLICT(id) DO UPDATE SET name=excluded.name,company=excluded.company,email=excluded.email,phone=excluded.phone,role=excluded.role,created_at_utc=excluded.created_at_utc,updated_at_utc=excluded.updated_at_utc",params![id,text(f,"name")?,nullable(f,"company")?,nullable(f,"email")?,nullable(f,"phone")?,nullable(f,"role")?,text(f,"created_at_utc")?,text(f,"updated_at_utc")?]).map_err(sql)?;
            }
            (10, None) => {
                tx.execute("DELETE FROM notes WHERE id=?1", [id])
                    .map_err(sql)?;
            }
            (11, None) => {
                let referenced: bool = tx
                    .query_row(
                        "SELECT EXISTS(SELECT 1 FROM tasks WHERE related_contact_id=?1)",
                        [&id],
                        |r| r.get(0),
                    )
                    .map_err(sql)?;
                if referenced {
                    return Err(refused());
                }
                tx.execute("DELETE FROM contacts WHERE id=?1", [id])
                    .map_err(sql)?;
            }
            _ => return Err(refused()),
        }
    }
    let violations: bool = tx
        .prepare("PRAGMA foreign_key_check")
        .map_err(sql)?
        .exists([])
        .map_err(sql)?;
    if violations {
        return Err(refused());
    }
    Ok(())
}

impl Database {
    /// Exclusive local DB access is required throughout preflight/backup/apply.
    /// Only a complete authenticated page is accepted. No applied-device receipt
    /// is issued, no file is touched, and no network/consent gate is enabled.
    pub(crate) fn apply_sync_v2_notes_contacts(
        &mut self,
        scope: StreamScope,
        after: u64,
        changes: &[PulledRecord],
        checkpoint: &SealedWorkspaceCheckpoint,
        key: &[u8; 32],
        backup_directory: &Path,
    ) -> AppResult<BackupInfo> {
        let prepared = prepare(self, scope, changes, key)?;
        // Validate the exact transaction against a private RAM copy, including
        // FK constraints, trigger failures and the whole checkpoint manifest.
        let mut candidate = Connection::open_in_memory().map_err(sql)?;
        {
            let backup = Backup::new(&self.connection, &mut candidate).map_err(sql)?;
            backup
                .run_to_completion(64, Duration::from_millis(1), None)
                .map_err(sql)?;
        }
        candidate
            .execute_batch("PRAGMA foreign_keys=ON;")
            .map_err(sql)?;
        let mut candidate = Database {
            connection: candidate,
        };
        candidate.stage_sync_v2_pull_with_apply(scope, after, changes, checkpoint, key, |tx| {
            apply(tx, &prepared)
        })?;
        let backup = recovery::create(
            self,
            backup_directory,
            &format!("sync-v2-pre-apply-{}", Uuid::new_v4()),
        )?;
        self.stage_sync_v2_pull_with_apply(scope, after, changes, checkpoint, key, |tx| {
            apply(tx, &prepared)
        })?;
        Ok(backup)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sync_v2_record_codec::{
        seal_record,
        tests::{header, note},
        RecordContent, SealedRecord,
    };
    use crate::sync_v2_record_protocol::{seal_workspace_checkpoint, RecordManifestEntry};
    const KEY: [u8; 32] = [0x41; 32];
    struct Fixture {
        db: Database,
        scope: StreamScope,
        root: std::path::PathBuf,
    }
    impl Fixture {
        fn new() -> Self {
            let h = header(10);
            let root = std::env::temp_dir().join(format!("sitedatum-c10-04c1-{}", Uuid::new_v4()));
            std::fs::create_dir(&root).unwrap();
            Self {
                db: Database::open_in_memory().unwrap(),
                scope: StreamScope {
                    owner_id: h.owner_id,
                    workspace_id: h.workspace_id,
                    device_id: Uuid::new_v4(),
                },
                root,
            }
        }
        fn run(
            &mut self,
            after: u64,
            records: &[SealedRecord],
            manifest: &[SealedRecord],
        ) -> AppResult<BackupInfo> {
            let through = after + records.len() as u64;
            let cp = seal_workspace_checkpoint(
                self.scope.owner_id,
                self.scope.workspace_id,
                1,
                &KEY,
                through,
                through,
                &manifest
                    .iter()
                    .map(|r| RecordManifestEntry {
                        record_id: r.header.record_id,
                        record_revision: r.header.expected_server_version + 1,
                        workspace_key_version: 1,
                        tombstone: r.header.tombstone,
                    })
                    .collect::<Vec<_>>(),
            )
            .unwrap();
            let changes = records
                .iter()
                .enumerate()
                .map(|(i, r)| PulledRecord {
                    change_seq: after + i as u64 + 1,
                    server_version: r.header.expected_server_version + 1,
                    record: r.clone(),
                })
                .collect::<Vec<_>>();
            self.db
                .apply_sync_v2_notes_contacts(self.scope, after, &changes, &cp, &KEY, &self.root)
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.root);
        }
    }
    fn record(body: &str) -> SealedRecord {
        let h = header(10);
        seal_record(h.clone(), &KEY, &note(&h, body)).unwrap()
    }
    fn cursor(f: &Fixture) -> i64 {
        f.db.connection
            .query_row(
                "SELECT COALESCE(MAX(pull_cursor),0) FROM sync_v2_local_streams",
                [],
                |r| r.get(0),
            )
            .unwrap()
    }
    #[test]
    fn verified_backup_precedes_atomic_apply_and_exact_retry() {
        let mut f = Fixture::new();
        let r = record("Fictional note");
        let b = f.run(0, &[r.clone()], &[r.clone()]).unwrap();
        assert_eq!(recovery::preview(Path::new(&b.path)).unwrap().notes, 0);
        assert_eq!(
            live(&f.db.connection, 10, r.header.record_id)
                .unwrap()
                .unwrap()["body"],
            "Fictional note"
        );
        f.run(0, &[r.clone()], &[r]).unwrap();
        assert_eq!(cursor(&f), 1);
    }
    #[test]
    fn local_edits_are_preserved_and_cursor_does_not_advance() {
        let mut f = Fixture::new();
        let first = record("Baseline");
        f.run(0, &[first.clone()], &[first.clone()]).unwrap();
        f.db.connection
            .execute("UPDATE notes SET body='Local offline change'", [])
            .unwrap();
        let mut h = first.header.clone();
        h.expected_server_version = 1;
        h.mutation_id = Uuid::new_v4();
        let remote = seal_record(h.clone(), &KEY, &note(&h, "Remote competing change")).unwrap();
        assert!(f.run(1, &[remote.clone()], &[remote]).is_err());
        assert_eq!(cursor(&f), 1);
        assert_eq!(
            live(&f.db.connection, 10, first.header.record_id)
                .unwrap()
                .unwrap()["body"],
            "Local offline change"
        );
    }
    #[test]
    fn unavailable_backup_blocks_all_changes() {
        let mut f = Fixture::new();
        f.root = f.root.join("missing");
        let r = record("Never applied");
        assert!(f.run(0, &[r.clone()], &[r.clone()]).is_err());
        assert_eq!(cursor(&f), 0);
        assert!(live(&f.db.connection, 10, r.header.record_id)
            .unwrap()
            .is_none());
        // Fixture changed its destination; remove only the uniquely-created parent.
        std::fs::remove_dir(f.root.parent().unwrap()).unwrap();
    }
    #[test]
    fn invalid_second_reference_rolls_back_whole_page_before_backup() {
        let mut f = Fixture::new();
        let a = record("First");
        let h = header(10);
        let mut n = note(&h, "Second");
        n.fields.as_mut().unwrap()["project_id"] = json!(Uuid::new_v4().to_string());
        let b = seal_record(h, &KEY, &n).unwrap();
        assert!(f.run(0, &[a.clone(), b.clone()], &[a.clone(), b]).is_err());
        assert_eq!(cursor(&f), 0);
        assert!(live(&f.db.connection, 10, a.header.record_id)
            .unwrap()
            .is_none());
        assert_eq!(std::fs::read_dir(&f.root).unwrap().count(), 0);
    }
    #[test]
    fn tombstone_requires_clean_baseline_and_preserves_backup() {
        let mut f = Fixture::new();
        let a = record("Preserved in safety backup");
        f.run(0, &[a.clone()], &[a.clone()]).unwrap();
        let mut h = a.header.clone();
        h.expected_server_version = 1;
        h.mutation_id = Uuid::new_v4();
        h.tombstone = true;
        let b = seal_record(
            h,
            &KEY,
            &RecordContent {
                schema_version: 1,
                fields: None,
            },
        )
        .unwrap();
        let backup = f.run(1, &[b.clone()], &[b]).unwrap();
        assert_eq!(recovery::preview(Path::new(&backup.path)).unwrap().notes, 1);
        assert!(live(&f.db.connection, 10, a.header.record_id)
            .unwrap()
            .is_none());
        assert_eq!(cursor(&f), 2);
    }
    #[test]
    fn contact_adapter_and_required_domain_validation() {
        let mut f = Fixture::new();
        let h = header(11);
        let content = RecordContent {
            schema_version: 1,
            fields: Some(
                json!({"id":h.record_id.to_string(),"name":"Fictional contact","company":null,"email":null,"phone":null,"role":null,"created_at_utc":"2026-10-10T00:00:00Z","updated_at_utc":"2026-10-10T00:00:00Z"}),
            ),
        };
        let r = seal_record(h.clone(), &KEY, &content).unwrap();
        f.run(0, &[r.clone()], &[r]).unwrap();
        assert_eq!(
            live(&f.db.connection, 11, h.record_id).unwrap().unwrap()["name"],
            "Fictional contact"
        );
        let blank = record("   ");
        assert!(f.run(1, &[blank.clone()], &[blank]).is_err());
        assert_eq!(cursor(&f), 1);
    }
    #[test]
    fn live_trigger_failure_rolls_back_staging_too() {
        let mut f = Fixture::new();
        f.db.connection.execute_batch("CREATE TRIGGER refuse_note BEFORE INSERT ON notes BEGIN SELECT RAISE(ABORT,'injected'); END;").unwrap();
        let r = record("Rejected");
        assert!(f.run(0, &[r.clone()], &[r]).is_err());
        assert_eq!(cursor(&f), 0);
        assert_eq!(std::fs::read_dir(&f.root).unwrap().count(), 0);
    }
    #[test]
    fn wrong_manifest_and_unsupported_kind_never_create_backup() {
        let mut f = Fixture::new();
        let r = record("Fictional note");
        let omitted = record("Omitted record");
        assert!(f.run(0, &[r.clone()], &[r.clone(), omitted]).is_err());
        let mut unsupported = r.clone();
        unsupported.header.record_kind = 1;
        assert!(f.run(0, &[unsupported.clone()], &[unsupported]).is_err());
        assert_eq!(cursor(&f), 0);
        assert_eq!(std::fs::read_dir(&f.root).unwrap().count(), 0);
    }
    #[test]
    fn adapter_failure_inside_transaction_rolls_back_checkpoint_and_records() {
        let mut f = Fixture::new();
        let r = record("Fictional note");
        let cp = seal_workspace_checkpoint(
            f.scope.owner_id,
            f.scope.workspace_id,
            1,
            &KEY,
            1,
            1,
            &[RecordManifestEntry {
                record_id: r.header.record_id,
                record_revision: 1,
                workspace_key_version: 1,
                tombstone: false,
            }],
        )
        .unwrap();
        let changes = [PulledRecord {
            change_seq: 1,
            server_version: 1,
            record: r,
        }];
        assert!(f.db.stage_sync_v2_pull_with_apply(f.scope,0,&changes,&cp,&KEY,|tx| {
            tx.execute("INSERT INTO notes(id,body,created_at_utc,updated_at_utc) VALUES('fictional','Rollback','2026-10-10T00:00:00Z','2026-10-10T00:00:00Z')",[]).unwrap();
            Err(refused())
        }).is_err());
        for table in [
            "notes",
            "sync_v2_record_snapshots",
            "sync_v2_checkpoint_anchors",
            "sync_v2_device_acknowledgements",
        ] {
            assert_eq!(
                f.db.connection
                    .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r
                        .get::<_, i64>(0))
                    .unwrap(),
                0
            );
        }
        assert_eq!(cursor(&f), 0);
    }
    #[test]
    fn fractional_utc_order_is_correct_without_normalizing_records() {
        assert!(
            timestamp_order("2026-10-10T00:00:00Z") < timestamp_order("2026-10-10T00:00:00.1Z")
        );
        assert_eq!(
            timestamp_order("2026-10-10T00:00:00.1Z"),
            timestamp_order("2026-10-10T00:00:00.100Z")
        );
        let mut f = Fixture::new();
        let h = header(10);
        let mut content = note(&h, "Fictional note");
        let fields = content.fields.as_mut().unwrap();
        fields["created_at_utc"] = json!("2026-10-10T00:00:00.5Z");
        fields["updated_at_utc"] = json!("2026-10-10T00:00:00Z");
        let r = seal_record(h, &KEY, &content).unwrap();
        assert!(f.run(0, &[r.clone()], &[r]).is_err());
        assert_eq!(cursor(&f), 0);
    }
}

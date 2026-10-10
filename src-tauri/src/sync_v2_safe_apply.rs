//! C10-04C1-C5: dormant, fail-closed metadata and task-link adapters.
//! No transport, document operations, or commands.
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
        7|8|9|12|13 => crate::sync_v2_metadata_adapters::live(connection,kind,Uuid::parse_str(&id).map_err(|_|invalid())?),
        3 | 4 => crate::sync_v2_register_adapters::live(connection,kind,Uuid::parse_str(&id).map_err(|_|invalid())?),
        1 => connection.query_row(
            "SELECT id,number,name,status,phase,custom_phase_name,customer,general_contractor,engineer,project_manager,superintendent,location,start_date,target_date,description,important_notes,project_path,is_pinned,archived_at_utc,created_at_utc,updated_at_utc FROM projects WHERE id=?1",
            [id], |r| Ok(json!({"id":r.get::<_,String>(0)?,"number":r.get::<_,String>(1)?,"name":r.get::<_,String>(2)?,"status":r.get::<_,String>(3)?,"phase":r.get::<_,String>(4)?,"custom_phase_name":r.get::<_,Option<String>>(5)?,"customer":r.get::<_,Option<String>>(6)?,"general_contractor":r.get::<_,Option<String>>(7)?,"engineer":r.get::<_,Option<String>>(8)?,"project_manager":r.get::<_,Option<String>>(9)?,"superintendent":r.get::<_,Option<String>>(10)?,"location":r.get::<_,Option<String>>(11)?,"start_date":r.get::<_,Option<String>>(12)?,"target_date":r.get::<_,Option<String>>(13)?,"description":r.get::<_,Option<String>>(14)?,"important_notes":r.get::<_,Option<String>>(15)?,"project_path":r.get::<_,String>(16)?,"is_pinned":r.get::<_,bool>(17)?,"archived_at_utc":r.get::<_,Option<String>>(18)?,"created_at_utc":r.get::<_,String>(19)?,"updated_at_utc":r.get::<_,String>(20)?})),
        ).optional().map_err(sql),
        2 => connection.query_row(
            "SELECT id,project_id,title,description,priority,status,category,due_date,follow_up_date,waiting_since_utc,waiting_on,related_contact_id,created_at_utc,updated_at_utc FROM tasks WHERE id=?1",
            [id], |r| Ok(json!({"id":r.get::<_,String>(0)?,"project_id":r.get::<_,String>(1)?,"title":r.get::<_,String>(2)?,"description":r.get::<_,Option<String>>(3)?,"priority":r.get::<_,String>(4)?,"status":r.get::<_,String>(5)?,"category":r.get::<_,Option<String>>(6)?,"due_date":r.get::<_,Option<String>>(7)?,"follow_up_date":r.get::<_,Option<String>>(8)?,"waiting_since_utc":r.get::<_,Option<String>>(9)?,"waiting_on":r.get::<_,Option<String>>(10)?,"related_contact_id":r.get::<_,Option<String>>(11)?,"created_at_utc":r.get::<_,String>(12)?,"updated_at_utc":r.get::<_,String>(13)?})),
        ).optional().map_err(sql),
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
    project_root: Option<std::path::PathBuf>,
    relationship: Option<crate::sync_v2_relationship_adapters::Binding>,
    workspace: Uuid,
    expected: Option<Value>,
    conflict: bool,
    portable_reference: Option<Value>,
    subtype: Option<String>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ConflictChoice {
    KeepLocal,
    AcceptRemote,
}
pub(crate) enum PullOutcome {
    Applied(BackupInfo),
    NeedsReview(Vec<Uuid>),
}
#[derive(Debug)]
pub(crate) struct AppliedReceipt {
    pub workspace_id: Uuid,
    pub device_id: Uuid,
    pub checkpoint_counter: u64,
    pub through_change_seq: u64,
    pub checkpoint_digest: [u8; 32],
    pub backup_id: Uuid,
}

fn copy_database(db: &Database) -> AppResult<Database> {
    let mut copy = Connection::open_in_memory().map_err(sql)?;
    Backup::new(&db.connection, &mut copy)
        .map_err(sql)?
        .run_to_completion(64, Duration::from_millis(1), None)
        .map_err(sql)?;
    copy.execute_batch("PRAGMA foreign_keys=ON").map_err(sql)?;
    Ok(Database { connection: copy })
}
fn portable_candidate(
    p: &Prepared,
    local: Option<Value>,
    remote: &crate::sync_v2_record_codec::SealedRecord,
    key: &[u8; 32],
) -> AppResult<crate::sync_v2_record_codec::SealedRecord> {
    let mut fields = local;
    if let Some(f) = &mut fields {
        if matches!(p.kind, 1 | 7 | 8 | 9) {
            let name = if p.kind == 1 {
                "project_path"
            } else {
                "file_path"
            };
            let portable = if p.kind == 1 {
                "portable_root"
            } else {
                "portable_path"
            };
            let path = text(f, name)?.to_owned();
            let reference = if path.starts_with("sitedatum-unresolved:") {
                p.portable_reference.clone().ok_or_else(refused)?
            } else {
                let root = p.project_root.as_deref().ok_or_else(refused)?;
                let relative = Path::new(&path).strip_prefix(root).map_err(|_| refused())?;
                let parts = relative
                    .components()
                    .map(|c| {
                        c.as_os_str()
                            .to_str()
                            .map(str::to_owned)
                            .ok_or_else(refused)
                    })
                    .collect::<AppResult<Vec<_>>>()?;
                json!({"kind":"workspace_relative","components":parts})
            };
            let object = f.as_object_mut().ok_or_else(refused)?;
            object.remove(name);
            object.insert(portable.into(), reference);
        }
    }
    let mut h = remote.header.clone();
    h.mutation_id = Uuid::new_v4();
    h.tombstone = fields.is_none();
    crate::sync_v2_record_codec::seal_record(
        h,
        key,
        &crate::sync_v2_record_codec::RecordContent {
            schema_version: 1,
            fields,
        },
    )
}
fn project_candidate(
    p: &Prepared,
    record: &crate::sync_v2_record_codec::SealedRecord,
    key: &[u8; 32],
) -> AppResult<Option<Value>> {
    let fields = open_record(record, key)?.fields;
    if p.kind == 1 {
        project_fields(fields, p.project_root.as_deref().ok_or_else(refused)?)
    } else if matches!(p.kind, 7 | 8 | 9) {
        crate::sync_v2_metadata_adapters::project_fields(
            fields,
            p.project_root.as_deref().ok_or_else(refused)?,
            p.id,
        )
    } else {
        Ok(fields)
    }
}
fn queue_resolution(
    tx: &Transaction<'_>,
    s: StreamScope,
    remote: &crate::sync_v2_record_codec::SealedRecord,
    resolution: &crate::sync_v2_record_codec::SealedRecord,
) -> AppResult<()> {
    let h = &resolution.header;
    if h.expected_server_version != remote.header.expected_server_version + 1
        || h.record_id != remote.header.record_id
        || h.record_kind != remote.header.record_kind
        || h.owner_id != s.owner_id
        || h.workspace_id != s.workspace_id
    {
        return Err(refused());
    }
    tx.execute("INSERT INTO sync_v2_committed_bases(workspace_id,record_id,server_version,envelope) VALUES(?1,?2,?3,?4)",params![s.workspace_id.to_string(),h.record_id.to_string(),h.expected_server_version as i64,crate::sync_v2_local_queue::encode(remote)?]).map_err(sql)?;
    let bytes = crate::sync_v2_local_queue::encode(resolution)?;
    tx.execute("INSERT INTO sync_v2_outbox(workspace_id,mutation_id,record_id,envelope) VALUES(?1,?2,?3,?4)",params![s.workspace_id.to_string(),h.mutation_id.to_string(),h.record_id.to_string(),bytes]).map_err(sql)?;
    tx.execute(
        "UPDATE sync_v2_record_snapshots SET envelope=?3 WHERE workspace_id=?1 AND record_id=?2",
        params![
            s.workspace_id.to_string(),
            h.record_id.to_string(),
            crate::sync_v2_local_queue::encode(resolution)?
        ],
    )
    .map_err(sql)?;
    Ok(())
}
fn project_fields(mut fields: Option<Value>, root: &Path) -> AppResult<Option<Value>> {
    if let Some(fields) = &mut fields {
        let path = crate::sync_v2_project_paths::project_path(root, &fields["portable_root"])?;
        let input = crate::project::ProjectInput {
            number: text(fields, "number")?.to_owned(),
            name: text(fields, "name")?.to_owned(),
            status: text(fields, "status")?.to_owned(),
            phase: text(fields, "phase")?.to_owned(),
            custom_phase_name: nullable(fields, "custom_phase_name")?.map(str::to_owned),
            customer: nullable(fields, "customer")?.map(str::to_owned),
            general_contractor: nullable(fields, "general_contractor")?.map(str::to_owned),
            engineer: nullable(fields, "engineer")?.map(str::to_owned),
            project_manager: nullable(fields, "project_manager")?.map(str::to_owned),
            superintendent: nullable(fields, "superintendent")?.map(str::to_owned),
            location: nullable(fields, "location")?.map(str::to_owned),
            start_date: nullable(fields, "start_date")?.map(str::to_owned),
            target_date: nullable(fields, "target_date")?.map(str::to_owned),
            description: nullable(fields, "description")?.map(str::to_owned),
            important_notes: nullable(fields, "important_notes")?.map(str::to_owned),
        };
        crate::project::validate_input(&input).map_err(|_| refused())?;
        let object = fields.as_object_mut().ok_or_else(invalid)?;
        object.remove("portable_root");
        object.insert(
            "project_path".to_owned(),
            json!(path.to_str().ok_or_else(refused)?),
        );
    }
    Ok(fields)
}
fn prepare(
    db: &Database,
    scope: StreamScope,
    changes: &[PulledRecord],
    key: &[u8; 32],
) -> AppResult<Vec<Prepared>> {
    prepare_policy(db, scope, changes, key, false)
}
fn prepare_policy(
    db: &Database,
    scope: StreamScope,
    changes: &[PulledRecord],
    key: &[u8; 32],
    allow_conflicts: bool,
) -> AppResult<Vec<Prepared>> {
    let mut identities: std::collections::HashMap<
        Uuid,
        (
            u8,
            Option<String>,
            Option<crate::sync_v2_relationship_adapters::Binding>,
        ),
    > = std::collections::HashMap::new();
    for change in changes {
        let h = &change.record.header;
        let f = open_record(&change.record, key)?.fields;
        let subtype = f
            .as_ref()
            .and_then(|f| f["subtype"].as_str())
            .map(str::to_owned);
        let pair = if matches!(h.record_kind, 5 | 6) {
            f.as_ref()
                .map(|f| crate::sync_v2_relationship_adapters::from_fields(h.record_kind, f))
                .transpose()?
        } else {
            None
        };
        if let Some((kind, old_sub, old_pair)) = identities.get(&h.record_id) {
            if *kind != h.record_kind
                || (subtype.is_some() && old_sub.is_some() && old_sub != &subtype)
                || (pair.is_some() && old_pair.is_some() && old_pair != &pair)
            {
                return Err(refused());
            }
        }
        let entry = identities
            .entry(h.record_id)
            .or_insert((h.record_kind, None, None));
        if subtype.is_some() {
            entry.1 = subtype;
        }
        if pair.is_some() {
            entry.2 = pair;
        }
    }
    let project_root = if changes
        .iter()
        .any(|c| matches!(c.record.header.record_kind, 1 | 7 | 8 | 9))
    {
        Some(crate::sync_v2_project_paths::selected_root(db)?)
    } else {
        None
    };
    changes.iter().map(|change| {
        let h = &change.record.header;
        if h.owner_id != scope.owner_id || h.workspace_id != scope.workspace_id || !(1..=13).contains(&h.record_kind) {
            return Err(refused());
        }
        let mut after = open_record(&change.record,key)?.fields;
        let mut portable_reference=after.as_ref().and_then(|f|f.get(if h.record_kind==1 {"portable_root"} else {"portable_path"})).cloned();
        let mut subtype=after.as_ref().and_then(|f|f["subtype"].as_str()).map(str::to_owned);
        if h.record_kind==1 {after=project_fields(after,project_root.as_deref().ok_or_else(refused)?)?;}
        if matches!(h.record_kind,7|8|9) {after=crate::sync_v2_metadata_adapters::project_fields(after,project_root.as_deref().ok_or_else(refused)?,h.record_id)?;}
        if let Some(fields) = &after {
            // Preserve exact identity rather than silently canonicalizing UUID
            // text during SQL application (SQLite text keys are case-sensitive).
            for field in ["id","project_id","related_contact_id","parent_submittal_id","rfi_id","submittal_id","task_id","entity_id"] {
                if let Some(value)=fields.get(field).and_then(Value::as_str) {
                    if Uuid::parse_str(value).map_err(|_|invalid())?.to_string()!=value {return Err(refused());}
                }
            }
            if matches!(h.record_kind,3|4) {crate::sync_v2_register_adapters::validate(h.record_kind,fields)?;}
            if matches!(h.record_kind,7|8|9|12|13) {crate::sync_v2_metadata_adapters::validate(h.record_kind,fields)?;}
            if h.record_kind==13 {
                let bound:Option<String>=db.connection.query_row("SELECT subtype FROM sync_v2_record_subtypes WHERE workspace_id=?1 AND record_id=?2",params![scope.workspace_id.to_string(),h.record_id.to_string()],|r|r.get(0)).optional().map_err(sql)?;
                if bound.as_deref().is_some_and(|old|Some(old)!=fields["subtype"].as_str()) {return Err(refused());}
            }
            if !matches!(h.record_kind,5|6) {
            let required = match h.record_kind {2=>"title",3=>"subject",7|8=>"display_name",9=>"file_name",10=>"body",12=>"summary",13 if fields["subtype"]=="work_item"=>"title",_=>"name"};
            if fields[required].as_str().is_none_or(|s|s.trim().is_empty()) {
                return Err(refused());
            }
            // Never normalize imported values silently, including timestamps.
            if !matches!(h.record_kind,7|8|12) && timestamp_order(text(fields, "created_at_utc")?)
                > timestamp_order(text(fields, "updated_at_utc")?)
            {
                return Err(refused());
            }
            }
        }
        let relationship = if matches!(h.record_kind,5|6) {
            let stored=crate::sync_v2_relationship_adapters::stored(&db.connection,scope.workspace_id,h.record_id)?;
            let binding=if let Some(fields)=&after {
                let incoming=crate::sync_v2_relationship_adapters::from_fields(h.record_kind,fields)?;
                if stored.as_ref().is_some_and(|b|b!=&incoming) {return Err(refused());}
                // Do not silently claim a pre-existing unmanaged local link.
                if stored.is_none() && crate::sync_v2_relationship_adapters::live(&db.connection,&incoming)?.is_some() {return Err(refused());}
                incoming
            } else {stored.or_else(||changes.iter().find_map(|c| {
                if c.record.header.record_id!=h.record_id {return None;}
                open_record(&c.record,key).ok()?.fields.and_then(|f|crate::sync_v2_relationship_adapters::from_fields(h.record_kind,&f).ok())
            })).ok_or_else(refused)?};
            if binding.kind!=h.record_kind {return Err(refused());}
            Some(binding)
        } else {None};
        let before = if let Some(binding)=&relationship {crate::sync_v2_relationship_adapters::live(&db.connection,binding)?} else {live(&db.connection,h.record_kind,h.record_id)?};
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
                let base:Option<Option<Vec<u8>>>=db.connection.query_row("SELECT envelope FROM sync_v2_committed_bases WHERE workspace_id=?1 AND record_id=?2",params![scope.workspace_id.to_string(),h.record_id.to_string()],|r|r.get(0)).optional().map_err(sql)?;
                let fields=match base {Some(Some(bytes))=>{
                    let committed=decode(&bytes)?;
                    if committed.header.owner_id!=scope.owner_id || committed.header.workspace_id!=scope.workspace_id || committed.header.record_id!=h.record_id || committed.header.record_kind!=h.record_kind {return Err(refused());}
                    open_record(&committed,key)?.fields
                },Some(None)=>None,None=>open_record(&old,key)?.fields};
                if let Some(fields)=&fields {
                    if let Some(reference)=fields.get(if h.record_kind==1 {"portable_root"} else {"portable_path"}) {portable_reference=Some(reference.clone());}
                    if subtype.is_none() {subtype=fields["subtype"].as_str().map(str::to_owned);}
                }
                if h.record_kind==1 {project_fields(fields,project_root.as_deref().ok_or_else(refused)?)?} else if matches!(h.record_kind,7|8|9) {crate::sync_v2_metadata_adapters::project_fields(fields,project_root.as_deref().ok_or_else(refused)?,h.record_id)?} else {fields}
            },
            None => None,
        };
        if let (Some(binding),Some(fields))=(&relationship,&expected) {
            if crate::sync_v2_relationship_adapters::from_fields(h.record_kind,fields)?!=*binding {return Err(refused());}
        }
        // Exact remote redelivery is harmless; divergent local work is never
        // overwritten, even when the outbox was not populated by an adapter.
        let queued:Option<Vec<u8>>=db.connection.query_row("SELECT envelope FROM sync_v2_outbox WHERE workspace_id=?1 AND record_id=?2",params![scope.workspace_id.to_string(),h.record_id.to_string()],|r|r.get(0)).optional().map_err(sql)?;
        let conflict=(before!=expected && before!=after) || (allow_conflicts && queued.as_ref().is_some_and(|b|crate::sync_v2_local_queue::encode(&change.record).ok().as_ref()!=Some(b)));
        if conflict && !allow_conflicts {
            return Err(refused());
        }
        if h.record_kind==13 {
            if let (Some(previous),Some(next))=(&expected,&after) {if previous["subtype"]!=next["subtype"] {return Err(refused());}}
        }
        if h.record_kind==1 {
            if let (Some(before),Some(after))=(&before,&after) {
                if before["project_path"]!=after["project_path"] {return Err(refused());}
            } else if before.is_none() {
                if let Some(after)=&after {
                    // Never silently adopt a pre-existing folder for a new ID.
                    if std::fs::symlink_metadata(text(after,"project_path")?).is_ok() {return Err(refused());}
                }
            }
        }
        if subtype.is_none() {subtype=identities.get(&h.record_id).and_then(|x|x.1.clone());}
        Ok(Prepared {id:h.record_id,kind:h.record_kind,before,after,project_root:project_root.clone(),relationship,workspace:scope.workspace_id,expected,conflict,portable_reference,subtype})
    }).collect::<AppResult<Vec<_>>>().map(|records| {
        // Validate each envelope above; apply only its final revision. Stream
        // staging independently checks the complete ordered revision chain.
        let mut final_records=std::collections::BTreeMap::new();
        for record in records {final_records.insert(record.id,record);}
        final_records.into_values().collect()
    })
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
    // Allows a submittal child before its parent in the verified stream. Final
    // FK and ancestry validation still runs before this transaction can commit.
    tx.execute_batch("PRAGMA defer_foreign_keys=ON;")
        .map_err(sql)?;
    // Check every baseline before any adapter changes dependency rows.
    for record in records {
        let current = if let Some(binding) = &record.relationship {
            crate::sync_v2_relationship_adapters::live(tx, binding)?
        } else {
            live(tx, record.kind, record.id)?
        };
        if current != record.before {
            return Err(refused());
        }
    }
    // Provider change order is verified by staging; SQL has a separate dependency
    // order. Stream cursor order is never changed by this local application sort.
    let mut ordered = records.iter().collect::<Vec<_>>();
    ordered.sort_by_key(|r| {
        (
            match (r.kind, r.after.is_some()) {
                (5 | 6, false) => -2,
                (7 | 8, false) => -2,
                (1, true) => -1,
                (11, true) => 0,
                (2, true) => 1,
                (3 | 4, true) => 2,
                (10, true) => 2,
                (5 | 6, true) => 3,
                (7 | 8 | 9 | 12 | 13, true) => 3,
                (10, false) => 4,
                (3 | 4, false) => 5,
                (9 | 12 | 13, false) => 4,
                (2, false) => 6,
                (11, false) => 7,
                (1, false) => 8,
                _ => 9,
            },
            if r.kind == 4 && r.after.is_none() {
                let mut depth = 0i32;
                let mut parent = r
                    .before
                    .as_ref()
                    .and_then(|f| f["parent_submittal_id"].as_str());
                let mut seen = std::collections::HashSet::new();
                while let Some(id) = parent {
                    if !seen.insert(id) {
                        break;
                    }
                    if let Some(p) = records.iter().find(|p| p.id.to_string() == id) {
                        depth += 1;
                        parent = p
                            .before
                            .as_ref()
                            .and_then(|f| f["parent_submittal_id"].as_str());
                    } else {
                        break;
                    }
                }
                -depth
            } else {
                0
            },
        )
    });
    for record in ordered {
        let id = record.id.to_string();
        match (record.kind, &record.after) {
            (7 | 8 | 9 | 12 | 13, fields) => {
                if record.kind == 13 {
                    if let Some(subtype) = &record.subtype {
                        tx.execute("INSERT INTO sync_v2_record_subtypes VALUES(?1,?2,?3) ON CONFLICT(workspace_id,record_id) DO NOTHING",params![record.workspace.to_string(),id,subtype]).map_err(sql)?;
                        let bound:String=tx.query_row("SELECT subtype FROM sync_v2_record_subtypes WHERE workspace_id=?1 AND record_id=?2",params![record.workspace.to_string(),id],|r|r.get(0)).map_err(sql)?;
                        if bound != *subtype {
                            return Err(refused());
                        }
                    }
                }
                if let Some(f) = fields {
                    if matches!(record.kind, 7 | 8 | 9)
                        && !text(f, "file_path")?.starts_with("sitedatum-unresolved:")
                    {
                        let root = record.project_root.as_deref().ok_or_else(refused)?;
                        let path = Path::new(text(f, "file_path")?);
                        crate::sync_v2_project_paths::verify_reference_path(root, path)?;
                    }
                }
                crate::sync_v2_metadata_adapters::apply(
                    tx,
                    record.kind,
                    record.id,
                    fields.as_ref(),
                )?;
            }
            (5 | 6, fields) => {
                let binding = record.relationship.as_ref().ok_or_else(refused)?;
                crate::sync_v2_relationship_adapters::bind(
                    tx,
                    record.workspace,
                    record.id,
                    binding,
                )?;
                crate::sync_v2_relationship_adapters::apply(tx, binding, fields.as_ref())?;
            }
            (3 | 4, Some(f)) => crate::sync_v2_register_adapters::upsert(tx, record.kind, f)?,
            (3 | 4, None) => crate::sync_v2_register_adapters::delete(tx, record.kind, record.id)?,
            (1, Some(f)) => {
                crate::sync_v2_project_paths::verify_local_path(
                    record.project_root.as_deref().ok_or_else(refused)?,
                    Path::new(text(f, "project_path")?),
                )?;
                if record.before.is_none()
                    && std::fs::symlink_metadata(text(f, "project_path")?).is_ok()
                {
                    return Err(refused());
                }
                let target = text(f, "project_path")?.replace('/', "\\").to_uppercase();
                let mut paths = tx
                    .prepare("SELECT project_path FROM projects WHERE id<>?1")
                    .map_err(sql)?;
                let mut collision = false;
                for path in paths
                    .query_map([&id], |r| r.get::<_, String>(0))
                    .map_err(sql)?
                {
                    if path.map_err(sql)?.replace('/', "\\").to_uppercase() == target {
                        collision = true;
                    }
                }
                if collision {
                    return Err(refused());
                }
                tx.execute("INSERT INTO projects(id,number,name,status,phase,custom_phase_name,customer,general_contractor,engineer,project_manager,superintendent,location,start_date,target_date,description,important_notes,project_path,is_pinned,archived_at_utc,created_at_utc,updated_at_utc) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,?21) ON CONFLICT(id) DO UPDATE SET number=excluded.number,name=excluded.name,status=excluded.status,phase=excluded.phase,custom_phase_name=excluded.custom_phase_name,customer=excluded.customer,general_contractor=excluded.general_contractor,engineer=excluded.engineer,project_manager=excluded.project_manager,superintendent=excluded.superintendent,location=excluded.location,start_date=excluded.start_date,target_date=excluded.target_date,description=excluded.description,important_notes=excluded.important_notes,project_path=excluded.project_path,is_pinned=excluded.is_pinned,archived_at_utc=excluded.archived_at_utc,created_at_utc=excluded.created_at_utc,updated_at_utc=excluded.updated_at_utc",params![id,text(f,"number")?,text(f,"name")?,text(f,"status")?,text(f,"phase")?,nullable(f,"custom_phase_name")?,nullable(f,"customer")?,nullable(f,"general_contractor")?,nullable(f,"engineer")?,nullable(f,"project_manager")?,nullable(f,"superintendent")?,nullable(f,"location")?,nullable(f,"start_date")?,nullable(f,"target_date")?,nullable(f,"description")?,nullable(f,"important_notes")?,text(f,"project_path")?,f["is_pinned"].as_bool().ok_or_else(invalid)?,nullable(f,"archived_at_utc")?,text(f,"created_at_utc")?,text(f,"updated_at_utc")?]).map_err(sql)?;
            }
            (2, Some(f)) => {
                let cross_project:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM rfi_task_relationships l JOIN rfis r ON r.id=l.rfi_id WHERE l.task_id=?1 AND r.project_id<>?2) OR EXISTS(SELECT 1 FROM submittal_task_relationships l JOIN submittals s ON s.id=l.submittal_id WHERE l.task_id=?1 AND s.project_id<>?2)",params![id,text(f,"project_id")?],|r|r.get(0)).map_err(sql)?;
                if cross_project {
                    return Err(refused());
                }
                if let Some(contact) = nullable(f, "related_contact_id")? {
                    let exists: bool = tx
                        .query_row(
                            "SELECT EXISTS(SELECT 1 FROM contacts WHERE id=?1)",
                            [contact],
                            |r| r.get(0),
                        )
                        .map_err(sql)?;
                    if !exists {
                        return Err(refused());
                    }
                }
                tx.execute("INSERT INTO tasks(id,project_id,title,description,priority,status,category,due_date,follow_up_date,waiting_since_utc,waiting_on,related_contact_id,created_at_utc,updated_at_utc) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14) ON CONFLICT(id) DO UPDATE SET project_id=excluded.project_id,title=excluded.title,description=excluded.description,priority=excluded.priority,status=excluded.status,category=excluded.category,due_date=excluded.due_date,follow_up_date=excluded.follow_up_date,waiting_since_utc=excluded.waiting_since_utc,waiting_on=excluded.waiting_on,related_contact_id=excluded.related_contact_id,created_at_utc=excluded.created_at_utc,updated_at_utc=excluded.updated_at_utc",params![id,text(f,"project_id")?,text(f,"title")?,nullable(f,"description")?,text(f,"priority")?,text(f,"status")?,nullable(f,"category")?,nullable(f,"due_date")?,nullable(f,"follow_up_date")?,nullable(f,"waiting_since_utc")?,nullable(f,"waiting_on")?,nullable(f,"related_contact_id")?,text(f,"created_at_utc")?,text(f,"updated_at_utc")?]).map_err(sql)?;
            }
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
            (2, None) => {
                // RESTRICT relationships prevent orphaning linked RFI/submittal
                // work unless an explicit link tombstone already removed it.
                tx.execute("DELETE FROM tasks WHERE id=?1", [id])
                    .map_err(sql)?;
            }
            (1, None) => {
                tx.execute("DELETE FROM projects WHERE id=?1", [id])
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
    if records.iter().any(|r| matches!(r.kind, 3 | 4 | 5 | 6)) {
        crate::sync_v2_register_adapters::validate_relationships(tx)?;
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
    /// Caller-selected local activity: assign a stable random envelope UUID to
    /// older hex-keyed history without renaming or modifying the original row.
    pub(crate) fn seal_sync_v2_existing_activity(
        &mut self,
        s: StreamScope,
        local_id: &str,
        key_version: u32,
        key: &[u8; 32],
    ) -> AppResult<crate::sync_v2_record_codec::SealedRecord> {
        let tx = self.connection.transaction().map_err(sql)?;
        crate::sync_v2_local_queue::scope(&tx, s)?;
        let mapped:Option<String>=tx.query_row("SELECT record_id FROM sync_v2_activity_bindings WHERE workspace_id=?1 AND local_id=?2",params![s.workspace_id.to_string(),local_id],|r|r.get(0)).optional().map_err(sql)?;
        let id = if let Some(id) = mapped {
            Uuid::parse_str(&id).map_err(|_| refused())?
        } else {
            let exists: bool = tx
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM activity_events WHERE id=?1)",
                    [local_id],
                    |r| r.get(0),
                )
                .map_err(sql)?;
            if !exists {
                return Err(refused());
            }
            let id = Uuid::parse_str(local_id)
                .ok()
                .filter(|id| {
                    id.to_string() == local_id
                        && id.get_variant() == uuid::Variant::RFC4122
                        && (1..=5).contains(&id.get_version_num())
                })
                .unwrap_or_else(Uuid::new_v4);
            tx.execute(
                "INSERT INTO sync_v2_activity_bindings VALUES(?1,?2,?3)",
                params![s.workspace_id.to_string(), id.to_string(), local_id],
            )
            .map_err(sql)?;
            id
        };
        let fields = crate::sync_v2_metadata_adapters::live(&tx, 12, id)?.ok_or_else(refused)?;
        let version:Option<i64>=tx.query_row("SELECT server_version FROM sync_v2_record_snapshots WHERE workspace_id=?1 AND record_id=?2",params![s.workspace_id.to_string(),id.to_string()],|r|r.get(0)).optional().map_err(sql)?;
        let record = crate::sync_v2_record_codec::seal_record(
            crate::sync_v2_record_codec::RecordHeader {
                owner_id: s.owner_id,
                workspace_id: s.workspace_id,
                record_id: id,
                record_kind: 12,
                mutation_id: Uuid::new_v4(),
                expected_server_version: version.unwrap_or(0) as u64,
                workspace_key_version: key_version,
                protocol_version: 1,
                tombstone: false,
            },
            key,
            &crate::sync_v2_record_codec::RecordContent {
                schema_version: 1,
                fields: Some(fields),
            },
        )?;
        tx.commit().map_err(sql)?;
        Ok(record)
    }
    /// Explicit bootstrap ownership of a caller-selected existing task link.
    /// No row adoption occurs in ordinary pulled-record application.
    pub(crate) fn seal_sync_v2_existing_link(
        &mut self,
        s: StreamScope,
        kind: u8,
        register: Uuid,
        task: Uuid,
        key_version: u32,
        key: &[u8; 32],
    ) -> AppResult<crate::sync_v2_record_codec::SealedRecord> {
        let binding = crate::sync_v2_relationship_adapters::Binding {
            kind,
            register_id: register.to_string(),
            task_id: task.to_string(),
        };
        let tx = self.connection.transaction().map_err(sql)?;
        crate::sync_v2_local_queue::scope(&tx, s)?;
        let id:Option<String>=tx.query_row("SELECT record_id FROM sync_v2_relationship_bindings WHERE workspace_id=?1 AND record_kind=?2 AND register_id=?3 AND task_id=?4",params![s.workspace_id.to_string(),kind,binding.register_id,binding.task_id],|r|r.get(0)).optional().map_err(sql)?;
        let id = id
            .map(|id| Uuid::parse_str(&id).map_err(|_| refused()))
            .transpose()?
            .unwrap_or_else(Uuid::new_v4);
        let fields =
            crate::sync_v2_relationship_adapters::live(&tx, &binding)?.ok_or_else(refused)?;
        crate::sync_v2_relationship_adapters::bind(&tx, s.workspace_id, id, &binding)?;
        let version:Option<i64>=tx.query_row("SELECT server_version FROM sync_v2_record_snapshots WHERE workspace_id=?1 AND record_id=?2",params![s.workspace_id.to_string(),id.to_string()],|r|r.get(0)).optional().map_err(sql)?;
        let record = crate::sync_v2_record_codec::seal_record(
            crate::sync_v2_record_codec::RecordHeader {
                owner_id: s.owner_id,
                workspace_id: s.workspace_id,
                record_id: id,
                record_kind: kind,
                mutation_id: Uuid::new_v4(),
                expected_server_version: version.unwrap_or(0) as u64,
                workspace_key_version: key_version,
                protocol_version: 1,
                tombstone: false,
            },
            key,
            &crate::sync_v2_record_codec::RecordContent {
                schema_version: 1,
                fields: Some(fields),
            },
        )?;
        tx.commit().map_err(sql)?;
        Ok(record)
    }
    /// Only this checked reader may provide a future transport an applied proof.
    /// Raw staging rows are not applied receipts and hosted compaction is not enabled.
    pub(crate) fn applied_sync_v2_receipt(
        &self,
        s: StreamScope,
        key: &[u8; 32],
        backups: &Path,
    ) -> AppResult<Option<AppliedReceipt>> {
        let pending:bool=self.connection.query_row("SELECT EXISTS(SELECT 1 FROM sync_v2_outbox WHERE workspace_id=?1) OR EXISTS(SELECT 1 FROM sync_v2_record_conflicts WHERE workspace_id=?1 AND resolved_counter IS NULL)",[s.workspace_id.to_string()],|r|r.get(0)).map_err(sql)?;
        if pending {
            return Ok(None);
        }
        let row:Option<(i64,i64,Vec<u8>,String)>=self.connection.query_row("SELECT r.checkpoint_counter,r.through_change_seq,r.checkpoint_digest,r.backup_id FROM sync_v2_applied_receipts r JOIN sync_v2_local_streams s USING(workspace_id) JOIN sync_v2_checkpoint_anchors a USING(workspace_id) WHERE r.workspace_id=?1 AND s.owner_id=?2 AND s.device_id=?3 AND r.device_id=s.device_id AND r.checkpoint_counter=a.checkpoint_counter AND r.through_change_seq=a.through_change_seq AND r.checkpoint_digest=a.checkpoint_digest",params![s.workspace_id.to_string(),s.owner_id.to_string(),s.device_id.to_string()],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?))).optional().map_err(sql)?;
        let Some((counter, through, digest, backup)) = row else {
            return Ok(None);
        };
        let backup_id = Uuid::parse_str(&backup).map_err(|_| refused())?;
        let prefix = format!("sync-v2-pre-apply-{backup_id}-");
        let found = std::fs::read_dir(backups)
            .map_err(|_| refused())?
            .filter_map(Result::ok)
            .any(|entry| {
                entry
                    .file_name()
                    .to_str()
                    .is_some_and(|name| name.starts_with(&prefix) && name.ends_with(".sqlite3"))
                    && recovery::preview(&entry.path()).is_ok()
            });
        if !found {
            return Ok(None);
        }
        let mut statement=self.connection.prepare("SELECT server_version,envelope FROM sync_v2_record_snapshots WHERE workspace_id=?1").map_err(sql)?;
        let records = statement
            .query_map([s.workspace_id.to_string()], |r| {
                Ok((r.get::<_, i64>(0)?, r.get::<_, Vec<u8>>(1)?))
            })
            .map_err(sql)?
            .map(|row| {
                let (version, bytes) = row.map_err(sql)?;
                Ok(PulledRecord {
                    change_seq: 1,
                    server_version: version as u64,
                    record: decode(&bytes)?,
                })
            })
            .collect::<AppResult<Vec<_>>>()?;
        let prepared = match prepare(self, s, &records, key) {
            Ok(prepared) => prepared,
            Err(_) => return Ok(None),
        };
        if prepared.iter().any(|p| p.before != p.after) {
            return Ok(None);
        }
        Ok(Some(AppliedReceipt {
            workspace_id: s.workspace_id,
            device_id: s.device_id,
            checkpoint_counter: counter as u64,
            through_change_seq: through as u64,
            checkpoint_digest: digest.try_into().map_err(|_| refused())?,
            backup_id,
        }))
    }
    /// Authenticate the entire prospective hosted set before retaining any
    /// conflict. No automatic choice and no live/cursor change on NeedsReview.
    pub(crate) fn apply_sync_v2_reviewed_records(
        &mut self,
        s: StreamScope,
        after: u64,
        changes: &[PulledRecord],
        cp: &SealedWorkspaceCheckpoint,
        key: &[u8; 32],
        backups: &Path,
    ) -> AppResult<PullOutcome> {
        let mut prepared = prepare_policy(self, s, changes, key, true)?;
        let final_remote = changes
            .iter()
            .map(|c| (c.record.header.record_id, c))
            .collect::<std::collections::BTreeMap<_, _>>();
        // Remote domain/reference graph and whole manifest must validate even
        // when local candidates differ. This private rehearsal never changes live work.
        let mut candidate = copy_database(self)?;
        candidate.stage_sync_v2_pull_options(s, after, changes, cp, key, true, |tx| {
            apply(tx, &prepared)
        })?;
        let mut choices = std::collections::BTreeMap::new();
        let mut unresolved = Vec::new();
        let tx = self.connection.transaction().map_err(sql)?;
        crate::sync_v2_local_queue::scope(&tx, s)?;
        for p in prepared.iter().filter(|p| p.conflict) {
            let remote = &final_remote.get(&p.id).ok_or_else(refused)?.record;
            let remote_bytes = crate::sync_v2_local_queue::encode(remote)?;
            let old:Option<(Vec<u8>,Vec<u8>,Option<String>,Option<Vec<u8>>) >=tx.query_row("SELECT local_envelope,remote_envelope,choice,resolution_envelope FROM sync_v2_record_conflicts WHERE workspace_id=?1 AND record_id=?2 AND checkpoint_counter=?3",params![s.workspace_id.to_string(),p.id.to_string(),cp.checkpoint_counter as i64],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?))).optional().map_err(sql)?;
            if let Some((local, old_remote, choice, resolution)) = old {
                if old_remote != remote_bytes
                    || project_candidate(p, &decode(&local)?, key)? != p.before
                {
                    return Err(refused());
                }
                match (choice, resolution) {
                    (Some(choice), Some(bytes)) => {
                        choices.insert(p.id, (choice, decode(&bytes)?));
                    }
                    (None, None) => unresolved.push(p.id),
                    _ => return Err(refused()),
                }
            } else {
                let local = portable_candidate(p, p.before.clone(), remote, key)?;
                let baseline = portable_candidate(p, p.expected.clone(), remote, key)?;
                let pending:Option<Vec<u8>>=tx.query_row("SELECT envelope FROM sync_v2_outbox WHERE workspace_id=?1 AND record_id=?2",params![s.workspace_id.to_string(),p.id.to_string()],|r|r.get(0)).optional().map_err(sql)?;
                tx.execute("INSERT INTO sync_v2_record_conflicts(workspace_id,record_id,checkpoint_counter,local_envelope,remote_envelope,baseline_envelope,pending_envelope) VALUES(?1,?2,?3,?4,?5,?6,?7)",params![s.workspace_id.to_string(),p.id.to_string(),cp.checkpoint_counter as i64,crate::sync_v2_local_queue::encode(&local)?,remote_bytes,crate::sync_v2_local_queue::encode(&baseline)?,pending]).map_err(sql)?;
                unresolved.push(p.id);
            }
        }
        tx.commit().map_err(sql)?;
        if !unresolved.is_empty() {
            return Ok(PullOutcome::NeedsReview(unresolved));
        }
        for p in &mut prepared {
            if let Some((choice, resolution)) = choices.get(&p.id) {
                if choice == "keep_local" {
                    p.after = project_candidate(p, resolution, key)?;
                } else if choice != "accept_remote" {
                    return Err(refused());
                }
            }
        }
        let finalize = |tx: &Transaction<'_>, backup_id: Uuid| -> AppResult<()> {
            apply(tx, &prepared)?;
            for p in &prepared {
                let remote = &final_remote.get(&p.id).ok_or_else(refused)?.record;
                // Replaced pending work is retained in encrypted conflict history.
                // Restore the committed remote baseline before queuing a choice.
                let pending:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM sync_v2_outbox WHERE workspace_id=?1 AND record_id=?2)",params![s.workspace_id.to_string(),p.id.to_string()],|r|r.get(0)).map_err(sql)?;
                if pending {
                    if !choices.contains_key(&p.id) && crate::sync_v2_local_queue::encode(remote)?!=tx.query_row::<Vec<u8>,_,_>("SELECT envelope FROM sync_v2_outbox WHERE workspace_id=?1 AND record_id=?2",params![s.workspace_id.to_string(),p.id.to_string()],|r|r.get(0)).map_err(sql)? {return Err(refused());}
                    tx.execute(
                        "DELETE FROM sync_v2_outbox WHERE workspace_id=?1 AND record_id=?2",
                        params![s.workspace_id.to_string(), p.id.to_string()],
                    )
                    .map_err(sql)?;
                }
                tx.execute(
                    "DELETE FROM sync_v2_committed_bases WHERE workspace_id=?1 AND record_id=?2",
                    params![s.workspace_id.to_string(), p.id.to_string()],
                )
                .map_err(sql)?;
                tx.execute("UPDATE sync_v2_record_snapshots SET server_version=?3,envelope=?4 WHERE workspace_id=?1 AND record_id=?2",params![s.workspace_id.to_string(),p.id.to_string(),remote.header.expected_server_version as i64+1,crate::sync_v2_local_queue::encode(remote)?]).map_err(sql)?;
                if let Some((choice, resolution)) = choices.get(&p.id) {
                    if choice == "keep_local" {
                        queue_resolution(tx, s, remote, resolution)?;
                    }
                    tx.execute("UPDATE sync_v2_record_conflicts SET resolved_counter=?3 WHERE workspace_id=?1 AND record_id=?2 AND checkpoint_counter<=?3 AND resolved_counter IS NULL",params![s.workspace_id.to_string(),p.id.to_string(),cp.checkpoint_counter as i64]).map_err(sql)?;
                }
            }
            tx.execute(
                "DELETE FROM sync_v2_pull_parts WHERE workspace_id=?1",
                [s.workspace_id.to_string()],
            )
            .map_err(sql)?;
            tx.execute(
                "DELETE FROM sync_v2_pull_batches WHERE workspace_id=?1",
                [s.workspace_id.to_string()],
            )
            .map_err(sql)?;
            applied_receipt(tx, s, backup_id)
        };
        let mut candidate = copy_database(self)?;
        candidate.stage_sync_v2_pull_options(s, after, changes, cp, key, true, |tx| {
            finalize(tx, Uuid::new_v4())
        })?;
        let backup_id = Uuid::new_v4();
        let backup = recovery::create(self, backups, &format!("sync-v2-pre-apply-{backup_id}"))?;
        self.stage_sync_v2_pull_options(s, after, changes, cp, key, true, |tx| {
            finalize(tx, backup_id)
        })?;
        Ok(PullOutcome::Applied(backup))
    }
    /// Explicit per-record decision; callers supply the exact checkpoint and
    /// choice. Repeated same choices reuse the exact persisted mutation bytes.
    pub(crate) fn choose_sync_v2_conflict(
        &mut self,
        s: StreamScope,
        id: Uuid,
        counter: u64,
        choice: ConflictChoice,
        key: &[u8; 32],
    ) -> AppResult<()> {
        if counter == 0 || counter > crate::sync_v2_record_codec::MAX_SAFE_INTEGER {
            return Err(refused());
        }
        let tx = self.connection.transaction().map_err(sql)?;
        crate::sync_v2_local_queue::scope(&tx, s)?;
        let (local,remote,old):(Vec<u8>,Vec<u8>,Option<String>)=tx.query_row("SELECT local_envelope,remote_envelope,choice FROM sync_v2_record_conflicts WHERE workspace_id=?1 AND record_id=?2 AND checkpoint_counter=?3",params![s.workspace_id.to_string(),id.to_string(),counter as i64],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).map_err(sql)?;
        let label = match choice {
            ConflictChoice::KeepLocal => "keep_local",
            ConflictChoice::AcceptRemote => "accept_remote",
        };
        if let Some(old) = old {
            if old != label {
                return Err(refused());
            }
            return Ok(());
        }
        let remote = decode(&remote)?;
        let local = decode(&local)?;
        if remote.header.owner_id != s.owner_id
            || remote.header.workspace_id != s.workspace_id
            || remote.header.record_id != id
            || local.header.record_id != id
            || local.header.owner_id != s.owner_id
            || local.header.workspace_id != s.workspace_id
            || local.header.record_kind != remote.header.record_kind
            || local.header.workspace_key_version != remote.header.workspace_key_version
        {
            return Err(refused());
        }
        open_record(&remote, key)?;
        let resolution = if choice == ConflictChoice::KeepLocal {
            let mut h = remote.header.clone();
            h.expected_server_version += 1;
            h.mutation_id = Uuid::new_v4();
            h.tombstone = local.header.tombstone;
            crate::sync_v2_record_codec::seal_record(h, key, &open_record(&local, key)?)?
        } else {
            remote
        };
        tx.execute("UPDATE sync_v2_record_conflicts SET choice=?4,resolution_envelope=?5 WHERE workspace_id=?1 AND record_id=?2 AND checkpoint_counter=?3",params![s.workspace_id.to_string(),id.to_string(),counter as i64,label,crate::sync_v2_local_queue::encode(&resolution)?]).map_err(sql)?;
        tx.commit().map_err(sql)
    }
    pub(crate) fn apply_buffered_sync_v2_records(
        &mut self,
        s: StreamScope,
        key: &[u8; 32],
        backups: &Path,
    ) -> AppResult<PullOutcome> {
        let (after, changes, cp) = self.buffered_sync_v2_batch(s)?;
        self.apply_sync_v2_reviewed_records(s, after, &changes, &cp, key, backups)
    }
    /// Dormant mutation entry point: live rows and exact encrypted retry state
    /// either commit together or neither commits. No ordinary command enables it.
    pub(crate) fn mutate_sync_v2_live(
        &mut self,
        scope: StreamScope,
        record: &crate::sync_v2_record_codec::SealedRecord,
        key: &[u8; 32],
    ) -> AppResult<()> {
        crate::sync_v2_record_codec::validate_header(&record.header)?;
        let change = PulledRecord {
            change_seq: 1,
            server_version: record.header.expected_server_version + 1,
            record: record.clone(),
        };
        let prepared = prepare(self, scope, &[change], key)?;
        self.stage_sync_v2_mutation_with_apply(scope, record, key, |tx| apply(tx, &prepared))
    }
    /// Exclusive local DB access is required throughout preflight/backup/apply.
    /// Only a complete authenticated page is accepted. No applied-device receipt
    /// is issued, no file is touched, and no network/consent gate is enabled.
    pub(crate) fn apply_sync_v2_supported_records(
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
        let backup_id = Uuid::new_v4();
        let backup = recovery::create(
            self,
            backup_directory,
            &format!("sync-v2-pre-apply-{backup_id}"),
        )?;
        self.stage_sync_v2_pull_with_apply(scope, after, changes, checkpoint, key, |tx| {
            apply(tx, &prepared)?;
            applied_receipt(tx, scope, backup_id)
        })?;
        Ok(backup)
    }
}

fn applied_receipt(tx: &Transaction<'_>, scope: StreamScope, backup: Uuid) -> AppResult<()> {
    let pending: bool = tx
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM sync_v2_outbox WHERE workspace_id=?1) OR EXISTS(SELECT 1 FROM sync_v2_record_conflicts WHERE workspace_id=?1 AND resolved_counter IS NULL)",
            [scope.workspace_id.to_string()],
            |r| r.get(0),
        )
        .map_err(sql)?;
    if pending {
        tx.execute(
            "DELETE FROM sync_v2_applied_receipts WHERE workspace_id=?1",
            [scope.workspace_id.to_string()],
        )
        .map_err(sql)?;
        return Ok(());
    }
    tx.execute("INSERT INTO sync_v2_applied_receipts(workspace_id,device_id,checkpoint_counter,checkpoint_digest,through_change_seq,backup_id) SELECT workspace_id,device_id,checkpoint_counter,checkpoint_digest,through_change_seq,?2 FROM sync_v2_device_acknowledgements WHERE workspace_id=?1 ON CONFLICT(workspace_id) DO UPDATE SET device_id=excluded.device_id,checkpoint_counter=excluded.checkpoint_counter,checkpoint_digest=excluded.checkpoint_digest,through_change_seq=excluded.through_change_seq,backup_id=excluded.backup_id",params![scope.workspace_id.to_string(),backup.to_string()]).map_err(sql)?;
    Ok(())
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
                .apply_sync_v2_supported_records(self.scope, after, &changes, &cp, &KEY, &self.root)
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
        unsupported.header.record_kind = 7;
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
    fn project(f: &Fixture) -> Uuid {
        let id = Uuid::new_v4();
        f.db.connection.execute("INSERT INTO projects(id,number,name,project_path,created_at_utc,updated_at_utc) VALUES(?1,'FICTIONAL','Fictional local project','C:\\Fictional\\Project','2026-10-10T00:00:00Z','2026-10-10T00:00:00Z')",[id.to_string()]).unwrap();
        id
    }
    fn task_record(project: Uuid, contact: Option<Uuid>) -> SealedRecord {
        let h = header(2);
        let fields = json!({"id":h.record_id.to_string(),"project_id":project.to_string(),"title":"Fictional task","description":null,"priority":"high","status":"waiting","category":null,"due_date":"2026-10-15","follow_up_date":"2026-10-12","waiting_since_utc":"2026-10-10T00:00:00Z","waiting_on":"Fictional response","related_contact_id":contact.map(|c|c.to_string()),"created_at_utc":"2026-10-10T00:00:00Z","updated_at_utc":"2026-10-10T00:00:00Z"});
        seal_record(
            h,
            &KEY,
            &RecordContent {
                schema_version: 1,
                fields: Some(fields),
            },
        )
        .unwrap()
    }
    fn contact_record() -> SealedRecord {
        let h = header(11);
        seal_record(h.clone(),&KEY,&RecordContent {schema_version:1,fields:Some(json!({"id":h.record_id.to_string(),"name":"Fictional contact","company":null,"email":null,"phone":null,"role":null,"created_at_utc":"2026-10-10T00:00:00Z","updated_at_utc":"2026-10-10T00:00:00Z"}))}).unwrap()
    }
    fn tombstone(record: &SealedRecord) -> SealedRecord {
        let mut h = record.header.clone();
        h.expected_server_version += 1;
        h.mutation_id = Uuid::new_v4();
        h.tombstone = true;
        seal_record(
            h,
            &KEY,
            &RecordContent {
                schema_version: 1,
                fields: None,
            },
        )
        .unwrap()
    }
    #[test]
    fn task_contact_dependency_order_preserves_authenticated_stream_order() {
        let mut f = Fixture::new();
        let p = project(&f);
        let c = contact_record();
        let t = task_record(p, Some(c.header.record_id));
        let backup = f
            .run(0, &[t.clone(), c.clone()], &[t.clone(), c.clone()])
            .unwrap();
        assert_eq!(recovery::preview(Path::new(&backup.path)).unwrap().tasks, 0);
        assert_eq!(
            live(&f.db.connection, 2, t.header.record_id).unwrap(),
            open_record(&t, &KEY).unwrap().fields
        );
        assert_eq!(cursor(&f), 2);
        // Contact deletion is received first, but the dependent task is deleted
        // first locally. Both encrypted tombstones remain in the manifest.
        let cd = tombstone(&c);
        let td = tombstone(&t);
        let backup = f.run(2, &[cd.clone(), td.clone()], &[td, cd]).unwrap();
        assert_eq!(recovery::preview(Path::new(&backup.path)).unwrap().tasks, 1);
        assert!(live(&f.db.connection, 2, t.header.record_id)
            .unwrap()
            .is_none());
        assert!(live(&f.db.connection, 11, c.header.record_id)
            .unwrap()
            .is_none());
        assert_eq!(cursor(&f), 4);
    }
    #[test]
    fn unknown_project_or_contact_refuses_entire_page() {
        let mut f = Fixture::new();
        let note = record("Must not partially apply");
        let missing = task_record(Uuid::new_v4(), None);
        assert!(f
            .run(
                0,
                &[note.clone(), missing.clone()],
                &[note.clone(), missing]
            )
            .is_err());
        let p = project(&f);
        let missing = task_record(p, Some(Uuid::new_v4()));
        assert!(f
            .run(
                0,
                &[note.clone(), missing.clone()],
                &[note.clone(), missing]
            )
            .is_err());
        assert_eq!(cursor(&f), 0);
        assert!(live(&f.db.connection, 10, note.header.record_id)
            .unwrap()
            .is_none());
        assert_eq!(std::fs::read_dir(&f.root).unwrap().count(), 0);
    }
    #[test]
    fn local_task_edit_is_never_overwritten_by_remote_revision() {
        let mut f = Fixture::new();
        let p = project(&f);
        let first = task_record(p, None);
        f.run(0, &[first.clone()], &[first.clone()]).unwrap();
        f.db.connection
            .execute("UPDATE tasks SET title='Local offline task'", [])
            .unwrap();
        let mut h = first.header.clone();
        h.expected_server_version = 1;
        h.mutation_id = Uuid::new_v4();
        let mut content = open_record(&first, &KEY).unwrap();
        content.fields.as_mut().unwrap()["title"] = json!("Remote edit");
        let remote = seal_record(h, &KEY, &content).unwrap();
        assert!(f.run(1, &[remote.clone()], &[remote]).is_err());
        assert_eq!(cursor(&f), 1);
        assert_eq!(
            live(&f.db.connection, 2, first.header.record_id)
                .unwrap()
                .unwrap()["title"],
            "Local offline task"
        );
    }
    #[test]
    fn referenced_contact_cannot_be_tombstoned_without_detaching_task() {
        let mut f = Fixture::new();
        let p = project(&f);
        let c = contact_record();
        let t = task_record(p, Some(c.header.record_id));
        f.run(0, &[c.clone(), t.clone()], &[c.clone(), t.clone()])
            .unwrap();
        let cd = tombstone(&c);
        assert!(f.run(2, &[cd.clone()], &[cd.clone(), t.clone()]).is_err());
        assert_eq!(cursor(&f), 2);
        let mut h = t.header.clone();
        h.expected_server_version = 1;
        h.mutation_id = Uuid::new_v4();
        let mut content = open_record(&t, &KEY).unwrap();
        content.fields.as_mut().unwrap()["related_contact_id"] = Value::Null;
        let detached = seal_record(h, &KEY, &content).unwrap();
        f.run(2, &[cd.clone(), detached.clone()], &[cd, detached])
            .unwrap();
        assert!(live(&f.db.connection, 11, c.header.record_id)
            .unwrap()
            .is_none());
        assert_eq!(cursor(&f), 4);
    }
    #[test]
    fn task_tombstone_preserves_linked_rfi_and_refuses_orphaning() {
        let mut f = Fixture::new();
        let p = project(&f);
        let t = task_record(p, None);
        f.run(0, &[t.clone()], &[t.clone()]).unwrap();
        let rfi = Uuid::new_v4().to_string();
        f.db.connection.execute("INSERT INTO rfis(id,project_id,number,subject,question,created_date,created_at_utc,updated_at_utc) VALUES(?1,?2,'F-1','Fictional RFI','Fictional question','2026-10-10','2026-10-10T00:00:00Z','2026-10-10T00:00:00Z')",params![rfi,p.to_string()]).unwrap();
        f.db.connection.execute("INSERT INTO rfi_task_relationships(rfi_id,task_id,created_at_utc) VALUES(?1,?2,'2026-10-10T00:00:00Z')",params![rfi,t.header.record_id.to_string()]).unwrap();
        let td = tombstone(&t);
        assert!(f.run(1, &[td.clone()], &[td]).is_err());
        assert_eq!(cursor(&f), 1);
        assert!(live(&f.db.connection, 2, t.header.record_id)
            .unwrap()
            .is_some());
        assert_eq!(
            f.db.connection
                .query_row("SELECT COUNT(*) FROM rfi_task_relationships", [], |r| r
                    .get::<_, i64>(0))
                .unwrap(),
            1
        );
        let other = Uuid::new_v4();
        f.db.connection.execute("INSERT INTO projects(id,number,name,project_path,created_at_utc,updated_at_utc) VALUES(?1,'FICTIONAL-2','Another fictional project','C:\\Fictional\\Other','2026-10-10T00:00:00Z','2026-10-10T00:00:00Z')",[other.to_string()]).unwrap();
        let mut h = t.header.clone();
        h.expected_server_version = 1;
        h.mutation_id = Uuid::new_v4();
        let mut content = open_record(&t, &KEY).unwrap();
        content.fields.as_mut().unwrap()["project_id"] = json!(other.to_string());
        let moved = seal_record(h, &KEY, &content).unwrap();
        assert!(f.run(1, &[moved.clone()], &[moved]).is_err());
        assert_eq!(
            live(&f.db.connection, 2, t.header.record_id)
                .unwrap()
                .unwrap()["project_id"],
            p.to_string()
        );
        assert_eq!(cursor(&f), 1);
    }
    #[test]
    fn blank_task_title_refused_and_valid_revisions_retry_exactly() {
        let mut f = Fixture::new();
        let p = project(&f);
        let t = task_record(p, None);
        let mut content = open_record(&t, &KEY).unwrap();
        content.fields.as_mut().unwrap()["title"] = json!(" ");
        let blank = seal_record(t.header.clone(), &KEY, &content).unwrap();
        assert!(f.run(0, &[blank.clone()], &[blank]).is_err());
        assert_eq!(cursor(&f), 0);
        let mut identity = t.header.clone();
        identity.record_id = Uuid::parse_str("aaaaaaaa-bbbb-4ccc-8ddd-eeeeeeeeeeee").unwrap();
        let mut noncanonical = open_record(&t, &KEY).unwrap();
        noncanonical.fields.as_mut().unwrap()["id"] =
            json!(identity.record_id.to_string().to_uppercase());
        let noncanonical = seal_record(identity, &KEY, &noncanonical).unwrap();
        assert!(f.run(0, &[noncanonical.clone()], &[noncanonical]).is_err());
        assert_eq!(cursor(&f), 0);
        f.run(0, &[t.clone()], &[t.clone()]).unwrap();
        let mut h = t.header.clone();
        h.expected_server_version = 1;
        h.mutation_id = Uuid::new_v4();
        let mut content = open_record(&t, &KEY).unwrap();
        let fields = content.fields.as_mut().unwrap();
        fields["title"] = json!("Updated fictional task");
        fields["status"] = json!("completed");
        fields["waiting_on"] = Value::Null;
        fields["waiting_since_utc"] = Value::Null;
        let updated = seal_record(h, &KEY, &content).unwrap();
        f.run(1, &[updated.clone()], &[updated.clone()]).unwrap();
        f.run(1, &[updated.clone()], &[updated.clone()]).unwrap();
        assert_eq!(cursor(&f), 2);
        assert_eq!(
            live(&f.db.connection, 2, t.header.record_id).unwrap(),
            open_record(&updated, &KEY).unwrap().fields
        );
    }
    fn mapping_root(f: &Fixture) -> std::path::PathBuf {
        let path = f.root.join("projects");
        std::fs::create_dir(&path).unwrap();
        f.db.connection
            .execute(
                "INSERT INTO app_settings(key,value,updated_at_utc) VALUES('project_root_path',?1,'2026-10-10T00:00:00Z')",
                [path.to_str().unwrap()],
            )
            .unwrap();
        // Windows hosted runners may expose TEMP through an 8.3 alias. Expected
        // paths must use the same canonical root as production mapping.
        crate::sync_v2_project_paths::selected_root(&f.db).unwrap()
    }
    fn project_record(component: &str) -> SealedRecord {
        let h = header(1);
        seal_record(h.clone(),&KEY,&RecordContent {schema_version:1,fields:Some(json!({"id":h.record_id.to_string(),"number":"FICTIONAL-SYNC","name":"Fictional synced project","status":"active","phase":"construction","custom_phase_name":null,"customer":null,"general_contractor":null,"engineer":null,"project_manager":null,"superintendent":null,"location":null,"start_date":"2026-10-10","target_date":null,"description":null,"important_notes":null,"portable_root":{"kind":"workspace_relative","components":[component]},"is_pinned":true,"archived_at_utc":null,"created_at_utc":"2026-10-10T00:00:00Z","updated_at_utc":"2026-10-10T00:00:00Z"}))}).unwrap()
    }
    #[test]
    fn project_dependency_page_maps_locally_without_creating_documents() {
        let mut f = Fixture::new();
        let root = mapping_root(&f);
        let p = project_record("Fictional");
        let t = task_record(p.header.record_id, None);
        let backup = f
            .run(0, &[t.clone(), p.clone()], &[p.clone(), t.clone()])
            .unwrap();
        assert_eq!(
            recovery::preview(Path::new(&backup.path)).unwrap().projects,
            0
        );
        let row = live(&f.db.connection, 1, p.header.record_id)
            .unwrap()
            .unwrap();
        assert_eq!(
            row["project_path"],
            root.join("Fictional").to_str().unwrap()
        );
        assert_eq!(row["number"], "FICTIONAL-SYNC");
        assert_eq!(row["is_pinned"], true);
        assert!(!root.join("Fictional").exists());
        assert_eq!(cursor(&f), 2);
        f.run(0, &[t.clone(), p.clone()], &[p.clone(), t.clone()])
            .unwrap();
        assert_eq!(cursor(&f), 2);
        let pd = tombstone(&p);
        assert!(f.run(2, &[pd.clone()], &[pd.clone(), t.clone()]).is_err());
        assert!(live(&f.db.connection, 1, p.header.record_id)
            .unwrap()
            .is_some());
        let td = tombstone(&t);
        let backup = f.run(2, &[pd.clone(), td.clone()], &[pd, td]).unwrap();
        assert_eq!(
            recovery::preview(Path::new(&backup.path)).unwrap().projects,
            1
        );
        assert!(live(&f.db.connection, 1, p.header.record_id)
            .unwrap()
            .is_none());
        assert_eq!(cursor(&f), 4);
        assert_eq!(std::fs::read_dir(root).unwrap().count(), 0);
    }
    #[test]
    fn project_local_edits_and_remote_path_reassignment_are_preserved() {
        let mut f = Fixture::new();
        let root = mapping_root(&f);
        let p = project_record("Original");
        f.run(0, &[p.clone()], &[p.clone()]).unwrap();
        let mut h = p.header.clone();
        h.expected_server_version = 1;
        h.mutation_id = Uuid::new_v4();
        let mut content = open_record(&p, &KEY).unwrap();
        content.fields.as_mut().unwrap()["portable_root"]["components"] = json!(["Redirected"]);
        let moved = seal_record(h.clone(), &KEY, &content).unwrap();
        assert!(f.run(1, &[moved.clone()], &[moved]).is_err());
        assert_eq!(cursor(&f), 1);
        f.db.connection
            .execute("UPDATE projects SET name='Local offline project'", [])
            .unwrap();
        let mut content = open_record(&p, &KEY).unwrap();
        content.fields.as_mut().unwrap()["name"] = json!("Remote competing project");
        let remote = seal_record(h, &KEY, &content).unwrap();
        assert!(f.run(1, &[remote.clone()], &[remote]).is_err());
        let row = live(&f.db.connection, 1, p.header.record_id)
            .unwrap()
            .unwrap();
        assert_eq!(row["name"], "Local offline project");
        assert_eq!(row["project_path"], root.join("Original").to_str().unwrap());
        assert_eq!(cursor(&f), 1);
    }
    #[test]
    fn missing_root_reserved_names_external_and_existing_folder_refused() {
        let mut f = Fixture::new();
        let p = project_record("Fictional");
        assert!(f.run(0, &[p.clone()], &[p.clone()]).is_err());
        let root = mapping_root(&f);
        let reserved = project_record("NUL.txt");
        assert!(f.run(0, &[reserved.clone()], &[reserved]).is_err());
        let mut content = open_record(&p, &KEY).unwrap();
        content.fields.as_mut().unwrap()["portable_root"] =
            json!({"kind":"external_name","components":["Fictional"]});
        let external = seal_record(p.header.clone(), &KEY, &content).unwrap();
        assert!(f.run(0, &[external.clone()], &[external]).is_err());
        std::fs::create_dir(root.join("Fictional")).unwrap();
        std::fs::write(
            root.join("Fictional").join("sentinel"),
            b"Fictional existing document",
        )
        .unwrap();
        assert!(f.run(0, &[p.clone()], &[p]).is_err());
        assert_eq!(
            std::fs::read(root.join("Fictional").join("sentinel")).unwrap(),
            b"Fictional existing document"
        );
        assert_eq!(cursor(&f), 0);
    }
    #[test]
    fn colliding_project_mapping_rolls_back_page_and_valid_update_preserves_path() {
        let mut f = Fixture::new();
        let root = mapping_root(&f);
        let p = project_record("Fictional");
        let other = project_record("FICTIONAL");
        let mut content = open_record(&other, &KEY).unwrap();
        content.fields.as_mut().unwrap()["number"] = json!("FICTIONAL-OTHER");
        let other = seal_record(other.header.clone(), &KEY, &content).unwrap();
        assert!(f
            .run(0, &[p.clone(), other.clone()], &[p.clone(), other])
            .is_err());
        assert_eq!(cursor(&f), 0);
        assert!(live(&f.db.connection, 1, p.header.record_id)
            .unwrap()
            .is_none());
        f.run(0, &[p.clone()], &[p.clone()]).unwrap();
        let mut h = p.header.clone();
        h.expected_server_version = 1;
        h.mutation_id = Uuid::new_v4();
        let mut content = open_record(&p, &KEY).unwrap();
        content.fields.as_mut().unwrap()["name"] = json!("Updated fictional project");
        let updated = seal_record(h, &KEY, &content).unwrap();
        f.run(1, &[updated.clone()], &[updated]).unwrap();
        assert_eq!(cursor(&f), 2);
        assert_eq!(
            live(&f.db.connection, 1, p.header.record_id)
                .unwrap()
                .unwrap()["project_path"],
            root.join("Fictional").to_str().unwrap()
        );
        assert_eq!(std::fs::read_dir(root).unwrap().count(), 0);
    }
    fn link_record(kind: u8, register: Uuid, task: Uuid) -> SealedRecord {
        let field = if kind == 5 { "rfi_id" } else { "submittal_id" };
        seal_record(header(kind),&KEY,&RecordContent {schema_version:1,fields:Some(json!({field:register.to_string(),"task_id":task.to_string(),"created_at_utc":"2026-10-10T00:00:00Z"}))}).unwrap()
    }
    fn link_live(f: &Fixture, link: &SealedRecord) -> Option<Value> {
        let binding = crate::sync_v2_relationship_adapters::stored(
            &f.db.connection,
            f.scope.workspace_id,
            link.header.record_id,
        )
        .unwrap()
        .unwrap();
        crate::sync_v2_relationship_adapters::live(&f.db.connection, &binding).unwrap()
    }
    #[test]
    fn relationship_dependency_order_retry_and_durable_identity() {
        let mut f = Fixture::new();
        let p = project(&f);
        let t = task_record(p, None);
        let r = rfi_record(p);
        let s = submittal_record(p, None, "0");
        let a = link_record(5, r.header.record_id, t.header.record_id);
        let b = link_record(6, s.header.record_id, t.header.record_id);
        let page = vec![a.clone(), b.clone(), s, r, t];
        let backup = f.run(0, &page, &page).unwrap();
        let before = Database::open(Path::new(&backup.path)).unwrap();
        assert_eq!(
            before
                .connection
                .query_row(
                    "SELECT COUNT(*) FROM sync_v2_relationship_bindings",
                    [],
                    |r| r.get::<_, i64>(0)
                )
                .unwrap(),
            0
        );
        assert_eq!(link_live(&f, &a), open_record(&a, &KEY).unwrap().fields);
        assert_eq!(link_live(&f, &b), open_record(&b, &KEY).unwrap().fields);
        // Reopen an actual private database copy to prove mapping survives restart.
        let path = f.root.join("restart.sqlite");
        {
            let mut copy = Connection::open(&path).unwrap();
            Backup::new(&f.db.connection, &mut copy)
                .unwrap()
                .run_to_completion(64, Duration::from_millis(1), None)
                .unwrap();
        }
        f.db = Database::open(&path).unwrap();
        f.run(0, &page, &page).unwrap();
        assert_eq!(cursor(&f), 5);
        assert!(link_live(&f, &a).is_some());
    }
    #[test]
    fn explicit_link_tombstones_allow_parent_and_task_deletion() {
        for kind in [5, 6] {
            let mut f = Fixture::new();
            let p = project(&f);
            let t = task_record(p, None);
            let register = if kind == 5 {
                rfi_record(p)
            } else {
                submittal_record(p, None, "0")
            };
            let link = link_record(kind, register.header.record_id, t.header.record_id);
            let page = vec![link.clone(), register.clone(), t.clone()];
            f.run(0, &page, &page).unwrap();
            let dead = vec![tombstone(&t), tombstone(&register), tombstone(&link)];
            let backup = f.run(3, &dead, &dead).unwrap();
            assert!(link_live(&f, &link).is_none());
            let old = Database::open(Path::new(&backup.path)).unwrap();
            assert_eq!(
                old.connection
                    .query_row(
                        "SELECT COUNT(*) FROM sync_v2_relationship_bindings",
                        [],
                        |r| r.get::<_, i64>(0)
                    )
                    .unwrap(),
                1
            );
            f.run(3, &dead, &dead).unwrap();
            assert_eq!(cursor(&f), 6);
            assert_eq!(
                f.db.connection
                    .query_row(
                        "SELECT COUNT(*) FROM sync_v2_relationship_bindings",
                        [],
                        |r| r.get::<_, i64>(0)
                    )
                    .unwrap(),
                1
            );
        }
    }
    #[test]
    fn relationship_identity_cannot_rebind_even_after_tombstone() {
        let mut f = Fixture::new();
        let p = project(&f);
        let t = task_record(p, None);
        let r = rfi_record(p);
        let link = link_record(5, r.header.record_id, t.header.record_id);
        let page = vec![r.clone(), t.clone(), link.clone()];
        f.run(0, &page, &page).unwrap();
        let changed = next_record(&link, |v| v["task_id"] = json!(Uuid::new_v4().to_string()));
        assert!(f
            .run(3, &[changed.clone()], &[r.clone(), t.clone(), changed])
            .is_err());
        assert_eq!(cursor(&f), 3);
        let dead = tombstone(&link);
        f.run(3, &[dead.clone()], &[r.clone(), t.clone(), dead.clone()])
            .unwrap();
        // Tombstones cannot be turned into another pair through an unauthenticated lookup.
        let mut h = dead.header.clone();
        h.expected_server_version += 1;
        h.mutation_id = Uuid::new_v4();
        h.tombstone = false;
        let new = link_record(5, r.header.record_id, Uuid::new_v4());
        let rebound = seal_record(h, &KEY, &open_record(&new, &KEY).unwrap()).unwrap();
        assert!(f.run(4, &[rebound.clone()], &[r, t, rebound]).is_err());
        assert_eq!(cursor(&f), 4);
    }
    #[test]
    fn duplicate_relationship_envelopes_roll_back_the_whole_page() {
        let mut f = Fixture::new();
        let p = project(&f);
        let t = task_record(p, None);
        let r = rfi_record(p);
        let a = link_record(5, r.header.record_id, t.header.record_id);
        let b = link_record(5, r.header.record_id, t.header.record_id);
        let page = vec![r, t, a, b];
        assert!(f.run(0, &page, &page).is_err());
        assert_eq!(cursor(&f), 0);
        assert_eq!(
            f.db.connection
                .query_row(
                    "SELECT COUNT(*) FROM sync_v2_relationship_bindings",
                    [],
                    |r| r.get::<_, i64>(0)
                )
                .unwrap(),
            0
        );
        assert_eq!(
            f.db.connection
                .query_row("SELECT COUNT(*) FROM rfis", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            0
        );
    }
    #[test]
    fn missing_cross_project_and_unknown_tombstone_links_are_refused() {
        for kind in [5, 6] {
            let mut f = Fixture::new();
            let p = project(&f);
            let t = task_record(p, None);
            let register = if kind == 5 {
                rfi_record(Uuid::new_v4())
            } else {
                submittal_record(Uuid::new_v4(), None, "0")
            };
            let link = link_record(kind, register.header.record_id, t.header.record_id);
            assert!(f.run(0, &[link.clone()], &[link.clone()]).is_err());
            let dead = tombstone(&link);
            assert!(f.run(0, &[dead.clone()], &[dead]).is_err());
            let other = project_record("Other");
            mapping_root(&f);
            let register = next_record(&register, |v| {
                v["project_id"] = json!(other.header.record_id.to_string())
            });
            // next_record has version 1; use a first-version envelope for a new register.
            let mut h = register.header.clone();
            h.expected_server_version = 0;
            let register = seal_record(h, &KEY, &open_record(&register, &KEY).unwrap()).unwrap();
            let page = vec![link, register, t, other];
            assert!(f.run(0, &page, &page).is_err());
            assert_eq!(cursor(&f), 0);
        }
    }
    #[test]
    fn local_relationship_edit_and_unmanaged_pair_are_preserved() {
        let mut f = Fixture::new();
        let p = project(&f);
        let t = task_record(p, None);
        let r = rfi_record(p);
        let link = link_record(5, r.header.record_id, t.header.record_id);
        let page = vec![r.clone(), t.clone(), link.clone()];
        f.run(0, &page, &page).unwrap();
        f.db.connection
            .execute(
                "UPDATE rfi_task_relationships SET created_at_utc='2026-10-10T00:00:01Z'",
                [],
            )
            .unwrap();
        let dead = tombstone(&link);
        assert!(f
            .run(3, &[dead.clone()], &[r.clone(), t.clone(), dead])
            .is_err());
        assert_eq!(cursor(&f), 3);
        let new = link_record(5, r.header.record_id, t.header.record_id);
        assert!(f
            .run(3, &[new.clone()], &[r, t, link.clone(), new])
            .is_err());
        assert_eq!(
            link_live(&f, &link).unwrap()["created_at_utc"],
            "2026-10-10T00:00:01Z"
        );
    }
    #[test]
    fn relationship_updates_and_exact_deletion_preserve_other_links() {
        let mut f = Fixture::new();
        let p = project(&f);
        let t = task_record(p, None);
        let r = rfi_record(p);
        let s = submittal_record(p, None, "0");
        let a = link_record(5, r.header.record_id, t.header.record_id);
        let b = link_record(6, s.header.record_id, t.header.record_id);
        let page = vec![r.clone(), s.clone(), t.clone(), a.clone(), b.clone()];
        f.run(0, &page, &page).unwrap();
        let update = next_record(&a, |v| v["created_at_utc"] = json!("2026-10-10T00:00:01Z"));
        let manifest = vec![r.clone(), s.clone(), t.clone(), update.clone(), b.clone()];
        f.run(5, &[update.clone()], &manifest).unwrap();
        assert_eq!(
            link_live(&f, &a).unwrap()["created_at_utc"],
            "2026-10-10T00:00:01Z"
        );
        let dead = tombstone(&update);
        f.run(6, &[dead.clone()], &[r, s, t, dead, b.clone()])
            .unwrap();
        assert!(link_live(&f, &a).is_none());
        assert!(link_live(&f, &b).is_some());
        assert_eq!(cursor(&f), 7);
    }
    fn content_record(kind: u8, mut fields: Value) -> SealedRecord {
        let h = header(kind);
        fields["id"] = json!(h.record_id.to_string());
        seal_record(
            h,
            &KEY,
            &RecordContent {
                schema_version: 1,
                fields: Some(fields),
            },
        )
        .unwrap()
    }
    fn work_record(p: Uuid) -> SealedRecord {
        content_record(
            13,
            json!({"subtype":"work_item","id":"","project_id":p.to_string(),"item_type":"milestone","number":null,"title":"Fictional milestone","description":null,"status":"open","priority":"medium","due_date":null,"occurred_date":null,"responsible_party":null,"company":null,"location":null,"amount_cents":450,"checklist_total":2,"checklist_completed":1,"archived_at_utc":null,"created_at_utc":"2026-10-10T00:00:00Z","updated_at_utc":"2026-10-10T00:00:00Z"}),
        )
    }
    fn template_record() -> SealedRecord {
        content_record(
            13,
            json!({"subtype":"project_template","id":"","name":"Fictional template","task_titles":["Review"],"milestone_titles":["Closeout"],"created_at_utc":"2026-10-10T00:00:00Z","updated_at_utc":"2026-10-10T00:00:00Z"}),
        )
    }
    fn checkpoint_page(
        f: &Fixture,
        after: u64,
        records: &[SealedRecord],
        manifest: &[SealedRecord],
    ) -> (Vec<PulledRecord>, SealedWorkspaceCheckpoint) {
        let through = after + records.len() as u64;
        let cp = seal_workspace_checkpoint(
            f.scope.owner_id,
            f.scope.workspace_id,
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
        (
            records
                .iter()
                .enumerate()
                .map(|(i, r)| PulledRecord {
                    change_seq: after + i as u64 + 1,
                    server_version: r.header.expected_server_version + 1,
                    record: r.clone(),
                })
                .collect(),
            cp,
        )
    }
    fn review(
        f: &mut Fixture,
        after: u64,
        records: &[SealedRecord],
        manifest: &[SealedRecord],
    ) -> AppResult<PullOutcome> {
        let (changes, cp) = checkpoint_page(f, after, records, manifest);
        f.db.apply_sync_v2_reviewed_records(f.scope, after, &changes, &cp, &KEY, &f.root)
    }
    fn count(f: &Fixture, table: &str) -> i64 {
        f.db.connection
            .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get(0))
            .unwrap()
    }
    #[test]
    fn remaining_metadata_adapters_preserve_fields_and_never_touch_documents() {
        let mut f = Fixture::new();
        let root = mapping_root(&f);
        let p = project(&f);
        let r = rfi_record(p);
        let s = submittal_record(p, None, "0");
        let sentinel = root.join("drawing.pdf");
        std::fs::write(&sentinel, b"Fictional document sentinel").unwrap();
        let portable = json!({"kind":"workspace_relative","components":["drawing.pdf"]});
        let a = content_record(
            7,
            json!({"id":"","rfi_id":r.header.record_id.to_string(),"portable_path":portable,"display_name":"RFI drawing","created_at_utc":"2026-10-10T00:00:00Z"}),
        );
        let b = content_record(
            8,
            json!({"id":"","submittal_id":s.header.record_id.to_string(),"portable_path":portable,"display_name":"Submittal drawing","created_at_utc":"2026-10-10T00:00:00Z"}),
        );
        let file = content_record(
            9,
            json!({"id":"","project_id":p.to_string(),"file_name":"drawing.pdf","portable_path":portable,"operation":"move","discipline":"controls","drawing_number":"F-1","title":"Fictional drawing","revision":"A","revision_date":"2026-10-10","received_date":"2026-10-10","is_current":true,"created_at_utc":"2026-10-10T00:00:00Z","updated_at_utc":"2026-10-10T00:00:00Z"}),
        );
        let event = content_record(
            12,
            json!({"id":"","event_type":"created","entity_type":"task","entity_id":Uuid::new_v4().to_string(),"summary":"Fictional retained history","occurred_at_utc":"2026-10-10T00:00:00Z"}),
        );
        let work = work_record(p);
        let template = template_record();
        let page = vec![
            a.clone(),
            b.clone(),
            file.clone(),
            event.clone(),
            work.clone(),
            template.clone(),
            r.clone(),
            s.clone(),
        ];
        f.run(0, &page, &page).unwrap();
        for record in [&a, &b, &file] {
            let fields = live(
                &f.db.connection,
                record.header.record_kind,
                record.header.record_id,
            )
            .unwrap()
            .unwrap();
            assert_eq!(fields["file_path"], sentinel.to_str().unwrap());
        }
        for record in [&event, &work, &template] {
            assert_eq!(
                live(
                    &f.db.connection,
                    record.header.record_kind,
                    record.header.record_id
                )
                .unwrap(),
                open_record(record, &KEY).unwrap().fields
            );
        }
        let deleted = page.iter().map(tombstone).collect::<Vec<_>>();
        f.run(8, &deleted, &deleted).unwrap();
        assert_eq!(
            std::fs::read(sentinel).unwrap(),
            b"Fictional document sentinel"
        );
        assert_eq!(count(&f, "registered_files"), 0);
    }
    #[test]
    fn external_references_are_unresolved_and_unsafe_mappings_are_refused() {
        let mut f = Fixture::new();
        mapping_root(&f);
        let p = project(&f);
        let r = rfi_record(p);
        let a = content_record(
            7,
            json!({"id":"","rfi_id":r.header.record_id.to_string(),"portable_path":{"kind":"external_name","components":["external.pdf"]},"display_name":"External","created_at_utc":"2026-10-10T00:00:00Z"}),
        );
        f.run(0, &[r.clone(), a.clone()], &[r.clone(), a.clone()])
            .unwrap();
        let path = live(&f.db.connection, 7, a.header.record_id)
            .unwrap()
            .unwrap()["file_path"]
            .as_str()
            .unwrap()
            .to_owned();
        assert!(path.starts_with("sitedatum-unresolved:"));
        assert!(!Path::new(&path).exists());
        let bad = next_record(&a, |v| {
            v["portable_path"] = json!({"kind":"workspace_relative","components":["CON.pdf"]})
        });
        assert!(f.run(2, &[bad.clone()], &[r, bad]).is_err());
        assert_eq!(cursor(&f), 2);
    }
    #[test]
    fn work_domain_subtype_and_immutable_history_checks_fail_closed() {
        let mut f = Fixture::new();
        let p = project(&f);
        let work = work_record(p);
        let bad = next_record(&work, |v| v["item_type"] = json!("daily_report"));
        assert!(f.run(0, &[bad.clone()], &[bad]).is_err());
        f.run(0, &[work.clone()], &[work.clone()]).unwrap();
        let dead = tombstone(&work);
        f.run(1, &[dead.clone()], &[dead.clone()]).unwrap();
        let mut template = template_record();
        template.header.record_id = work.header.record_id;
        template.header.expected_server_version = 2;
        let mut fields = open_record(&template_record(), &KEY).unwrap();
        fields.fields.as_mut().unwrap()["id"] = json!(work.header.record_id.to_string());
        let swapped = seal_record(template.header, &KEY, &fields).unwrap();
        assert!(f.run(2, &[swapped.clone()], &[swapped]).is_err());
        let event = content_record(
            12,
            json!({"id":"","event_type":"created","entity_type":"task","entity_id":Uuid::new_v4().to_string(),"summary":"History","occurred_at_utc":"2026-10-10T00:00:00Z"}),
        );
        f.run(2, &[event.clone()], &[dead.clone(), event.clone()])
            .unwrap();
        let changed = next_record(&event, |v| v["summary"] = json!("Rewrite history"));
        assert!(f.run(3, &[changed.clone()], &[dead, changed]).is_err());
        assert_eq!(cursor(&f), 3);
    }
    #[test]
    fn repeated_revisions_apply_once_and_replay_requires_exact_history() {
        let mut f = Fixture::new();
        let a = record("First");
        let b = next_record(&a, |v| v["body"] = json!("Second"));
        let c = next_record(&b, |v| v["body"] = json!("Third"));
        let page = vec![a.clone(), b.clone(), c.clone()];
        f.run(0, &page, &[c.clone()]).unwrap();
        f.run(0, &page, &[c.clone()]).unwrap();
        assert_eq!(cursor(&f), 3);
        assert_eq!(
            live(&f.db.connection, 10, a.header.record_id)
                .unwrap()
                .unwrap()["body"],
            "Third"
        );
        let altered = next_record(&a, |v| v["body"] = json!("Altered second"));
        assert!(f.run(0, &[a.clone(), altered, c.clone()], &[c]).is_err());
        assert_eq!(cursor(&f), 3);
    }
    #[test]
    fn explicit_revision_tree_deletion_orders_children_before_parents() {
        let mut f = Fixture::new();
        let p = project(&f);
        let parent = submittal_record(p, None, "0");
        let child = submittal_record(p, Some(parent.header.record_id), "1");
        f.run(
            0,
            &[parent.clone(), child.clone()],
            &[parent.clone(), child.clone()],
        )
        .unwrap();
        let dead = vec![tombstone(&parent), tombstone(&child)];
        f.run(2, &dead, &dead).unwrap();
        assert_eq!(count(&f, "submittals"), 0);
    }
    #[test]
    fn live_mutation_outbox_and_receipts_roll_back_together_on_failure() {
        let mut f = Fixture::new();
        let note = record("Atomic local");
        f.db.connection.execute_batch("CREATE TRIGGER reject_note BEFORE INSERT ON notes BEGIN SELECT RAISE(ABORT,'injected'); END").unwrap();
        assert!(f.db.mutate_sync_v2_live(f.scope, &note, &KEY).is_err());
        assert_eq!(count(&f, "sync_v2_outbox"), 0);
        assert_eq!(count(&f, "sync_v2_committed_bases"), 0);
        assert_eq!(count(&f, "notes"), 0);
        f.db.connection
            .execute_batch("DROP TRIGGER reject_note")
            .unwrap();
        f.db.mutate_sync_v2_live(f.scope, &note, &KEY).unwrap();
        f.db.mutate_sync_v2_live(f.scope, &note, &KEY).unwrap();
        assert_eq!(count(&f, "sync_v2_outbox"), 1);
        assert_eq!(count(&f, "notes"), 1);
        assert_eq!(count(&f, "sync_v2_applied_receipts"), 0);
    }
    #[test]
    fn extreme_record_versions_and_copied_committed_bases_fail_without_state_changes() {
        let mut f = Fixture::new();
        let a = record("First baseline");
        let b = record("Second baseline");
        f.run(0, &[a.clone(), b.clone()], &[a.clone(), b.clone()])
            .unwrap();
        let local = next_record(&a, |v| v["body"] = json!("Offline edit"));
        f.db.mutate_sync_v2_live(f.scope, &local, &KEY).unwrap();
        let pending = f.db.pending_sync_v2_mutations(f.scope).unwrap();
        f.db.connection
            .execute(
                "UPDATE sync_v2_committed_bases SET envelope=?1",
                [crate::sync_v2_local_queue::encode(&b).unwrap()],
            )
            .unwrap();
        let remote = next_record(&a, |v| v["body"] = json!("Remote edit"));
        assert!(review(&mut f, 2, &[remote.clone()], &[remote, b.clone()]).is_err());
        assert_eq!(cursor(&f), 2);
        assert_eq!(count(&f, "sync_v2_record_conflicts"), 0);
        assert_eq!(f.db.pending_sync_v2_mutations(f.scope).unwrap(), pending);
        let mut extreme = b.clone();
        extreme.header.expected_server_version = u64::MAX;
        assert!(f.db.mutate_sync_v2_live(f.scope, &extreme, &KEY).is_err());
        let (mut changes, cp) = checkpoint_page(&f, 2, &[b.clone()], &[a, b]);
        changes[0].record = extreme;
        assert!(f
            .db
            .buffer_sync_v2_page(f.scope, 2, &changes, &cp, &KEY)
            .is_err());
        assert_eq!(count(&f, "sync_v2_pull_parts"), 0);
        assert_eq!(cursor(&f), 2);
    }
    #[test]
    fn durable_conflicts_require_explicit_idempotent_record_choices() {
        for choice in [ConflictChoice::KeepLocal, ConflictChoice::AcceptRemote] {
            let mut f = Fixture::new();
            let a = record("Baseline");
            f.run(0, &[a.clone()], &[a.clone()]).unwrap();
            let local = next_record(&a, |v| v["body"] = json!("Local candidate"));
            f.db.mutate_sync_v2_live(f.scope, &local, &KEY).unwrap();
            let remote = next_record(&a, |v| v["body"] = json!("Remote candidate"));
            assert!(matches!(
                review(&mut f, 1, &[remote.clone()], &[remote.clone()]).unwrap(),
                PullOutcome::NeedsReview(_)
            ));
            assert_eq!(cursor(&f), 1);
            assert_eq!(count(&f, "sync_v2_record_conflicts"), 1);
            assert_eq!(count(&f, "sync_v2_outbox"), 1);
            let original_pending: Vec<u8> =
                f.db.connection
                    .query_row(
                        "SELECT pending_envelope FROM sync_v2_record_conflicts",
                        [],
                        |r| r.get(0),
                    )
                    .unwrap();
            assert_eq!(
                original_pending,
                crate::sync_v2_local_queue::encode(&local).unwrap()
            );
            let restart_path = f.root.join("conflict-restart.sqlite3");
            {
                let mut copy = Connection::open(&restart_path).unwrap();
                Backup::new(&f.db.connection, &mut copy)
                    .unwrap()
                    .run_to_completion(64, Duration::from_millis(1), None)
                    .unwrap();
            }
            f.db = Database::open(&restart_path).unwrap();
            let bytes: Vec<u8> =
                f.db.connection
                    .query_row(
                        "SELECT local_envelope FROM sync_v2_record_conflicts",
                        [],
                        |r| r.get(0),
                    )
                    .unwrap();
            assert!(!bytes.windows(5).any(|v| v == b"Local"));
            f.db.choose_sync_v2_conflict(f.scope, a.header.record_id, 2, choice, &KEY)
                .unwrap();
            f.db.choose_sync_v2_conflict(f.scope, a.header.record_id, 2, choice, &KEY)
                .unwrap();
            assert!(matches!(
                review(&mut f, 1, &[remote.clone()], &[remote.clone()]).unwrap(),
                PullOutcome::Applied(_)
            ));
            assert_eq!(cursor(&f), 2);
            let kept = choice == ConflictChoice::KeepLocal;
            assert_eq!(
                live(&f.db.connection, 10, a.header.record_id)
                    .unwrap()
                    .unwrap()["body"],
                if kept {
                    "Local candidate"
                } else {
                    "Remote candidate"
                }
            );
            assert_eq!(count(&f, "sync_v2_outbox"), i64::from(kept));
            assert_eq!(count(&f, "sync_v2_applied_receipts"), i64::from(!kept));
            assert!(matches!(
                review(&mut f, 1, &[remote.clone()], &[remote]).unwrap(),
                PullOutcome::Applied(_)
            ));
            assert_eq!(count(&f, "sync_v2_outbox"), i64::from(kept));
            f.db = Database::open_in_memory().unwrap();
        }
    }
    #[test]
    fn corrupt_checkpoints_and_stale_conflict_choices_preserve_local_work() {
        let mut f = Fixture::new();
        let a = record("Baseline");
        f.run(0, &[a.clone()], &[a.clone()]).unwrap();
        f.db.connection
            .execute("UPDATE notes SET body='Local changed'", [])
            .unwrap();
        let b = next_record(&a, |v| v["body"] = json!("Remote"));
        let (changes, mut cp) = checkpoint_page(&f, 1, &[b.clone()], &[b.clone()]);
        cp.ciphertext[30] ^= 1;
        assert!(f
            .db
            .apply_sync_v2_reviewed_records(f.scope, 1, &changes, &cp, &KEY, &f.root)
            .is_err());
        assert_eq!(count(&f, "sync_v2_record_conflicts"), 0);
        assert!(matches!(
            review(&mut f, 1, &[b.clone()], &[b.clone()]).unwrap(),
            PullOutcome::NeedsReview(_)
        ));
        f.db.choose_sync_v2_conflict(
            f.scope,
            a.header.record_id,
            2,
            ConflictChoice::AcceptRemote,
            &KEY,
        )
        .unwrap();
        f.db.connection
            .execute("UPDATE notes SET body='Newer local work'", [])
            .unwrap();
        assert!(review(&mut f, 1, &[b.clone()], &[b]).is_err());
        assert_eq!(cursor(&f), 1);
        assert_eq!(
            live(&f.db.connection, 10, a.header.record_id)
                .unwrap()
                .unwrap()["body"],
            "Newer local work"
        );
    }
    #[test]
    fn paged_pull_survives_restart_without_advancing_live_cursor() {
        let mut f = Fixture::new();
        let a = record("Page one");
        let b = record("Page two");
        let page = vec![a.clone(), b.clone()];
        let (changes, cp) = checkpoint_page(&f, 0, &page, &page);
        assert!(!f
            .db
            .buffer_sync_v2_page(f.scope, 0, &changes[..1], &cp, &KEY)
            .unwrap());
        assert_eq!(cursor(&f), 0);
        assert_eq!(count(&f, "notes"), 0);
        assert!(f
            .db
            .apply_buffered_sync_v2_records(f.scope, &KEY, &f.root)
            .is_err());
        assert_eq!(cursor(&f), 0);
        let path = f.root.join("paged.sqlite");
        {
            let mut copy = Connection::open(&path).unwrap();
            Backup::new(&f.db.connection, &mut copy)
                .unwrap()
                .run_to_completion(64, Duration::from_millis(1), None)
                .unwrap();
        }
        f.db = Database::open(&path).unwrap();
        assert!(!f
            .db
            .buffer_sync_v2_page(f.scope, 0, &changes[..1], &cp, &KEY)
            .unwrap());
        assert!(f
            .db
            .buffer_sync_v2_page(f.scope, 0, &changes[1..], &cp, &KEY)
            .unwrap());
        assert!(matches!(
            f.db.apply_buffered_sync_v2_records(f.scope, &KEY, &f.root)
                .unwrap(),
            PullOutcome::Applied(_)
        ));
        assert_eq!(cursor(&f), 2);
        assert_eq!(count(&f, "sync_v2_pull_parts"), 0);
        assert_eq!(count(&f, "sync_v2_applied_receipts"), 1);
        f.db = Database::open_in_memory().unwrap();
    }
    #[test]
    fn omitted_reordered_and_substituted_pages_do_not_advance_or_discard_buffer() {
        let mut f = Fixture::new();
        let a = record("First");
        let b = record("Second");
        let records = vec![a.clone(), b.clone()];
        let (changes, cp) = checkpoint_page(&f, 0, &records, &records);
        assert!(f
            .db
            .buffer_sync_v2_page(f.scope, 0, &changes[1..], &cp, &KEY)
            .is_err());
        assert_eq!(count(&f, "sync_v2_pull_parts"), 0);
        f.db.buffer_sync_v2_page(f.scope, 0, &changes[..1], &cp, &KEY)
            .unwrap();
        let mut altered = changes[..1].to_vec();
        altered[0].record = record("Substitution");
        assert!(f
            .db
            .buffer_sync_v2_page(f.scope, 0, &altered, &cp, &KEY)
            .is_err());
        assert_eq!(count(&f, "sync_v2_pull_parts"), 1);
        assert_eq!(cursor(&f), 0);
    }
    #[test]
    fn applied_receipt_trigger_failure_rolls_back_live_state_and_cursor() {
        let mut f = Fixture::new();
        let a = record("Receipt rollback");
        f.db.connection.execute_batch("CREATE TRIGGER reject_receipt BEFORE INSERT ON sync_v2_applied_receipts BEGIN SELECT RAISE(ABORT,'injected'); END").unwrap();
        assert!(review(&mut f, 0, &[a.clone()], &[a]).is_err());
        assert_eq!(count(&f, "notes"), 0);
        assert_eq!(cursor(&f), 0);
        assert_eq!(count(&f, "sync_v2_applied_receipts"), 0);
    }
    #[test]
    fn two_offline_devices_resolve_keep_local_then_converge_through_receipts() {
        let mut first = Fixture::new();
        let mut second = Fixture::new();
        let baseline = record("Shared baseline");
        first
            .run(0, &[baseline.clone()], &[baseline.clone()])
            .unwrap();
        second
            .run(0, &[baseline.clone()], &[baseline.clone()])
            .unwrap();
        let remote = next_record(&baseline, |v| v["body"] = json!("First device edit"));
        let local = next_record(&baseline, |v| v["body"] = json!("Second device edit"));
        first
            .db
            .mutate_sync_v2_live(first.scope, &remote, &KEY)
            .unwrap();
        second
            .db
            .mutate_sync_v2_live(second.scope, &local, &KEY)
            .unwrap();
        first
            .db
            .accept_sync_v2_receipts(
                first.scope,
                &[crate::sync_v2_local_queue::MutationReceipt {
                    record_id: remote.header.record_id,
                    mutation_id: remote.header.mutation_id,
                    server_version: 2,
                }],
            )
            .unwrap();
        first.run(1, &[remote.clone()], &[remote.clone()]).unwrap();
        assert!(matches!(
            review(&mut second, 1, &[remote.clone()], &[remote.clone()]).unwrap(),
            PullOutcome::NeedsReview(_)
        ));
        second
            .db
            .choose_sync_v2_conflict(
                second.scope,
                baseline.header.record_id,
                2,
                ConflictChoice::KeepLocal,
                &KEY,
            )
            .unwrap();
        review(&mut second, 1, &[remote.clone()], &[remote]).unwrap();
        let resolved = second
            .db
            .pending_sync_v2_mutations(second.scope)
            .unwrap()
            .remove(0);
        assert_eq!(resolved.header.expected_server_version, 2);
        second
            .db
            .accept_sync_v2_receipts(
                second.scope,
                &[crate::sync_v2_local_queue::MutationReceipt {
                    record_id: resolved.header.record_id,
                    mutation_id: resolved.header.mutation_id,
                    server_version: 3,
                }],
            )
            .unwrap();
        second
            .run(2, &[resolved.clone()], &[resolved.clone()])
            .unwrap();
        first
            .run(2, &[resolved.clone()], &[resolved.clone()])
            .unwrap();
        assert_eq!(
            live(&first.db.connection, 10, resolved.header.record_id).unwrap(),
            live(&second.db.connection, 10, resolved.header.record_id).unwrap()
        );
        assert_eq!(cursor(&first), 3);
        assert_eq!(cursor(&second), 3);
        assert!(second
            .db
            .applied_sync_v2_receipt(second.scope, &KEY, &second.root)
            .unwrap()
            .is_some());
    }
    #[test]
    fn deletion_conflicts_preserve_local_candidate_until_explicit_choice() {
        for choice in [ConflictChoice::KeepLocal, ConflictChoice::AcceptRemote] {
            let mut f = Fixture::new();
            let a = record("Before deletion");
            f.run(0, &[a.clone()], &[a.clone()]).unwrap();
            f.db.connection
                .execute("UPDATE notes SET body='Offline work'", [])
                .unwrap();
            let dead = tombstone(&a);
            assert!(matches!(
                review(&mut f, 1, &[dead.clone()], &[dead.clone()]).unwrap(),
                PullOutcome::NeedsReview(_)
            ));
            assert_eq!(count(&f, "notes"), 1);
            f.db.choose_sync_v2_conflict(f.scope, a.header.record_id, 2, choice, &KEY)
                .unwrap();
            review(&mut f, 1, &[dead.clone()], &[dead]).unwrap();
            assert_eq!(
                count(&f, "notes"),
                i64::from(choice == ConflictChoice::KeepLocal)
            );
            assert_eq!(count(&f, "sync_v2_record_conflicts"), 1);
        }
    }
    #[test]
    fn external_attachment_tombstone_conflict_keeps_original_portable_name() {
        let mut f = Fixture::new();
        mapping_root(&f);
        let p = project(&f);
        let r = rfi_record(p);
        let a = content_record(
            7,
            json!({"id":"","rfi_id":r.header.record_id.to_string(),"portable_path":{"kind":"external_name","components":["original.pdf"]},"display_name":"Original display","created_at_utc":"2026-10-10T00:00:00Z"}),
        );
        f.run(0, &[r.clone(), a.clone()], &[r.clone(), a.clone()])
            .unwrap();
        f.db.connection
            .execute(
                "UPDATE rfi_attachment_references SET display_name='Offline display'",
                [],
            )
            .unwrap();
        let dead = tombstone(&a);
        assert!(matches!(
            review(&mut f, 2, &[dead.clone()], &[r.clone(), dead.clone()]).unwrap(),
            PullOutcome::NeedsReview(_)
        ));
        f.db.choose_sync_v2_conflict(
            f.scope,
            a.header.record_id,
            3,
            ConflictChoice::KeepLocal,
            &KEY,
        )
        .unwrap();
        review(&mut f, 2, &[dead.clone()], &[r, dead]).unwrap();
        let queued = f.db.pending_sync_v2_mutations(f.scope).unwrap();
        assert_eq!(
            open_record(&queued[0], &KEY).unwrap().fields.unwrap()["portable_path"]["components"]
                [0],
            "original.pdf"
        );
    }
    #[test]
    fn applied_receipt_requires_current_live_rows_scope_and_existing_verified_backup() {
        let mut f = Fixture::new();
        let a = record("Receipt evidence");
        let backup = f.run(0, &[a.clone()], &[a.clone()]).unwrap();
        let receipt =
            f.db.applied_sync_v2_receipt(f.scope, &KEY, &f.root)
                .unwrap()
                .unwrap();
        assert_eq!(receipt.workspace_id, f.scope.workspace_id);
        assert_eq!(receipt.device_id, f.scope.device_id);
        assert_eq!(receipt.checkpoint_counter, 1);
        assert_eq!(receipt.through_change_seq, 1);
        assert!(backup.file_name.contains(&receipt.backup_id.to_string()));
        assert_ne!(receipt.checkpoint_digest, [0; 32]);
        let other = StreamScope {
            device_id: Uuid::new_v4(),
            ..f.scope
        };
        assert!(f
            .db
            .applied_sync_v2_receipt(other, &KEY, &f.root)
            .unwrap()
            .is_none());
        f.db.connection
            .execute("UPDATE notes SET body='Unsynced local edit'", [])
            .unwrap();
        assert!(f
            .db
            .applied_sync_v2_receipt(f.scope, &KEY, &f.root)
            .unwrap()
            .is_none());
        f.db.connection
            .execute("UPDATE notes SET body='Receipt evidence'", [])
            .unwrap();
        std::fs::remove_file(&backup.path).unwrap();
        assert!(f
            .db
            .applied_sync_v2_receipt(f.scope, &KEY, &f.root)
            .unwrap()
            .is_none());
    }
    #[test]
    fn legacy_activity_and_existing_link_bootstrap_ids_are_stable_without_row_renames() {
        let mut f = Fixture::new();
        let entity = Uuid::new_v4();
        f.db.connection.execute("INSERT INTO activity_events VALUES('1234567890abcdef1234567890abcdef','created','task',?1,'Legacy fictional history','2026-10-10T00:00:00Z')",[entity.to_string()]).unwrap();
        let a = f
            .db
            .seal_sync_v2_existing_activity(f.scope, "1234567890abcdef1234567890abcdef", 1, &KEY)
            .unwrap();
        let b = f
            .db
            .seal_sync_v2_existing_activity(f.scope, "1234567890abcdef1234567890abcdef", 1, &KEY)
            .unwrap();
        assert_eq!(a.header.record_id, b.header.record_id);
        assert_eq!(
            open_record(&a, &KEY).unwrap(),
            open_record(&b, &KEY).unwrap()
        );
        f.db.mutate_sync_v2_live(f.scope, &a, &KEY).unwrap();
        assert_eq!(count(&f, "activity_events"), 1);
        assert_eq!(
            f.db.connection
                .query_row("SELECT id FROM activity_events", [], |r| r
                    .get::<_, String>(0))
                .unwrap(),
            "1234567890abcdef1234567890abcdef"
        );
        let mut no_pending = Fixture::new();
        let p2 = project(&no_pending);
        let t2 = task_record(p2, None);
        let r2 = rfi_record(p2);
        let page = vec![t2.clone(), r2.clone()];
        no_pending.run(0, &page, &page).unwrap();
        no_pending
            .db
            .connection
            .execute(
                "INSERT INTO rfi_task_relationships VALUES(?1,?2,'2026-10-10T00:00:00Z')",
                params![
                    r2.header.record_id.to_string(),
                    t2.header.record_id.to_string()
                ],
            )
            .unwrap();
        let first = no_pending
            .db
            .seal_sync_v2_existing_link(
                no_pending.scope,
                5,
                r2.header.record_id,
                t2.header.record_id,
                1,
                &KEY,
            )
            .unwrap();
        let again = no_pending
            .db
            .seal_sync_v2_existing_link(
                no_pending.scope,
                5,
                r2.header.record_id,
                t2.header.record_id,
                1,
                &KEY,
            )
            .unwrap();
        assert_eq!(first.header.record_id, again.header.record_id);
        no_pending
            .db
            .mutate_sync_v2_live(no_pending.scope, &first, &KEY)
            .unwrap();
        assert_eq!(count(&no_pending, "rfi_task_relationships"), 1);
    }
    fn rfi_record(project: Uuid) -> SealedRecord {
        let h = header(3);
        seal_record(h.clone(),&KEY,&RecordContent {schema_version:1,fields:Some(json!({"id":h.record_id.to_string(),"project_id":project.to_string(),"number":"F-RFI-1","subject":"Fictional RFI","question":"Fictional question","recipient":"Fictional engineer","status":"open","created_date":"2026-10-10","submitted_date":"2026-10-10","response_due_date":"2026-10-15","response_received_date":null,"response":null,"notes":"Fictional notes","rfi_location":"Fictional location","drawing_number":"F-DWG-1","cost_impact":"None","time_delay":"None","suggested_solution":"Fictional suggestion","requested_by":"Fictional requester","created_at_utc":"2026-10-10T00:00:00Z","updated_at_utc":"2026-10-10T00:00:00Z"}))}).unwrap()
    }
    fn submittal_record(project: Uuid, parent: Option<Uuid>, revision: &str) -> SealedRecord {
        let h = header(4);
        seal_record(h.clone(),&KEY,&RecordContent {schema_version:1,fields:Some(json!({"id":h.record_id.to_string(),"project_id":project.to_string(),"parent_submittal_id":parent.map(|p|p.to_string()),"number":"F-SUB-1","name":"Fictional submittal","package":"Fictional package","revision":revision,"recipient":"Fictional reviewer","status":"approved","disposition":"approved","created_date":"2026-10-10","submitted_date":"2026-10-10","response_date":"2026-10-10","resubmission_required":false,"notes":"Fictional notes","created_at_utc":"2026-10-10T00:00:00Z","updated_at_utc":"2026-10-10T00:00:00Z"}))}).unwrap()
    }
    fn next_record(record: &SealedRecord, change: impl FnOnce(&mut Value)) -> SealedRecord {
        let mut h = record.header.clone();
        h.expected_server_version += 1;
        h.mutation_id = Uuid::new_v4();
        let mut content = open_record(record, &KEY).unwrap();
        change(content.fields.as_mut().unwrap());
        seal_record(h, &KEY, &content).unwrap()
    }
    #[test]
    fn register_fields_parent_order_backup_and_exact_retry() {
        let mut f = Fixture::new();
        let p = project(&f);
        let r = rfi_record(p);
        let parent = submittal_record(p, None, "0");
        let child = submittal_record(p, Some(parent.header.record_id), "1");
        let rows = [child.clone(), r.clone(), parent.clone()];
        let backup = f.run(0, &rows, &rows).unwrap();
        let preview = recovery::preview(Path::new(&backup.path)).unwrap();
        assert_eq!(preview.rfis, 0);
        assert_eq!(preview.submittals, 0);
        for row in &rows {
            assert_eq!(
                live(
                    &f.db.connection,
                    row.header.record_kind,
                    row.header.record_id
                )
                .unwrap(),
                open_record(row, &KEY).unwrap().fields
            );
        }
        f.run(0, &rows, &rows).unwrap();
        assert_eq!(cursor(&f), 3);
    }
    #[test]
    fn invalid_register_lifecycle_refuses_entire_page() {
        let mut f = Fixture::new();
        let p = project(&f);
        let r = rfi_record(p);
        let sub = submittal_record(p, None, "0");
        for (row, field, value) in [
            (&r, "recipient", Value::Null),
            (&r, "question", json!(" ")),
            (&sub, "response_date", Value::Null),
            (&sub, "disposition", json!("rejected")),
        ] {
            let mut content = open_record(row, &KEY).unwrap();
            content.fields.as_mut().unwrap()[field] = value;
            let invalid = seal_record(row.header.clone(), &KEY, &content).unwrap();
            let note = record("Must not partially apply");
            assert!(f
                .run(
                    0,
                    &[note.clone(), invalid.clone()],
                    &[note.clone(), invalid]
                )
                .is_err());
            assert_eq!(cursor(&f), 0);
            assert!(live(&f.db.connection, 10, note.header.record_id)
                .unwrap()
                .is_none());
        }
        let mut content = open_record(&r, &KEY).unwrap();
        content.fields.as_mut().unwrap()["status"] = json!("closed");
        let closed = seal_record(r.header.clone(), &KEY, &content).unwrap();
        assert!(f.run(0, &[closed.clone()], &[closed]).is_err());
        assert_eq!(cursor(&f), 0);
    }
    #[test]
    fn submittal_parent_cycles_missing_and_cross_project_refused() {
        let mut f = Fixture::new();
        let p = project(&f);
        let missing = submittal_record(p, Some(Uuid::new_v4()), "0");
        assert!(f.run(0, &[missing.clone()], &[missing]).is_err());
        let parent = submittal_record(p, None, "0");
        let other = Uuid::new_v4();
        f.db.connection.execute("INSERT INTO projects(id,number,name,project_path,created_at_utc,updated_at_utc) VALUES(?1,'F-PARENT-OTHER','Fictional other','C:\\Fictional\\ParentOther','2026-10-10T00:00:00Z','2026-10-10T00:00:00Z')",[other.to_string()]).unwrap();
        let child = submittal_record(other, Some(parent.header.record_id), "1");
        assert!(f
            .run(
                0,
                &[parent.clone(), child.clone()],
                &[parent.clone(), child]
            )
            .is_err());
        let child = submittal_record(p, Some(parent.header.record_id), "1");
        let mut content = open_record(&parent, &KEY).unwrap();
        content.fields.as_mut().unwrap()["parent_submittal_id"] =
            json!(child.header.record_id.to_string());
        let cyclic = seal_record(parent.header.clone(), &KEY, &content).unwrap();
        assert!(f
            .run(0, &[cyclic.clone(), child.clone()], &[cyclic, child])
            .is_err());
        let mut content = open_record(&parent, &KEY).unwrap();
        content.fields.as_mut().unwrap()["parent_submittal_id"] =
            json!(parent.header.record_id.to_string());
        let self_parent = seal_record(parent.header.clone(), &KEY, &content).unwrap();
        assert!(f.run(0, &[self_parent.clone()], &[self_parent]).is_err());
        assert_eq!(cursor(&f), 0);
        let child = submittal_record(p, Some(parent.header.record_id), "1");
        f.run(
            0,
            &[parent.clone(), child.clone()],
            &[parent.clone(), child.clone()],
        )
        .unwrap();
        let cyclic_update = next_record(&parent, |v| {
            v["parent_submittal_id"] = json!(child.header.record_id.to_string())
        });
        assert!(f
            .run(2, &[cyclic_update.clone()], &[cyclic_update, child])
            .is_err());
        assert_eq!(cursor(&f), 2);
        assert!(live(&f.db.connection, 4, parent.header.record_id)
            .unwrap()
            .unwrap()["parent_submittal_id"]
            .is_null());
    }
    #[test]
    fn register_local_edits_and_unknown_project_are_preserved() {
        let mut f = Fixture::new();
        let missing = rfi_record(Uuid::new_v4());
        assert!(f.run(0, &[missing.clone()], &[missing]).is_err());
        let p = project(&f);
        let r = rfi_record(p);
        let s = submittal_record(p, None, "0");
        f.run(0, &[r.clone(), s.clone()], &[r.clone(), s.clone()])
            .unwrap();
        f.db.connection
            .execute("UPDATE rfis SET question='Local RFI edit'", [])
            .unwrap();
        let remote = next_record(&r, |v| v["question"] = json!("Remote RFI edit"));
        assert!(f.run(2, &[remote.clone()], &[remote, s.clone()]).is_err());
        f.db.connection
            .execute("UPDATE submittals SET notes='Local submittal edit'", [])
            .unwrap();
        let remote = next_record(&s, |v| v["notes"] = json!("Remote submittal edit"));
        assert!(f.run(2, &[remote.clone()], &[r.clone(), remote]).is_err());
        assert_eq!(cursor(&f), 2);
        assert_eq!(
            live(&f.db.connection, 3, r.header.record_id)
                .unwrap()
                .unwrap()["question"],
            "Local RFI edit"
        );
        assert_eq!(
            live(&f.db.connection, 4, s.header.record_id)
                .unwrap()
                .unwrap()["notes"],
            "Local submittal edit"
        );
    }
    #[test]
    fn register_tombstones_refuse_cascades_and_preserve_documents() {
        let mut f = Fixture::new();
        let p = project(&f);
        let r = rfi_record(p);
        let s = submittal_record(p, None, "0");
        f.run(0, &[r.clone(), s.clone()], &[r.clone(), s.clone()])
            .unwrap();
        let path = f.root.join("fictional-sentinel.txt");
        std::fs::write(&path, b"Fictional document").unwrap();
        f.db.connection.execute("INSERT INTO rfi_attachment_references(id,rfi_id,file_path,display_name,created_at_utc) VALUES(?1,?2,?3,'Fictional','2026-10-10T00:00:00Z')",params![Uuid::new_v4().to_string(),r.header.record_id.to_string(),path.to_str().unwrap()]).unwrap();
        f.db.connection.execute("INSERT INTO submittal_attachment_references(id,submittal_id,file_path,display_name,created_at_utc) VALUES(?1,?2,?3,'Fictional','2026-10-10T00:00:00Z')",params![Uuid::new_v4().to_string(),s.header.record_id.to_string(),path.to_str().unwrap()]).unwrap();
        let rd = tombstone(&r);
        let sd = tombstone(&s);
        assert!(f.run(2, &[rd.clone()], &[rd, s.clone()]).is_err());
        assert!(f.run(2, &[sd.clone()], &[r.clone(), sd]).is_err());
        assert_eq!(cursor(&f), 2);
        assert_eq!(std::fs::read(path).unwrap(), b"Fictional document");
        for table in [
            "rfi_attachment_references",
            "submittal_attachment_references",
        ] {
            assert_eq!(
                f.db.connection
                    .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r
                        .get::<_, i64>(0))
                    .unwrap(),
                1
            );
        }
    }
    #[test]
    fn linked_register_reassignment_and_deletion_refused() {
        let mut f = Fixture::new();
        let p = project(&f);
        let r = rfi_record(p);
        let s = submittal_record(p, None, "0");
        let t = task_record(p, None);
        f.run(
            0,
            &[r.clone(), s.clone(), t.clone()],
            &[r.clone(), s.clone(), t.clone()],
        )
        .unwrap();
        f.db.connection.execute("INSERT INTO rfi_task_relationships(rfi_id,task_id,created_at_utc) VALUES(?1,?2,'2026-10-10T00:00:00Z')",params![r.header.record_id.to_string(),t.header.record_id.to_string()]).unwrap();
        f.db.connection.execute("INSERT INTO submittal_task_relationships(submittal_id,task_id,created_at_utc) VALUES(?1,?2,'2026-10-10T00:00:00Z')",params![s.header.record_id.to_string(),t.header.record_id.to_string()]).unwrap();
        let other = Uuid::new_v4();
        f.db.connection.execute("INSERT INTO projects(id,number,name,project_path,created_at_utc,updated_at_utc) VALUES(?1,'F-OTHER','Fictional other','C:\\Fictional\\Other','2026-10-10T00:00:00Z','2026-10-10T00:00:00Z')",[other.to_string()]).unwrap();
        for row in [&r, &s] {
            let moved = next_record(row, |v| v["project_id"] = json!(other.to_string()));
            let manifest = if row.header.record_kind == 3 {
                vec![moved.clone(), s.clone(), t.clone()]
            } else {
                vec![r.clone(), moved.clone(), t.clone()]
            };
            assert!(f.run(3, &[moved], &manifest).is_err());
            let deleted = tombstone(row);
            let manifest = if row.header.record_kind == 3 {
                vec![deleted.clone(), s.clone(), t.clone()]
            } else {
                vec![r.clone(), deleted.clone(), t.clone()]
            };
            assert!(f.run(3, &[deleted], &manifest).is_err());
        }
        assert_eq!(cursor(&f), 3);
    }
    #[test]
    fn register_uniqueness_and_parent_delete_preserve_all_rows() {
        let mut f = Fixture::new();
        let p = project(&f);
        let r = rfi_record(p);
        let duplicate = rfi_record(p);
        assert!(f
            .run(0, &[r.clone(), duplicate.clone()], &[r.clone(), duplicate])
            .is_err());
        let parent = submittal_record(p, None, "0");
        let child = submittal_record(p, Some(parent.header.record_id), "1");
        f.run(
            0,
            &[r.clone(), parent.clone(), child.clone()],
            &[r.clone(), parent.clone(), child.clone()],
        )
        .unwrap();
        let deleted = tombstone(&parent);
        assert!(f
            .run(3, &[deleted.clone()], &[r.clone(), deleted, child.clone()])
            .is_err());
        assert_eq!(cursor(&f), 3);
        assert!(live(&f.db.connection, 4, parent.header.record_id)
            .unwrap()
            .is_some());
    }
    #[test]
    fn valid_register_updates_and_unreferenced_tombstones_are_recoverable() {
        let mut f = Fixture::new();
        let p = project(&f);
        let r = rfi_record(p);
        let s = submittal_record(p, None, "0");
        f.run(0, &[r.clone(), s.clone()], &[r.clone(), s.clone()])
            .unwrap();
        let closed = next_record(&r, |v| {
            v["status"] = json!("closed");
            v["response"] = json!("Fictional answer");
            v["response_received_date"] = json!("2026-10-10");
        });
        let closed_s = next_record(&s, |v| v["status"] = json!("closed"));
        f.run(
            2,
            &[closed.clone(), closed_s.clone()],
            &[closed.clone(), closed_s.clone()],
        )
        .unwrap();
        assert_eq!(
            live(&f.db.connection, 3, r.header.record_id).unwrap(),
            open_record(&closed, &KEY).unwrap().fields
        );
        let rd = tombstone(&closed);
        let sd = tombstone(&closed_s);
        let backup = f.run(4, &[rd.clone(), sd.clone()], &[rd, sd]).unwrap();
        let preview = recovery::preview(Path::new(&backup.path)).unwrap();
        assert_eq!(preview.rfis, 1);
        assert_eq!(preview.submittals, 1);
        assert!(live(&f.db.connection, 3, r.header.record_id)
            .unwrap()
            .is_none());
        assert!(live(&f.db.connection, 4, s.header.record_id)
            .unwrap()
            .is_none());
        assert_eq!(cursor(&f), 6);
    }
}

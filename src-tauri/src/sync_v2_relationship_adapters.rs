//! Dormant composite-key task links. Opaque record identities never rebind.
#![allow(dead_code)]
use crate::{error::AppResult, sync_v2_record_codec::invalid};
use rusqlite::{params, Connection, OptionalExtension, Transaction};
use serde_json::{json, Value};
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Binding {
    pub kind: u8,
    pub register_id: String,
    pub task_id: String,
}
fn schema(kind: u8) -> AppResult<(&'static str, &'static str, &'static str)> {
    match kind {
        5 => Ok(("rfi_task_relationships", "rfi_id", "rfis")),
        6 => Ok(("submittal_task_relationships", "submittal_id", "submittals")),
        _ => Err(invalid()),
    }
}
pub(crate) fn from_fields(kind: u8, fields: &Value) -> AppResult<Binding> {
    let (_, field, _) = schema(kind)?;
    let id = |field: &str| -> AppResult<String> {
        let value = fields[field].as_str().ok_or_else(invalid)?;
        if Uuid::parse_str(value).map_err(|_| invalid())?.to_string() != value {
            return Err(invalid());
        }
        Ok(value.to_owned())
    };
    Ok(Binding {
        kind,
        register_id: id(field)?,
        task_id: id("task_id")?,
    })
}
pub(crate) fn stored(db: &Connection, workspace: Uuid, record: Uuid) -> AppResult<Option<Binding>> {
    db.query_row("SELECT record_kind,register_id,task_id FROM sync_v2_relationship_bindings WHERE workspace_id=?1 AND record_id=?2", params![workspace.to_string(),record.to_string()], |r| Ok(Binding {kind:r.get(0)?,register_id:r.get(1)?,task_id:r.get(2)?})).optional().map_err(|_|invalid())
}
pub(crate) fn live(db: &Connection, binding: &Binding) -> AppResult<Option<Value>> {
    let (table, field, _) = schema(binding.kind)?;
    db.query_row(&format!("SELECT created_at_utc FROM {table} WHERE {field}=?1 AND task_id=?2"), params![binding.register_id,binding.task_id], |r| {
        Ok(json!({field:binding.register_id,"task_id":binding.task_id,"created_at_utc":r.get::<_,String>(0)?}))
    }).optional().map_err(|_|invalid())
}
pub(crate) fn bind(
    tx: &Transaction<'_>,
    workspace: Uuid,
    record: Uuid,
    binding: &Binding,
) -> AppResult<()> {
    if let Some(existing) = stored(tx, workspace, record)? {
        if existing != *binding {
            return Err(invalid());
        }
    } else {
        tx.execute("INSERT INTO sync_v2_relationship_bindings(workspace_id,record_id,record_kind,register_id,task_id) VALUES(?1,?2,?3,?4,?5)",params![workspace.to_string(),record.to_string(),binding.kind,binding.register_id,binding.task_id]).map_err(|_|invalid())?;
    }
    Ok(())
}
pub(crate) fn apply(
    tx: &Transaction<'_>,
    binding: &Binding,
    fields: Option<&Value>,
) -> AppResult<()> {
    let (table, field, register) = schema(binding.kind)?;
    if let Some(fields) = fields {
        if from_fields(binding.kind, fields)? != *binding {
            return Err(invalid());
        }
        let same_project: bool = tx.query_row(&format!("SELECT EXISTS(SELECT 1 FROM {register} r JOIN tasks t ON t.project_id=r.project_id WHERE r.id=?1 AND t.id=?2)"),params![binding.register_id,binding.task_id],|r|r.get(0)).map_err(|_|invalid())?;
        if !same_project {
            return Err(invalid());
        }
        let created = fields["created_at_utc"].as_str().ok_or_else(invalid)?;
        tx.execute(&format!("INSERT INTO {table}({field},task_id,created_at_utc) VALUES(?1,?2,?3) ON CONFLICT({field},task_id) DO UPDATE SET created_at_utc=excluded.created_at_utc"),params![binding.register_id,binding.task_id,created]).map_err(|_|invalid())?;
    } else {
        tx.execute(
            &format!("DELETE FROM {table} WHERE {field}=?1 AND task_id=?2"),
            params![binding.register_id, binding.task_id],
        )
        .map_err(|_| invalid())?;
    }
    Ok(())
}

//! Explicit dormant adapters for reference metadata, activity and operational rows.
#![allow(dead_code)]
use crate::{error::AppResult, sync_v2_record_codec::invalid};
use rusqlite::{params, types::Value as SqlValue, Connection, OptionalExtension, Transaction};
use serde_json::{json, Value};
use uuid::Uuid;

const RFI: &str = "id rfi_id file_path display_name created_at_utc";
const SUB: &str = "id submittal_id file_path display_name created_at_utc";
const FILE:&str="id project_id file_name file_path operation discipline drawing_number title revision revision_date received_date is_current created_at_utc updated_at_utc";
const ACTIVITY: &str = "id event_type entity_type entity_id summary occurred_at_utc";
const WORK:&str="id project_id item_type number title description status priority due_date occurred_date responsible_party company location amount_cents checklist_total checklist_completed archived_at_utc created_at_utc updated_at_utc";
const TEMPLATE: &str =
    "id name task_titles_json milestone_titles_json created_at_utc updated_at_utc";
fn schema(kind: u8, subtype: &str) -> AppResult<(&'static str, &'static str)> {
    match (kind, subtype) {
        (7, _) => Ok(("rfi_attachment_references", RFI)),
        (8, _) => Ok(("submittal_attachment_references", SUB)),
        (9, _) => Ok(("registered_files", FILE)),
        (12, _) => Ok(("activity_events", ACTIVITY)),
        (13, "work_item") => Ok(("work_items", WORK)),
        (13, "project_template") => Ok(("project_templates", TEMPLATE)),
        _ => Err(invalid()),
    }
}
fn row(db: &Connection, kind: u8, subtype: &str, id: Uuid) -> AppResult<Option<Value>> {
    let (table, columns) = schema(kind, subtype)?;
    let names = columns.split_whitespace().collect::<Vec<_>>();
    let local_id = if kind == 12 {
        db.query_row(
            "SELECT local_id FROM sync_v2_activity_bindings WHERE record_id=?1",
            [id.to_string()],
            |r| r.get::<_, String>(0),
        )
        .optional()
        .map_err(|_| invalid())?
        .unwrap_or(id.to_string())
    } else {
        id.to_string()
    };
    db.query_row(
        &format!("SELECT {} FROM {table} WHERE id=?1", names.join(",")),
        [local_id],
        |r| {
            let mut fields = serde_json::Map::new();
            for (i, name) in names.iter().enumerate() {
                let value = match r.get::<_, SqlValue>(i)? {
                    SqlValue::Null => Value::Null,
                    SqlValue::Text(v) => json!(v),
                    SqlValue::Integer(v) if *name == "is_current" => json!(v != 0),
                    SqlValue::Integer(v) => json!(v),
                    _ => return Err(rusqlite::Error::InvalidQuery),
                };
                fields.insert((*name).to_owned(), value);
            }
            if kind == 12 {
                fields.insert("id".into(), json!(id.to_string()));
            }
            if kind == 13 {
                fields.insert("subtype".into(), json!(subtype));
                if subtype == "project_template" {
                    for (stored, portable) in [
                        ("task_titles_json", "task_titles"),
                        ("milestone_titles_json", "milestone_titles"),
                    ] {
                        let value = fields.remove(stored).ok_or(rusqlite::Error::InvalidQuery)?;
                        let array: Value = serde_json::from_str(
                            value.as_str().ok_or(rusqlite::Error::InvalidQuery)?,
                        )
                        .map_err(|_| rusqlite::Error::InvalidQuery)?;
                        fields.insert(portable.into(), array);
                    }
                }
            }
            Ok(Value::Object(fields))
        },
    )
    .optional()
    .map_err(|_| invalid())
}
pub(crate) fn live(db: &Connection, kind: u8, id: Uuid) -> AppResult<Option<Value>> {
    if kind != 13 {
        return row(db, kind, "", id);
    }
    let work = row(db, 13, "work_item", id)?;
    let template = row(db, 13, "project_template", id)?;
    if work.is_some() && template.is_some() {
        return Err(invalid());
    }
    Ok(work.or(template))
}
pub(crate) fn project_fields(
    mut fields: Option<Value>,
    root: &std::path::Path,
    id: Uuid,
) -> AppResult<Option<Value>> {
    if let Some(f) = &mut fields {
        let path = crate::sync_v2_project_paths::reference_path(root, &f["portable_path"], id)?;
        let o = f.as_object_mut().ok_or_else(invalid)?;
        o.remove("portable_path");
        o.insert("file_path".into(), json!(path));
    }
    Ok(fields)
}
pub(crate) fn validate(kind: u8, f: &Value) -> AppResult<()> {
    let text = |n: &str| f[n].as_str().ok_or_else(invalid);
    let optional = |n: &str| -> AppResult<Option<String>> {
        if f[n].is_null() {
            Ok(None)
        } else {
            Ok(Some(text(n)?.to_owned()))
        }
    };
    if kind == 13 && f["subtype"] == "work_item" {
        crate::work_item::validate(&crate::work_item::WorkItemInput {
            project_id: text("project_id")?.into(),
            item_type: text("item_type")?.into(),
            number: optional("number")?,
            title: text("title")?.into(),
            description: optional("description")?,
            status: text("status")?.into(),
            priority: text("priority")?.into(),
            due_date: optional("due_date")?,
            occurred_date: optional("occurred_date")?,
            responsible_party: optional("responsible_party")?,
            company: optional("company")?,
            location: optional("location")?,
            amount_cents: f["amount_cents"].as_i64(),
            checklist_total: f["checklist_total"].as_i64().ok_or_else(invalid)?,
            checklist_completed: f["checklist_completed"].as_i64().ok_or_else(invalid)?,
        })
        .map_err(|_| invalid())?;
    }
    if kind == 13 && f["subtype"] == "project_template" {
        for field in ["task_titles", "milestone_titles"] {
            if f[field]
                .as_array()
                .ok_or_else(invalid)?
                .iter()
                .any(|v| v.as_str().is_none_or(|s| s.trim().is_empty()))
            {
                return Err(invalid());
            }
        }
    }
    if kind == 12
        && ["event_type", "entity_type"]
            .iter()
            .any(|n| f[*n].as_str().is_none_or(|s| s.trim().is_empty()))
    {
        return Err(invalid());
    }
    Ok(())
}
pub(crate) fn apply(
    tx: &Transaction<'_>,
    kind: u8,
    id: Uuid,
    fields: Option<&Value>,
) -> AppResult<()> {
    let existing = live(tx, kind, id)?;
    let subtype = fields
        .or(existing.as_ref())
        .and_then(|f| f["subtype"].as_str())
        .unwrap_or("");
    if kind == 13 && fields.is_none() && existing.is_none() {
        return Ok(());
    }
    let (table, columns) = schema(kind, subtype)?;
    let local_id = if kind == 12 {
        tx.query_row(
            "SELECT local_id FROM sync_v2_activity_bindings WHERE record_id=?1",
            [id.to_string()],
            |r| r.get::<_, String>(0),
        )
        .optional()
        .map_err(|_| invalid())?
        .unwrap_or(id.to_string())
    } else {
        id.to_string()
    };
    if let Some(f) = fields {
        validate(kind, f)?;
        if kind == 13
            && existing
                .as_ref()
                .is_some_and(|old| old["subtype"] != f["subtype"])
        {
            return Err(invalid());
        }
        // History records may be removed explicitly, but never rewritten silently.
        if kind == 12 && existing.as_ref().is_some_and(|old| old != f) {
            return Err(invalid());
        }
        let names = columns.split_whitespace().collect::<Vec<_>>();
        let values = names
            .iter()
            .map(|name| -> AppResult<SqlValue> {
                if *name == "id" {
                    return Ok(SqlValue::Text(local_id.clone()));
                }
                if *name == "task_titles_json" || *name == "milestone_titles_json" {
                    return Ok(SqlValue::Text(
                        serde_json::to_string(&f[name.trim_end_matches("_json")])
                            .map_err(|_| invalid())?,
                    ));
                }
                match &f[*name] {
                    Value::Null => Ok(SqlValue::Null),
                    Value::String(v) => Ok(SqlValue::Text(v.clone())),
                    Value::Bool(v) => Ok(SqlValue::Integer(i64::from(*v))),
                    Value::Number(v) => Ok(SqlValue::Integer(v.as_i64().ok_or_else(invalid)?)),
                    _ => Err(invalid()),
                }
            })
            .collect::<AppResult<Vec<_>>>()?;
        let placeholders = (1..=names.len())
            .map(|i| format!("?{i}"))
            .collect::<Vec<_>>()
            .join(",");
        let update = names
            .iter()
            .skip(1)
            .map(|n| format!("{n}=excluded.{n}"))
            .collect::<Vec<_>>()
            .join(",");
        tx.execute(&format!("INSERT INTO {table}({}) VALUES({placeholders}) ON CONFLICT(id) DO UPDATE SET {update}",names.join(",")),rusqlite::params_from_iter(values)).map_err(|_|invalid())?;
    } else {
        tx.execute(
            &format!("DELETE FROM {table} WHERE id=?1"),
            params![local_id],
        )
        .map_err(|_| invalid())?;
    }
    Ok(())
}

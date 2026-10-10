//! Dormant C10-04C4 RFI/submittal metadata adapters. Explicit local allowlists only.
#![allow(dead_code)]
use crate::{error::AppResult, sync_v2_record_codec::invalid};
use rusqlite::{
    params_from_iter,
    types::{Value as SqlValue, ValueRef},
    Connection, OptionalExtension, Transaction,
};
use serde_json::{Map, Value};
use uuid::Uuid;

const RFI:&str="id project_id number subject question recipient status created_date submitted_date response_due_date response_received_date response notes rfi_location drawing_number cost_impact time_delay suggested_solution requested_by created_at_utc updated_at_utc";
const SUBMITTAL:&str="id project_id parent_submittal_id number name package revision recipient status disposition created_date submitted_date response_date resubmission_required notes created_at_utc updated_at_utc";
fn schema(kind: u8) -> AppResult<(&'static str, &'static str)> {
    match kind {
        3 => Ok(("rfis", RFI)),
        4 => Ok(("submittals", SUBMITTAL)),
        _ => Err(invalid()),
    }
}
fn sql(_: rusqlite::Error) -> crate::error::AppError {
    invalid()
}
fn text<'a>(f: &'a Value, n: &str) -> AppResult<&'a str> {
    f[n].as_str().ok_or_else(invalid)
}
fn optional(f: &Value, n: &str) -> Option<String> {
    f[n].as_str().map(str::to_owned)
}
pub(crate) fn validate(kind: u8, f: &Value) -> AppResult<()> {
    match kind {
        3 => crate::rfi::validate(&crate::rfi::RfiInput {
            project_id: text(f, "project_id")?.to_owned(),
            number: text(f, "number")?.to_owned(),
            subject: text(f, "subject")?.to_owned(),
            question: text(f, "question")?.to_owned(),
            recipient: optional(f, "recipient"),
            status: text(f, "status")?.to_owned(),
            created_date: text(f, "created_date")?.to_owned(),
            submitted_date: optional(f, "submitted_date"),
            response_due_date: optional(f, "response_due_date"),
            response_received_date: optional(f, "response_received_date"),
            response: optional(f, "response"),
            notes: optional(f, "notes"),
            rfi_location: optional(f, "rfi_location"),
            drawing_number: optional(f, "drawing_number"),
            cost_impact: optional(f, "cost_impact"),
            time_delay: optional(f, "time_delay"),
            suggested_solution: optional(f, "suggested_solution"),
            requested_by: optional(f, "requested_by"),
            related_task_id: None,
        })
        .map_err(|_| invalid()),
        4 => {
            crate::submittal::validate(&crate::submittal::SubmittalInput {
                project_id: text(f, "project_id")?.to_owned(),
                parent_submittal_id: optional(f, "parent_submittal_id"),
                number: text(f, "number")?.to_owned(),
                name: text(f, "name")?.to_owned(),
                package: optional(f, "package"),
                revision: Some(text(f, "revision")?.to_owned()),
                recipient: optional(f, "recipient"),
                status: text(f, "status")?.to_owned(),
                created_date: text(f, "created_date")?.to_owned(),
                submitted_date: optional(f, "submitted_date"),
                response_date: optional(f, "response_date"),
                resubmission_required: f["resubmission_required"].as_bool().ok_or_else(invalid)?,
                notes: optional(f, "notes"),
                related_task_id: None,
            })
            .map_err(|_| invalid())?;
            if let Some(disposition) = crate::submittal::disposition_for_status(text(f, "status")?)
            {
                if f["disposition"].as_str() != Some(disposition) {
                    return Err(invalid());
                }
            }
            Ok(())
        }
        _ => Err(invalid()),
    }
}
pub(crate) fn live(connection: &Connection, kind: u8, id: Uuid) -> AppResult<Option<Value>> {
    let (table, columns) = schema(kind)?;
    let names = columns.split_whitespace().collect::<Vec<_>>();
    // Identifiers derive exclusively from the two compile-time lists above.
    connection
        .query_row(
            &format!("SELECT {} FROM {table} WHERE id=?1", names.join(",")),
            [id.to_string()],
            |row| {
                let mut values = Map::new();
                for (index, name) in names.iter().enumerate() {
                    let value = if *name == "resubmission_required" {
                        Value::Bool(row.get(index)?)
                    } else {
                        match row.get_ref(index)? {
                            ValueRef::Null => Value::Null,
                            ValueRef::Text(bytes) => Value::String(
                                std::str::from_utf8(bytes)
                                    .map_err(|_| rusqlite::Error::InvalidQuery)?
                                    .to_owned(),
                            ),
                            _ => return Err(rusqlite::Error::InvalidQuery),
                        }
                    };
                    values.insert((*name).to_owned(), value);
                }
                Ok(Value::Object(values))
            },
        )
        .optional()
        .map_err(sql)
}
pub(crate) fn upsert(tx: &Transaction<'_>, kind: u8, f: &Value) -> AppResult<()> {
    validate(kind, f)?;
    let (table, columns) = schema(kind)?;
    let names = columns.split_whitespace().collect::<Vec<_>>();
    let values = names
        .iter()
        .map(|name| match &f[*name] {
            Value::Null => Ok(SqlValue::Null),
            Value::String(s) => Ok(SqlValue::Text(s.clone())),
            Value::Bool(b) if *name == "resubmission_required" => {
                Ok(SqlValue::Integer(i64::from(*b)))
            }
            _ => Err(invalid()),
        })
        .collect::<AppResult<Vec<_>>>()?;
    let placeholders = (1..=names.len())
        .map(|i| format!("?{i}"))
        .collect::<Vec<_>>()
        .join(",");
    let updates = names
        .iter()
        .skip(1)
        .map(|n| format!("{n}=excluded.{n}"))
        .collect::<Vec<_>>()
        .join(",");
    tx.execute(&format!("INSERT INTO {table}({}) VALUES({placeholders}) ON CONFLICT(id) DO UPDATE SET {updates}",names.join(",")),params_from_iter(values)).map_err(sql)?;
    Ok(())
}
pub(crate) fn delete(tx: &Transaction<'_>, kind: u8, id: Uuid) -> AppResult<()> {
    let (table, _) = schema(kind)?;
    let references=match kind {
        3=>"SELECT EXISTS(SELECT 1 FROM rfi_task_relationships WHERE rfi_id=?1) OR EXISTS(SELECT 1 FROM rfi_attachment_references WHERE rfi_id=?1)",
        4=>"SELECT EXISTS(SELECT 1 FROM submittal_task_relationships WHERE submittal_id=?1) OR EXISTS(SELECT 1 FROM submittal_attachment_references WHERE submittal_id=?1) OR EXISTS(SELECT 1 FROM submittals WHERE parent_submittal_id=?1)",
        _=>return Err(invalid()),
    };
    let referenced: bool = tx
        .query_row(references, [id.to_string()], |r| r.get(0))
        .map_err(sql)?;
    if referenced {
        return Err(invalid());
    }
    tx.execute(
        &format!("DELETE FROM {table} WHERE id=?1"),
        [id.to_string()],
    )
    .map_err(sql)?;
    Ok(())
}
pub(crate) fn validate_relationships(tx: &Transaction<'_>) -> AppResult<()> {
    for query in [
        "SELECT EXISTS(SELECT 1 FROM rfi_task_relationships l JOIN rfis r ON r.id=l.rfi_id JOIN tasks t ON t.id=l.task_id WHERE r.project_id<>t.project_id)",
        "SELECT EXISTS(SELECT 1 FROM submittal_task_relationships l JOIN submittals s ON s.id=l.submittal_id JOIN tasks t ON t.id=l.task_id WHERE s.project_id<>t.project_id)",
        "SELECT EXISTS(SELECT 1 FROM submittals c JOIN submittals p ON p.id=c.parent_submittal_id WHERE c.project_id<>p.project_id)",
    ] {if tx.query_row(query,[],|r|r.get::<_,bool>(0)).map_err(sql)? {return Err(invalid());}}
    let mut statement = tx
        .prepare("SELECT id,parent_submittal_id FROM submittals")
        .map_err(sql)?;
    let parents = statement
        .query_map([], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, Option<String>>(1)?))
        })
        .map_err(sql)?
        .collect::<Result<std::collections::HashMap<_, _>, _>>()
        .map_err(sql)?;
    // Includes existing descendants, so changing a parent cannot introduce a
    // cycle or inconsistent ancestry outside the received page.
    let mut complete = std::collections::HashSet::new();
    for id in parents.keys() {
        let mut seen = std::collections::HashSet::new();
        let mut current = Some(id);
        while let Some(id) = current {
            if complete.contains(id) {
                break;
            }
            if !seen.insert(id.clone()) {
                return Err(invalid());
            }
            current = parents.get(id).and_then(Option::as_ref);
        }
        complete.extend(seen);
    }
    Ok(())
}

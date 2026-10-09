use std::{collections::BTreeMap, path::Path};

use reqwest::blocking::Client;
use rusqlite::types::{Value as SqlValue, ValueRef};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use uuid::Uuid;

use crate::{
    cloud_auth,
    error::{AppError, AppResult},
    persistence::Database,
};

// Legacy Sync is intentionally disabled. Do not compile live legacy service
// coordinates into release artifacts while MetadataSync remains off.
const PROJECT_URL: &str = "https://legacy-sync-disabled.invalid";
const PUBLISHABLE_KEY: &str = "legacy-sync-disabled";
const TABLES: &[&str] = &[
    "projects",
    "tasks",
    "rfis",
    "submittals",
    "rfi_task_relationships",
    "submittal_task_relationships",
    "rfi_attachment_references",
    "submittal_attachment_references",
    "registered_files",
    "notes",
    "contacts",
    "activity_events",
    "work_items",
    "project_templates",
];
const PATH_FIELDS: &[(&str, &str)] = &[
    ("projects", "project_path"),
    ("rfi_attachment_references", "file_path"),
    ("submittal_attachment_references", "file_path"),
    ("registered_files", "file_path"),
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceSnapshot {
    schema_version: i64,
    tables: BTreeMap<String, Vec<Map<String, Value>>>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncResult {
    pub outcome: String,
    pub message: String,
    pub cloud_version: i64,
    pub conflict_count: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncConflict {
    pub id: String,
    pub created_at_utc: String,
}

#[derive(Deserialize)]
struct RemoteRecord {
    version: i64,
    payload: WorkspaceSnapshot,
}

#[derive(Deserialize)]
struct PushResponse {
    outcome: String,
    version: Option<i64>,
    record: Option<PushConflictRecord>,
}

#[derive(Deserialize)]
struct PushConflictRecord {
    version: i64,
    payload: WorkspaceSnapshot,
}

fn sync_error(code: &'static str, message: &str, detail: impl Into<String>) -> AppError {
    AppError::from_technical(
        code,
        message,
        "Check your connection and sign-in, then try again. Local data was not changed.",
        detail,
    )
}

fn value_from_ref(value: ValueRef<'_>) -> AppResult<Value> {
    Ok(match value {
        ValueRef::Null => Value::Null,
        ValueRef::Integer(v) => Value::from(v),
        ValueRef::Real(v) => Value::from(v),
        ValueRef::Text(v) => Value::String(String::from_utf8_lossy(v).into_owned()),
        ValueRef::Blob(_) => {
            return Err(sync_error(
                "SYNC_PAYLOAD_UNSUPPORTED",
                "Local metadata contains a value that legacy Sync cannot preserve.",
                "BLOB values are not supported by the legacy snapshot format.",
            ))
        }
    })
}

fn sql_value(value: &Value) -> AppResult<SqlValue> {
    Ok(match value {
        Value::Null => SqlValue::Null,
        Value::Bool(v) => SqlValue::Integer(i64::from(*v)),
        Value::Number(v) if v.is_i64() => SqlValue::Integer(v.as_i64().unwrap_or_default()),
        Value::Number(v) => SqlValue::Real(v.as_f64().unwrap_or_default()),
        Value::String(v) => SqlValue::Text(v.clone()),
        _ => {
            return Err(sync_error(
                "SYNC_PAYLOAD_INVALID",
                "Cloud data could not be applied safely.",
                "Nested value in database row.",
            ))
        }
    })
}

fn portable_path(path: &str, root: &Path) -> String {
    let value = Path::new(path);
    if let Ok(relative) = value.strip_prefix(root) {
        format!("@root/{}", relative.to_string_lossy().replace('\\', "/"))
    } else {
        let name = value
            .file_name()
            .and_then(|v| v.to_str())
            .unwrap_or("unavailable-file");
        format!("@external/{name}")
    }
}

fn valid_portable_component(component: &str) -> bool {
    if component.is_empty()
        || component == "."
        || component == ".."
        || component.ends_with(['.', ' '])
        || component.chars().any(|value| {
            value.is_control() || matches!(value, '<' | '>' | ':' | '"' | '\\' | '|' | '?' | '*')
        })
    {
        return false;
    }
    let stem = component.split('.').next().unwrap_or_default();
    !matches!(
        stem.to_ascii_uppercase().as_str(),
        "CON"
            | "PRN"
            | "AUX"
            | "NUL"
            | "COM1"
            | "COM2"
            | "COM3"
            | "COM4"
            | "COM5"
            | "COM6"
            | "COM7"
            | "COM8"
            | "COM9"
            | "COM¹"
            | "COM²"
            | "COM³"
            | "LPT1"
            | "LPT2"
            | "LPT3"
            | "LPT4"
            | "LPT5"
            | "LPT6"
            | "LPT7"
            | "LPT8"
            | "LPT9"
            | "LPT¹"
            | "LPT²"
            | "LPT³"
    )
}

fn portable_components(value: &str, single_component: bool) -> AppResult<Vec<&str>> {
    let components = value.split('/').collect::<Vec<_>>();
    if components.is_empty()
        || (single_component && components.len() != 1)
        || components
            .iter()
            .any(|part| !valid_portable_component(part))
    {
        return Err(sync_error(
            "SYNC_PAYLOAD_INVALID",
            "Cloud data could not be applied safely.",
            "A synchronized file path was not a safe portable relative path.",
        ));
    }
    Ok(components)
}

fn local_path(path: &str, root: &Path) -> AppResult<String> {
    let rebased = if let Some(relative) = path.strip_prefix("@root/") {
        portable_components(relative, false)?
            .into_iter()
            .fold(root.to_path_buf(), |path, component| path.join(component))
    } else if let Some(name) = path.strip_prefix("@external/") {
        portable_components(name, true)?
            .into_iter()
            .fold(root.join(".anydesk-missing"), |path, component| {
                path.join(component)
            })
    } else {
        root.join(".anydesk-missing").join("unavailable-file")
    };
    Ok(rebased.to_string_lossy().into_owned())
}

impl Database {
    pub fn export_workspace_snapshot(&self) -> AppResult<WorkspaceSnapshot> {
        let root = self.get_project_root()?.path.ok_or_else(|| {
            sync_error(
                "PROJECT_ROOT_NOT_SET",
                "Set the dedicated OneDrive project root before syncing.",
                "No project root.",
            )
        })?;
        let root = Path::new(&root);
        let mut tables = BTreeMap::new();
        for table in TABLES {
            let mut statement = self
                .connection
                .prepare(&format!("SELECT * FROM {table}"))
                .map_err(db_error)?;
            let columns: Vec<String> = statement
                .column_names()
                .iter()
                .map(|v| (*v).to_string())
                .collect();
            let mut rows = statement.query([]).map_err(db_error)?;
            let mut values = Vec::new();
            while let Some(row) = rows.next().map_err(db_error)? {
                let mut object = Map::new();
                for (index, column) in columns.iter().enumerate() {
                    let mut value = value_from_ref(row.get_ref(index).map_err(db_error)?)?;
                    if PATH_FIELDS.contains(&(*table, column.as_str())) {
                        if let Some(path) = value.as_str() {
                            value = Value::String(portable_path(path, root));
                        }
                    }
                    object.insert(column.clone(), value);
                }
                values.push(object);
            }
            tables.insert((*table).to_string(), values);
        }
        Ok(WorkspaceSnapshot {
            schema_version: 1,
            tables,
        })
    }

    pub fn import_workspace_snapshot(&mut self, snapshot: &WorkspaceSnapshot) -> AppResult<()> {
        if snapshot.schema_version != 1
            || snapshot.tables.len() != TABLES.len()
            || TABLES
                .iter()
                .any(|required| !snapshot.tables.contains_key(*required))
            || snapshot
                .tables
                .keys()
                .any(|table| !TABLES.contains(&table.as_str()))
        {
            return Err(sync_error(
                "SYNC_SCHEMA_UNSUPPORTED",
                "Cloud data uses an unsupported schema.",
                "Unexpected snapshot schema or table.",
            ));
        }
        let root = self.get_project_root()?.path.ok_or_else(|| {
            sync_error(
                "PROJECT_ROOT_NOT_SET",
                "Set the dedicated OneDrive project root before syncing.",
                "No project root.",
            )
        })?;
        let root = Path::new(&root);
        let mut tables = snapshot.tables.clone();
        for (table, rows) in &mut tables {
            for row in rows {
                for (path_table, field) in PATH_FIELDS {
                    if *path_table == table.as_str() {
                        match row.get_mut(*field) {
                            Some(Value::String(path)) => *path = local_path(path, root)?,
                            Some(_) => {
                                return Err(sync_error(
                                    "SYNC_PAYLOAD_INVALID",
                                    "Cloud data could not be applied safely.",
                                    "A synchronized file path was not text.",
                                ));
                            }
                            None => {}
                        }
                    }
                }
            }
        }
        let transaction = self.connection.transaction().map_err(db_error)?;
        transaction
            .execute_batch("PRAGMA defer_foreign_keys=ON;")
            .map_err(db_error)?;
        for table in TABLES.iter().rev() {
            transaction
                .execute(&format!("DELETE FROM {table}"), [])
                .map_err(db_error)?;
        }
        for table in TABLES {
            let rows = tables.get(*table).cloned().unwrap_or_default();
            let allowed: Vec<String> = {
                let mut statement = transaction
                    .prepare(&format!("PRAGMA table_info({table})"))
                    .map_err(db_error)?;
                let columns = statement
                    .query_map([], |row| row.get::<_, String>(1))
                    .map_err(db_error)?
                    .collect::<Result<_, _>>()
                    .map_err(db_error)?;
                columns
            };
            for row in rows {
                if row.keys().any(|column| !allowed.contains(column)) {
                    return Err(sync_error(
                        "SYNC_PAYLOAD_INVALID",
                        "Cloud data could not be applied safely.",
                        format!("Invalid columns for {table}."),
                    ));
                }
                let placeholders = (1..=allowed.len())
                    .map(|index| format!("?{index}"))
                    .collect::<Vec<_>>()
                    .join(",");
                let sql = format!(
                    "INSERT INTO {table} ({}) VALUES ({placeholders})",
                    allowed.join(",")
                );
                let params = allowed
                    .iter()
                    .map(|column| sql_value(row.get(column).unwrap_or(&Value::Null)))
                    .collect::<AppResult<Vec<_>>>()?;
                transaction
                    .execute(&sql, rusqlite::params_from_iter(params))
                    .map_err(db_error)?;
            }
        }
        transaction.commit().map_err(db_error)
    }

    fn baseline(&self) -> AppResult<Option<(String, i64)>> {
        use rusqlite::OptionalExtension;
        self.connection.query_row("SELECT content_hash,cloud_version FROM sync_local_records WHERE entity_type='workspace' AND entity_id='primary'", [], |row| Ok((row.get(0)?, row.get(1)?))).optional().map_err(db_error)
    }

    fn save_baseline(&self, json: &str, version: i64) -> AppResult<()> {
        self.connection.execute("INSERT INTO sync_local_records(entity_type,entity_id,content_hash,cloud_version) VALUES('workspace','primary',?1,?2) ON CONFLICT(entity_type,entity_id) DO UPDATE SET content_hash=excluded.content_hash,cloud_version=excluded.cloud_version", rusqlite::params![json,version]).map_err(db_error)?;
        Ok(())
    }

    fn workspace_is_empty(&self) -> AppResult<bool> {
        let count: i64 = self.connection.query_row("SELECT (SELECT count(*) FROM projects)+(SELECT count(*) FROM tasks)+(SELECT count(*) FROM rfis)+(SELECT count(*) FROM submittals)+(SELECT count(*) FROM registered_files)+(SELECT count(*) FROM notes)+(SELECT count(*) FROM contacts)", [], |row| row.get(0)).map_err(db_error)?;
        Ok(count == 0)
    }

    fn save_conflict(&self, local: &str, cloud: &str, cloud_version: i64) -> AppResult<()> {
        self.connection.execute("DELETE FROM sync_conflicts WHERE entity_type='workspace' AND entity_id='primary' AND resolved_at_utc IS NULL", []).map_err(db_error)?;
        self.connection.execute("INSERT INTO sync_conflicts(id,entity_type,entity_id,local_payload_json,cloud_payload_json,cloud_version,created_at_utc) VALUES(?1,'workspace','primary',?2,?3,?4,strftime('%Y-%m-%dT%H:%M:%fZ','now'))", rusqlite::params![Uuid::new_v4().to_string(),local,cloud,cloud_version]).map_err(db_error)?;
        Ok(())
    }

    pub fn list_sync_conflicts(&self) -> AppResult<Vec<SyncConflict>> {
        let mut statement = self.connection.prepare("SELECT id,created_at_utc FROM sync_conflicts WHERE resolved_at_utc IS NULL ORDER BY created_at_utc DESC").map_err(db_error)?;
        let conflicts = statement
            .query_map([], |row| {
                Ok(SyncConflict {
                    id: row.get(0)?,
                    created_at_utc: row.get(1)?,
                })
            })
            .map_err(db_error)?
            .collect::<Result<_, _>>()
            .map_err(db_error)?;
        Ok(conflicts)
    }
}

fn db_error(error: rusqlite::Error) -> AppError {
    sync_error(
        "SYNC_DATABASE_ERROR",
        "Sync state could not be read or saved.",
        error.to_string(),
    )
}

fn fetch_remote(client: &Client, token: &str) -> AppResult<Option<RemoteRecord>> {
    let response = client.get(format!("{PROJECT_URL}/rest/v1/sync_records?entity_type=eq.workspace&entity_id=eq.primary&select=version,payload"))
        .header("apikey", PUBLISHABLE_KEY).bearer_auth(token).send().map_err(|e| sync_error("SYNC_NETWORK_FAILED", "The cloud workspace could not be reached.", e.to_string()))?;
    if !response.status().is_success() {
        let status = response.status().as_u16();
        return Err(sync_error(
            "SYNC_REMOTE_FAILED",
            "Cloud metadata could not be read.",
            format!("Legacy Sync read failed with HTTP {status}."),
        ));
    }
    let mut records: Vec<RemoteRecord> = response.json().map_err(|e| {
        sync_error(
            "SYNC_REMOTE_INVALID",
            "Cloud metadata returned an invalid response.",
            e.to_string(),
        )
    })?;
    Ok(records.pop())
}

fn push_remote(
    client: &Client,
    token: &str,
    expected_version: i64,
    snapshot: &WorkspaceSnapshot,
) -> AppResult<PushResponse> {
    let response = client.post(format!("{PROJECT_URL}/rest/v1/rpc/sync_upsert_record"))
        .header("apikey", PUBLISHABLE_KEY).bearer_auth(token)
        .json(&serde_json::json!({"p_entity_type":"workspace","p_entity_id":"primary","p_expected_version":expected_version,"p_payload":snapshot,"p_is_deleted":false}))
        .send().map_err(|e| sync_error("SYNC_NETWORK_FAILED", "The cloud workspace could not be reached.", e.to_string()))?;
    if !response.status().is_success() {
        let status = response.status().as_u16();
        return Err(sync_error(
            "SYNC_REMOTE_FAILED",
            "Cloud metadata could not be saved.",
            format!("Legacy Sync write failed with HTTP {status}."),
        ));
    }
    response.json().map_err(|e| {
        sync_error(
            "SYNC_REMOTE_INVALID",
            "Cloud metadata returned an invalid response.",
            e.to_string(),
        )
    })
}

pub fn sync_now(database: &mut Database) -> AppResult<SyncResult> {
    let token = cloud_auth::refreshed_access_token()?;
    let client = Client::new();
    let local = database.export_workspace_snapshot()?;
    let local_json = serde_json::to_string(&local).map_err(|e| {
        sync_error(
            "SYNC_SERIALIZE_FAILED",
            "Local metadata could not be prepared.",
            e.to_string(),
        )
    })?;
    let baseline = database.baseline()?;
    let remote = fetch_remote(&client, &token)?;
    match remote {
        None => {
            let pushed = push_remote(&client, &token, 0, &local)?;
            let version = pushed.version.unwrap_or(1);
            database.save_baseline(&local_json, version)?;
            Ok(SyncResult {
                outcome: "uploaded".into(),
                message: "Local metadata uploaded to the cloud workspace.".into(),
                cloud_version: version,
                conflict_count: 0,
            })
        }
        Some(remote) => {
            let remote_json = serde_json::to_string(&remote.payload).map_err(|e| {
                sync_error(
                    "SYNC_SERIALIZE_FAILED",
                    "Cloud metadata could not be prepared.",
                    e.to_string(),
                )
            })?;
            let (base_json, base_version) = baseline.unwrap_or_default();
            if base_version == 0 {
                if database.workspace_is_empty()? {
                    database.import_workspace_snapshot(&remote.payload)?;
                    database.save_baseline(&remote_json, remote.version)?;
                    return Ok(SyncResult {
                        outcome: "downloaded".into(),
                        message: "Cloud metadata downloaded to this computer.".into(),
                        cloud_version: remote.version,
                        conflict_count: 0,
                    });
                }
                if local_json == remote_json {
                    database.save_baseline(&local_json, remote.version)?;
                    return Ok(SyncResult {
                        outcome: "current".into(),
                        message: "Workspace is already synchronized.".into(),
                        cloud_version: remote.version,
                        conflict_count: 0,
                    });
                }
            } else {
                let local_changed = local_json != base_json;
                let remote_changed = remote.version != base_version;
                if !local_changed && !remote_changed {
                    return Ok(SyncResult {
                        outcome: "current".into(),
                        message: "Workspace is already synchronized.".into(),
                        cloud_version: remote.version,
                        conflict_count: 0,
                    });
                }
                if !local_changed && remote_changed {
                    database.import_workspace_snapshot(&remote.payload)?;
                    database.save_baseline(&remote_json, remote.version)?;
                    return Ok(SyncResult {
                        outcome: "downloaded".into(),
                        message: "Cloud changes downloaded to this computer.".into(),
                        cloud_version: remote.version,
                        conflict_count: 0,
                    });
                }
                if local_changed && !remote_changed {
                    let pushed = push_remote(&client, &token, remote.version, &local)?;
                    if pushed.outcome == "applied" {
                        let version = pushed.version.unwrap_or(remote.version + 1);
                        database.save_baseline(&local_json, version)?;
                        return Ok(SyncResult {
                            outcome: "uploaded".into(),
                            message: "Local changes uploaded to the cloud workspace.".into(),
                            cloud_version: version,
                            conflict_count: 0,
                        });
                    }
                    if let Some(record) = pushed.record {
                        let cloud_json = serde_json::to_string(&record.payload).map_err(|e| {
                            sync_error(
                                "SYNC_SERIALIZE_FAILED",
                                "Cloud metadata could not be prepared.",
                                e.to_string(),
                            )
                        })?;
                        database.save_conflict(&local_json, &cloud_json, record.version)?;
                        return Ok(SyncResult { outcome:"conflict".into(), message:"Both computers changed the workspace. Choose which version to keep.".into(), cloud_version:record.version, conflict_count:1 });
                    }
                }
            }
            database.save_conflict(&local_json, &remote_json, remote.version)?;
            Ok(SyncResult {
                outcome: "conflict".into(),
                message: "Both computers changed the workspace. Choose which version to keep."
                    .into(),
                cloud_version: remote.version,
                conflict_count: 1,
            })
        }
    }
}

pub fn resolve_conflict(database: &mut Database, id: &str, choice: &str) -> AppResult<SyncResult> {
    let (local_json, cloud_json, cloud_version): (String,String,i64) = database.connection.query_row("SELECT local_payload_json,cloud_payload_json,cloud_version FROM sync_conflicts WHERE id=?1 AND resolved_at_utc IS NULL", [id], |row| Ok((row.get(0)?,row.get(1)?,row.get(2)?))).map_err(db_error)?;
    let token = cloud_auth::refreshed_access_token()?;
    let client = Client::new();
    let (snapshot, version, outcome) = if choice == "cloud" {
        let snapshot: WorkspaceSnapshot = serde_json::from_str(&cloud_json).map_err(|e| {
            sync_error(
                "SYNC_PAYLOAD_INVALID",
                "Cloud metadata could not be applied safely.",
                e.to_string(),
            )
        })?;
        database.import_workspace_snapshot(&snapshot)?;
        (snapshot, cloud_version, "downloaded")
    } else if choice == "local" {
        let snapshot: WorkspaceSnapshot = serde_json::from_str(&local_json).map_err(|e| {
            sync_error(
                "SYNC_PAYLOAD_INVALID",
                "Local metadata could not be uploaded safely.",
                e.to_string(),
            )
        })?;
        let pushed = push_remote(&client, &token, cloud_version, &snapshot)?;
        if pushed.outcome != "applied" {
            return Err(sync_error(
                "SYNC_CONFLICT_CHANGED",
                "The cloud workspace changed again.",
                "Resolve the new conflict after syncing again.",
            ));
        }
        let version = pushed.version.unwrap_or(cloud_version + 1);
        (snapshot, version, "uploaded")
    } else {
        return Err(sync_error(
            "SYNC_CHOICE_INVALID",
            "Choose either this computer or the cloud version.",
            choice,
        ));
    };
    let json = serde_json::to_string(&snapshot).map_err(|e| {
        sync_error(
            "SYNC_SERIALIZE_FAILED",
            "Metadata could not be prepared.",
            e.to_string(),
        )
    })?;
    database.save_baseline(&json, version)?;
    database.connection.execute("UPDATE sync_conflicts SET resolved_at_utc=strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE id=?1",[id]).map_err(db_error)?;
    Ok(SyncResult {
        outcome: outcome.into(),
        message: "Conflict resolved. The workspace is synchronized.".into(),
        cloud_version: version,
        conflict_count: 0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_round_trips_and_rebases_project_paths() {
        let base = std::env::temp_dir().join(format!("anydesk-sync-{}", Uuid::new_v4()));
        let first_root = base.join("first");
        let second_root = base.join("second");
        std::fs::create_dir_all(&first_root).unwrap();
        std::fs::create_dir_all(&second_root).unwrap();
        let mut first = Database::open(&base.join("first.sqlite3")).unwrap();
        first
            .save_project_root(&first_root.to_string_lossy())
            .unwrap();
        let first_project_path = first_root.join("P-100 - Test Project");
        first.connection.execute("INSERT INTO projects(id,number,name,project_path,created_at_utc,updated_at_utc) VALUES('p1','P-100','Test Project',?1,'2026-09-22T00:00:00Z','2026-09-22T00:00:00Z')", [first_project_path.to_string_lossy().to_string()]).unwrap();
        first.connection.execute("INSERT INTO tasks(id,project_id,title,priority,status,created_at_utc,updated_at_utc) VALUES('t1','p1','Review controls','high','open','2026-09-22T00:00:00Z','2026-09-22T00:00:00Z')", []).unwrap();

        let snapshot = first.export_workspace_snapshot().unwrap();
        assert_eq!(
            snapshot.tables["projects"][0]["project_path"],
            "@root/P-100 - Test Project"
        );

        let mut second = Database::open(&base.join("second.sqlite3")).unwrap();
        second
            .save_project_root(&second_root.to_string_lossy())
            .unwrap();
        second.import_workspace_snapshot(&snapshot).unwrap();
        let imported_path: String = second
            .connection
            .query_row(
                "SELECT project_path FROM projects WHERE id='p1'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(
            Path::new(&imported_path),
            second_root.join("P-100 - Test Project")
        );
        assert_eq!(second.list_tasks().unwrap().len(), 1);
        drop(first);
        drop(second);
        std::fs::remove_dir_all(base).unwrap();
    }

    #[test]
    fn portable_paths_reject_windows_escape_and_device_forms() {
        let root = Path::new(r"C:\Projects");
        for malicious in [
            "@root/../escape",
            "@root/folder/../../escape",
            r"@root/..\escape",
            r"@root/folder\..\escape",
            "@root/C:/escape",
            "@root/C:escape",
            "@root//server/share",
            r"@root/\\server\share",
            r"@root/\\?\C:\escape",
            r"@root/\\.\C:\escape",
            "@root/file:stream",
            "@root/CON.txt",
            "@root/folder/NUL",
            "@root/COM¹.txt",
            "@root/LPT³.log",
            "@external/folder/file.txt",
            "@external/../file.txt",
            r"@external/..\file.txt",
        ] {
            let error = local_path(malicious, root).unwrap_err();
            assert_eq!(error.code, "SYNC_PAYLOAD_INVALID", "{malicious}");
        }
    }

    #[test]
    fn portable_paths_preserve_safe_nested_and_external_names() {
        let root = Path::new(r"\\server\share\Projects");
        assert_eq!(
            Path::new(&local_path("@root/100 – Café/02 Submittals/file.pdf", root).unwrap()),
            root.join("100 – Café")
                .join("02 Submittals")
                .join("file.pdf")
        );
        assert_eq!(
            Path::new(&local_path("@external/field photo 01.jpg", root).unwrap()),
            root.join(".anydesk-missing").join("field photo 01.jpg")
        );
    }

    #[test]
    fn invalid_snapshot_path_does_not_replace_existing_workspace() {
        let base = std::env::temp_dir().join(format!("anydesk-sync-invalid-{}", Uuid::new_v4()));
        let root = base.join("projects");
        std::fs::create_dir_all(&root).unwrap();
        let mut database = Database::open(&base.join("workspace.sqlite3")).unwrap();
        database.save_project_root(&root.to_string_lossy()).unwrap();
        database.connection.execute(
            "INSERT INTO projects(id,number,name,project_path,created_at_utc,updated_at_utc) VALUES('existing','E-1','Existing',?1,'2026-09-22T00:00:00Z','2026-09-22T00:00:00Z')",
            [root.join("E-1 - Existing").to_string_lossy().to_string()],
        ).unwrap();

        let clean_snapshot = database.export_workspace_snapshot().unwrap();
        for (table, field) in PATH_FIELDS {
            let mut snapshot = clean_snapshot.clone();
            let row = if *table == "projects" {
                &mut snapshot.tables.get_mut(*table).unwrap()[0]
            } else {
                snapshot.tables.get_mut(*table).unwrap().push(Map::new());
                snapshot.tables.get_mut(*table).unwrap().last_mut().unwrap()
            };
            row.insert((*field).into(), Value::String("@root/../escape".into()));
            let error = database.import_workspace_snapshot(&snapshot).unwrap_err();
            assert_eq!(error.code, "SYNC_PAYLOAD_INVALID", "{table}.{field}");
        }
        let count: i64 = database
            .connection
            .query_row(
                "SELECT count(*) FROM projects WHERE id='existing'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
        drop(database);
        std::fs::remove_dir_all(base).unwrap();
    }

    #[test]
    fn incomplete_snapshot_is_rejected_without_replacing_existing_workspace() {
        let base = std::env::temp_dir().join(format!("anydesk-sync-incomplete-{}", Uuid::new_v4()));
        let root = base.join("projects");
        std::fs::create_dir_all(&root).unwrap();
        let mut database = Database::open(&base.join("workspace.sqlite3")).unwrap();
        database.save_project_root(&root.to_string_lossy()).unwrap();
        database.connection.execute(
            "INSERT INTO projects(id,number,name,project_path,created_at_utc,updated_at_utc) VALUES('existing','E-1','Existing',?1,'2026-09-22T00:00:00Z','2026-09-22T00:00:00Z')",
            [root.join("E-1 - Existing").to_string_lossy().to_string()],
        ).unwrap();

        let mut snapshot = database.export_workspace_snapshot().unwrap();
        snapshot.tables.remove("tasks");
        let error = database.import_workspace_snapshot(&snapshot).unwrap_err();
        assert_eq!(error.code, "SYNC_SCHEMA_UNSUPPORTED");
        let count: i64 = database
            .connection
            .query_row(
                "SELECT count(*) FROM projects WHERE id='existing'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);

        drop(database);
        std::fs::remove_dir_all(base).unwrap();
    }

    #[test]
    fn blob_value_is_rejected_instead_of_silently_exported_as_null() {
        let base = std::env::temp_dir().join(format!("anydesk-sync-blob-{}", Uuid::new_v4()));
        let root = base.join("projects");
        std::fs::create_dir_all(&root).unwrap();
        let mut database = Database::open(&base.join("workspace.sqlite3")).unwrap();
        database.save_project_root(&root.to_string_lossy()).unwrap();
        database.connection.execute(
            "INSERT INTO projects(id,number,name,project_path,created_at_utc,updated_at_utc) VALUES('blob','B-1',?1,?2,'2026-09-22T00:00:00Z','2026-09-22T00:00:00Z')",
            rusqlite::params![vec![0_u8, 1_u8, 2_u8], root.join("B-1").to_string_lossy().to_string()],
        ).unwrap();

        let error = database.export_workspace_snapshot().unwrap_err();
        assert_eq!(error.code, "SYNC_PAYLOAD_UNSUPPORTED");

        drop(database);
        std::fs::remove_dir_all(base).unwrap();
    }
}

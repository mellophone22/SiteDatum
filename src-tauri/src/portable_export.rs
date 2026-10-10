use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

use rusqlite::types::ValueRef;
use serde::Serialize;
use uuid::Uuid;

use crate::{
    data_exchange::safe_csv_text,
    error::{AppError, AppResult},
    persistence::Database,
};

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PortableExportFile {
    pub entity: String,
    pub file_name: String,
    pub row_count: usize,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PortableExportResult {
    pub path: String,
    pub exported_at_utc: u64,
    pub files: Vec<PortableExportFile>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ExportManifest {
    format: &'static str,
    format_version: u8,
    exported_at_utc: u64,
    database_schema_version: i64,
    includes_document_bytes: bool,
    files: Vec<PortableExportFile>,
}

struct TableSpec {
    entity: &'static str,
    file_name: &'static str,
    headers: &'static [&'static str],
    sql: &'static str,
}

const TABLES: &[TableSpec] = &[
    TableSpec { entity: "projects", file_name: "projects.csv", headers: &["id","number","name","status","phase","custom_phase_name","customer","general_contractor","engineer","project_manager","superintendent","location","start_date","target_date","description","important_notes","project_path","is_pinned","archived_at_utc","created_at_utc","updated_at_utc"], sql: "SELECT id,number,name,status,phase,custom_phase_name,customer,general_contractor,engineer,project_manager,superintendent,location,start_date,target_date,description,important_notes,project_path,is_pinned,archived_at_utc,created_at_utc,updated_at_utc FROM projects ORDER BY created_at_utc,id" },
    TableSpec { entity: "tasks", file_name: "tasks.csv", headers: &["id","project_id","title","description","priority","status","category","due_date","follow_up_date","waiting_since_utc","waiting_on","related_contact_id","created_at_utc","updated_at_utc"], sql: "SELECT id,project_id,title,description,priority,status,category,due_date,follow_up_date,waiting_since_utc,waiting_on,related_contact_id,created_at_utc,updated_at_utc FROM tasks ORDER BY created_at_utc,id" },
    TableSpec { entity: "rfis", file_name: "rfis.csv", headers: &["id","project_id","number","subject","question","recipient","status","created_date","submitted_date","response_due_date","response_received_date","response","notes","created_at_utc","updated_at_utc","rfi_location","drawing_number","cost_impact","time_delay","suggested_solution","requested_by"], sql: "SELECT id,project_id,number,subject,question,recipient,status,created_date,submitted_date,response_due_date,response_received_date,response,notes,created_at_utc,updated_at_utc,rfi_location,drawing_number,cost_impact,time_delay,suggested_solution,requested_by FROM rfis ORDER BY created_at_utc,id" },
    TableSpec { entity: "rfi_task_relationships", file_name: "rfi_task_relationships.csv", headers: &["rfi_id","task_id","created_at_utc"], sql: "SELECT rfi_id,task_id,created_at_utc FROM rfi_task_relationships ORDER BY created_at_utc,rfi_id,task_id" },
    TableSpec { entity: "rfi_attachment_references", file_name: "rfi_attachment_references.csv", headers: &["id","rfi_id","file_path","display_name","created_at_utc"], sql: "SELECT id,rfi_id,file_path,display_name,created_at_utc FROM rfi_attachment_references ORDER BY created_at_utc,id" },
    TableSpec { entity: "submittals", file_name: "submittals.csv", headers: &["id","project_id","parent_submittal_id","number","name","package","revision","recipient","status","disposition","created_date","submitted_date","response_date","resubmission_required","notes","created_at_utc","updated_at_utc"], sql: "SELECT id,project_id,parent_submittal_id,number,name,package,revision,recipient,status,disposition,created_date,submitted_date,response_date,resubmission_required,notes,created_at_utc,updated_at_utc FROM submittals ORDER BY created_at_utc,id" },
    TableSpec { entity: "submittal_task_relationships", file_name: "submittal_task_relationships.csv", headers: &["submittal_id","task_id","created_at_utc"], sql: "SELECT submittal_id,task_id,created_at_utc FROM submittal_task_relationships ORDER BY created_at_utc,submittal_id,task_id" },
    TableSpec { entity: "submittal_attachment_references", file_name: "submittal_attachment_references.csv", headers: &["id","submittal_id","file_path","display_name","created_at_utc"], sql: "SELECT id,submittal_id,file_path,display_name,created_at_utc FROM submittal_attachment_references ORDER BY created_at_utc,id" },
    TableSpec { entity: "registered_files", file_name: "registered_files.csv", headers: &["id","project_id","file_name","file_path","operation","discipline","drawing_number","title","revision","revision_date","received_date","is_current","created_at_utc","updated_at_utc"], sql: "SELECT id,project_id,file_name,file_path,operation,discipline,drawing_number,title,revision,revision_date,received_date,is_current,created_at_utc,updated_at_utc FROM registered_files ORDER BY created_at_utc,id" },
    TableSpec { entity: "notes", file_name: "notes.csv", headers: &["id","project_id","body","created_at_utc","updated_at_utc"], sql: "SELECT id,project_id,body,created_at_utc,updated_at_utc FROM notes ORDER BY created_at_utc,id" },
    TableSpec { entity: "contacts", file_name: "contacts.csv", headers: &["id","name","company","email","phone","role","created_at_utc","updated_at_utc"], sql: "SELECT id,name,company,email,phone,role,created_at_utc,updated_at_utc FROM contacts ORDER BY created_at_utc,id" },
    TableSpec { entity: "work_items", file_name: "work_items.csv", headers: &["id","project_id","item_type","number","title","description","status","priority","due_date","occurred_date","responsible_party","company","location","amount_cents","checklist_total","checklist_completed","archived_at_utc","created_at_utc","updated_at_utc"], sql: "SELECT id,project_id,item_type,number,title,description,status,priority,due_date,occurred_date,responsible_party,company,location,amount_cents,checklist_total,checklist_completed,archived_at_utc,created_at_utc,updated_at_utc FROM work_items ORDER BY created_at_utc,id" },
    TableSpec { entity: "project_templates", file_name: "project_templates.csv", headers: &["id","name","task_titles_json","milestone_titles_json","created_at_utc","updated_at_utc"], sql: "SELECT id,name,task_titles_json,milestone_titles_json,created_at_utc,updated_at_utc FROM project_templates ORDER BY created_at_utc,id" },
    TableSpec { entity: "activity_events", file_name: "activity_events.csv", headers: &["id","event_type","entity_type","entity_id","summary","occurred_at_utc"], sql: "SELECT id,event_type,entity_type,entity_id,summary,occurred_at_utc FROM activity_events ORDER BY occurred_at_utc,id" },
];

fn export_error(message: &str, recovery: &str, detail: impl Into<String>) -> AppError {
    AppError::from_technical("DATA_EXPORT_FAILED", message, recovery, detail)
}

fn unix_seconds() -> AppResult<u64> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_secs())
        .map_err(|error| {
            export_error(
                "The export time could not be determined.",
                "Check the Windows clock and try again.",
                error.to_string(),
            )
        })
}

fn write_table(database: &Database, directory: &Path, spec: &TableSpec) -> AppResult<usize> {
    let path = directory.join(spec.file_name);
    let file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .map_err(|error| {
            export_error(
                "A CSV file could not be created.",
                "Choose a writable folder and try again.",
                format!("{}: {error}", path.display()),
            )
        })?;
    let mut writer = csv::WriterBuilder::new()
        .terminator(csv::Terminator::CRLF)
        .from_writer(file);
    writer.write_record(spec.headers).map_err(|error| {
        export_error(
            "A CSV header could not be written.",
            "Try the export again.",
            error.to_string(),
        )
    })?;
    let mut statement = database.connection.prepare(spec.sql).map_err(|error| {
        export_error(
            "The local data could not be prepared for export.",
            "Try the export again. Your records were not changed.",
            error.to_string(),
        )
    })?;
    let column_count = statement.column_count();
    let mut rows = statement.query([]).map_err(|error| {
        export_error(
            "The local data could not be read for export.",
            "Try the export again. Your records were not changed.",
            error.to_string(),
        )
    })?;
    let mut count = 0;
    while let Some(row) = rows.next().map_err(|error| {
        export_error(
            "The local data could not be read for export.",
            "Try the export again. Your records were not changed.",
            error.to_string(),
        )
    })? {
        let mut record = Vec::with_capacity(column_count);
        for index in 0..column_count {
            let value = row.get_ref(index).map_err(|error| {
                export_error(
                    "A local value could not be read for export.",
                    "Try the export again. Your records were not changed.",
                    error.to_string(),
                )
            })?;
            record.push(match value {
                ValueRef::Null => String::new(),
                ValueRef::Integer(value) => value.to_string(),
                ValueRef::Real(value) => value.to_string(),
                ValueRef::Text(value) => {
                    let text = String::from_utf8_lossy(value);
                    safe_csv_text(&text).into_owned()
                }
                ValueRef::Blob(_) => {
                    return Err(export_error(
                        "An unsupported local value was found.",
                        "Contact SiteDatum support with the error reference.",
                        format!("BLOB in {} column {index}", spec.entity),
                    ))
                }
            });
        }
        writer.write_record(record).map_err(|error| {
            export_error(
                "A CSV row could not be written.",
                "Try the export again.",
                error.to_string(),
            )
        })?;
        count += 1;
    }
    writer.flush().map_err(|error| {
        export_error(
            "A CSV file could not be completed.",
            "Check the destination and try again.",
            error.to_string(),
        )
    })?;
    Ok(count)
}

pub fn export(database: &Database, parent: &Path) -> AppResult<PortableExportResult> {
    let parent = fs::canonicalize(parent).map_err(|error| {
        export_error(
            "The selected export folder is unavailable.",
            "Choose an existing writable folder and try again.",
            error.to_string(),
        )
    })?;
    if !parent.is_dir() {
        return Err(export_error(
            "The selected export destination is not a folder.",
            "Choose an existing folder and try again.",
            parent.display().to_string(),
        ));
    }
    let exported_at_utc = unix_seconds()?;
    let identity = Uuid::new_v4().simple().to_string();
    let final_path = parent.join(format!(
        "SiteDatum-export-{exported_at_utc}-{}",
        &identity[..8]
    ));
    let partial_path = parent.join(format!(".SiteDatum-export-{identity}.partial"));
    if final_path.exists() || partial_path.exists() {
        return Err(export_error(
            "The export destination already exists.",
            "Choose another folder or try again.",
            final_path.display().to_string(),
        ));
    }
    fs::create_dir(&partial_path).map_err(|error| {
        export_error(
            "The export folder could not be created.",
            "Choose a writable folder and try again.",
            error.to_string(),
        )
    })?;

    let operation = (|| -> AppResult<Vec<PortableExportFile>> {
        let mut files = Vec::with_capacity(TABLES.len());
        for spec in TABLES {
            files.push(PortableExportFile {
                entity: spec.entity.to_owned(),
                file_name: spec.file_name.to_owned(),
                row_count: write_table(database, &partial_path, spec)?,
            });
        }
        let database_schema_version = database
            .connection
            .query_row(
                "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
                [],
                |row| row.get::<_, i64>(0),
            )
            .map_err(|error| {
                export_error(
                    "The database version could not be recorded.",
                    "Try the export again.",
                    error.to_string(),
                )
            })?;
        let manifest = ExportManifest {
            format: "SiteDatum portable CSV directory",
            format_version: 1,
            exported_at_utc,
            database_schema_version,
            includes_document_bytes: false,
            files: files.clone(),
        };
        let bytes = serde_json::to_vec_pretty(&manifest).map_err(|error| {
            export_error(
                "The export manifest could not be created.",
                "Try the export again.",
                error.to_string(),
            )
        })?;
        let manifest_path = partial_path.join("manifest.json");
        let mut manifest_file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&manifest_path)
            .map_err(|error| {
                export_error(
                    "The export manifest could not be created.",
                    "Choose a writable folder and try again.",
                    error.to_string(),
                )
            })?;
        manifest_file
            .write_all(&bytes)
            .and_then(|_| manifest_file.sync_all())
            .map_err(|error| {
                export_error(
                    "The export manifest could not be completed.",
                    "Check the destination and try again.",
                    error.to_string(),
                )
            })?;
        Ok(files)
    })();

    let files = match operation {
        Ok(files) => files,
        Err(error) => {
            if let Err(cleanup_error) = fs::remove_dir_all(&partial_path) {
                eprintln!(
                    "Portable export cleanup failed for {}: {cleanup_error}",
                    partial_path.display()
                );
            }
            return Err(error);
        }
    };
    fs::rename(&partial_path, &final_path).map_err(|error| {
        let _ = fs::remove_dir_all(&partial_path);
        export_error(
            "The completed export could not be finalized.",
            "Check the destination folder and try again.",
            error.to_string(),
        )
    })?;
    Ok(PortableExportResult {
        path: final_path.to_string_lossy().into_owned(),
        exported_at_utc,
        files,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exports_every_core_table_and_protects_csv_text() {
        let database = Database::open_in_memory().unwrap();
        database.connection.execute("INSERT INTO projects(id,number,name,status,phase,project_path,is_pinned,archived_at_utc,created_at_utc,updated_at_utc) VALUES('p1','P-001','=SUM(1,1)','active','construction','C:\\Projects\\P-001',0,'2026-01-02T00:00:00Z','2026-01-01T00:00:00Z','2026-01-02T00:00:00Z')", []).unwrap();
        database.connection.execute("INSERT INTO project_templates(id,name,task_titles_json,milestone_titles_json,created_at_utc,updated_at_utc) VALUES('template-1','Closeout','[]','[]','2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')", []).unwrap();
        let parent =
            std::env::temp_dir().join(format!("sitedatum-portable-export-test-{}", Uuid::new_v4()));
        fs::create_dir(&parent).unwrap();
        let result = export(&database, &parent).unwrap();
        assert_eq!(result.files.len(), TABLES.len());
        assert!(result
            .files
            .iter()
            .all(|file| Path::new(&result.path).join(&file.file_name).is_file()));
        assert!(Path::new(&result.path).join("manifest.json").is_file());
        assert!(!fs::read_dir(&parent).unwrap().any(|entry| entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .ends_with(".partial")));

        let mut reader =
            csv::Reader::from_path(Path::new(&result.path).join("projects.csv")).unwrap();
        let row = reader.records().next().unwrap().unwrap();
        assert_eq!(&row[2], "'=SUM(1,1)");
        assert_eq!(&row[17], "0");
        assert_eq!(&row[18], "2026-01-02T00:00:00Z");
        let manifest: serde_json::Value = serde_json::from_slice(
            &fs::read(Path::new(&result.path).join("manifest.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(manifest["formatVersion"], 1);
        assert_eq!(manifest["databaseSchemaVersion"], 11);
        assert_eq!(manifest["includesDocumentBytes"], false);
        assert_eq!(manifest["files"].as_array().unwrap().len(), TABLES.len());
        assert_eq!(
            result
                .files
                .iter()
                .find(|file| file.entity == "projects")
                .unwrap()
                .row_count,
            1
        );
        assert_eq!(
            result
                .files
                .iter()
                .find(|file| file.entity == "project_templates")
                .unwrap()
                .row_count,
            1
        );

        fs::remove_dir_all(parent).unwrap();
    }
}

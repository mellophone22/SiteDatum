use crate::{
    error::{AppError, AppResult},
    persistence::Database,
};
use rusqlite::{backup::Backup, Connection, OpenFlags};
use serde::Serialize;
use std::{
    fs,
    path::{Path, PathBuf},
    time::UNIX_EPOCH,
};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupInfo {
    pub path: String,
    pub file_name: String,
    pub size_bytes: u64,
    pub modified_at_utc: String,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupPreview {
    pub projects: i64,
    pub tasks: i64,
    pub rfis: i64,
    pub submittals: i64,
    pub work_items: i64,
    pub notes: i64,
    pub contacts: i64,
}
fn err(code: &'static str, message: &str, detail: impl Into<String>) -> AppError {
    AppError::from_technical(
        code,
        message,
        "Choose a valid SiteDatum backup and try again.",
        detail,
    )
}
pub fn list(backups: &Path) -> AppResult<Vec<BackupInfo>> {
    if !backups.exists() {
        return Ok(Vec::new());
    }
    let mut values = fs::read_dir(backups)
        .map_err(|e| {
            err(
                "BACKUP_LIST_FAILED",
                "Backups could not be listed.",
                e.to_string(),
            )
        })?
        .filter_map(Result::ok)
        .filter(|entry| entry.path().extension().and_then(|v| v.to_str()) == Some("sqlite3"))
        .filter_map(|entry| {
            let metadata = entry.metadata().ok()?;
            let modified = metadata
                .modified()
                .ok()?
                .duration_since(UNIX_EPOCH)
                .ok()?
                .as_secs();
            Some(BackupInfo {
                path: entry.path().to_string_lossy().into_owned(),
                file_name: entry.file_name().to_string_lossy().into_owned(),
                size_bytes: metadata.len(),
                modified_at_utc: format!("{modified}"),
            })
        })
        .collect::<Vec<_>>();
    values.sort_by(|a, b| b.modified_at_utc.cmp(&a.modified_at_utc));
    Ok(values)
}
fn count(connection: &Connection, table: &str) -> i64 {
    connection
        .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get(0))
        .unwrap_or(0)
}
pub fn preview(path: &Path) -> AppResult<BackupPreview> {
    let connection =
        Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY).map_err(|e| {
            err(
                "BACKUP_OPEN_FAILED",
                "The backup could not be opened.",
                e.to_string(),
            )
        })?;
    connection
        .query_row("PRAGMA integrity_check", [], |r| r.get::<_, String>(0))
        .map_err(|e| {
            err(
                "BACKUP_INVALID",
                "The backup failed its integrity check.",
                e.to_string(),
            )
        })
        .and_then(|v| {
            if v == "ok" {
                Ok(())
            } else {
                Err(err(
                    "BACKUP_INVALID",
                    "The backup failed its integrity check.",
                    v,
                ))
            }
        })?;
    Ok(BackupPreview {
        projects: count(&connection, "projects"),
        tasks: count(&connection, "tasks"),
        rfis: count(&connection, "rfis"),
        submittals: count(&connection, "submittals"),
        work_items: count(&connection, "work_items"),
        notes: count(&connection, "notes"),
        contacts: count(&connection, "contacts"),
    })
}
pub fn validate_target(path: &str, backups: &Path) -> AppResult<PathBuf> {
    let canonical = Path::new(path).canonicalize().map_err(|e| {
        err(
            "BACKUP_NOT_FOUND",
            "The selected backup is unavailable.",
            e.to_string(),
        )
    })?;
    let root = backups.canonicalize().map_err(|e| {
        err(
            "BACKUP_DIRECTORY_FAILED",
            "Backup storage is unavailable.",
            e.to_string(),
        )
    })?;
    if canonical.parent() != Some(root.as_path())
        || canonical.extension().and_then(|v| v.to_str()) != Some("sqlite3")
    {
        return Err(err(
            "BACKUP_PATH_INVALID",
            "Only a SiteDatum app-local backup can be restored.",
            canonical.display().to_string(),
        ));
    }
    Ok(canonical)
}
pub fn restore(database: &mut Database, source: &Path) -> AppResult<()> {
    preview(source)?;
    database.checkpoint()?;
    let source =
        Connection::open_with_flags(source, OpenFlags::SQLITE_OPEN_READ_ONLY).map_err(|e| {
            err(
                "BACKUP_OPEN_FAILED",
                "The backup could not be opened.",
                e.to_string(),
            )
        })?;
    {
        let backup = Backup::new(&source, &mut database.connection).map_err(|e| {
            err(
                "RESTORE_START_FAILED",
                "Restore could not start.",
                e.to_string(),
            )
        })?;
        backup
            .run_to_completion(64, std::time::Duration::from_millis(10), None)
            .map_err(|e| {
                err(
                    "RESTORE_FAILED",
                    "The backup could not be restored.",
                    e.to_string(),
                )
            })?;
    }
    crate::persistence::apply_migrations(&mut database.connection)?;
    database
        .connection
        .execute_batch("PRAGMA foreign_keys = ON;")
        .map_err(|e| {
            err(
                "RESTORE_CONFIGURATION_FAILED",
                "The restored database could not be configured.",
                e.to_string(),
            )
        })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn restore_replaces_data_and_preserves_database_guards() {
        let root = std::env::temp_dir().join(format!("anydesk-restore-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();
        let source_path = root.join("source.sqlite3");
        let target_path = root.join("target.sqlite3");

        let source = Database::open(&source_path).unwrap();
        source.connection.execute("INSERT INTO projects(id,number,name,status,phase,project_path,created_at_utc,updated_at_utc) VALUES('source','P-100','Source project','active','engineering','C:\\Source',strftime('%Y-%m-%dT%H:%M:%fZ','now'),strftime('%Y-%m-%dT%H:%M:%fZ','now'))", []).unwrap();
        source.checkpoint().unwrap();
        drop(source);

        let mut target = Database::open(&target_path).unwrap();
        target.connection.execute("INSERT INTO projects(id,number,name,status,phase,project_path,created_at_utc,updated_at_utc) VALUES('target','P-200','Target project','active','engineering','C:\\Target',strftime('%Y-%m-%dT%H:%M:%fZ','now'),strftime('%Y-%m-%dT%H:%M:%fZ','now'))", []).unwrap();
        restore(&mut target, &source_path).unwrap();

        let project_name: String = target
            .connection
            .query_row("SELECT name FROM projects", [], |row| row.get(0))
            .unwrap();
        let foreign_keys: i64 = target
            .connection
            .query_row("PRAGMA foreign_keys", [], |row| row.get(0))
            .unwrap();
        let latest_migration: i64 = target
            .connection
            .query_row(
                "SELECT COUNT(*) FROM schema_migrations WHERE version = 10",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(project_name, "Source project");
        assert_eq!(foreign_keys, 1);
        assert_eq!(latest_migration, 1);

        drop(target);
        std::fs::remove_dir_all(root).unwrap();
    }
}

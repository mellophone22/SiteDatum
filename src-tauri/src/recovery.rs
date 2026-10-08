use crate::{
    error::{AppError, AppResult},
    persistence::Database,
};
use rusqlite::{backup::Backup, Connection, OpenFlags};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

const BACKUP_SETTINGS_KEY: &str = "backup_settings_v1";

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupInfo {
    pub path: String,
    pub file_name: String,
    pub size_bytes: u64,
    pub modified_at_utc: String,
}
#[derive(Clone, Serialize)]
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

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StartupRecoveryResult {
    pub restored_path: String,
    pub preserved_path: Option<String>,
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupSettings {
    pub schedule: String,
    pub destination_path: Option<String>,
    pub last_success_utc: Option<u64>,
}

impl Default for BackupSettings {
    fn default() -> Self {
        Self {
            schedule: "off".to_owned(),
            destination_path: None,
            last_success_utc: None,
        }
    }
}

impl BackupSettings {
    pub fn validate(mut self) -> AppResult<Self> {
        if !matches!(self.schedule.as_str(), "off" | "daily" | "weekly") {
            return Err(err(
                "BACKUP_SCHEDULE_INVALID",
                "The backup schedule is not supported.",
                &self.schedule,
            ));
        }
        self.destination_path = self
            .destination_path
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty());
        if self.schedule != "off" && self.destination_path.is_none() {
            return Err(err(
                "BACKUP_DESTINATION_REQUIRED",
                "Choose an external backup folder before enabling scheduled backups.",
                "No external destination was supplied.",
            ));
        }
        Ok(self)
    }
}

pub fn settings_key() -> &'static str {
    BACKUP_SETTINGS_KEY
}

pub fn decode_settings(value: Option<String>) -> AppResult<BackupSettings> {
    match value {
        Some(value) => serde_json::from_str::<BackupSettings>(&value)
            .map_err(|error| {
                err(
                    "BACKUP_SETTINGS_INVALID",
                    "Backup settings could not be read.",
                    error.to_string(),
                )
            })?
            .validate(),
        None => Ok(BackupSettings::default()),
    }
}

pub fn encode_settings(settings: BackupSettings) -> AppResult<(BackupSettings, String)> {
    let settings = settings.validate()?;
    let value = serde_json::to_string(&settings).map_err(|error| {
        err(
            "BACKUP_SETTINGS_SAVE_FAILED",
            "Backup settings could not be saved.",
            error.to_string(),
        )
    })?;
    Ok((settings, value))
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
    values
        .sort_by_key(|value| std::cmp::Reverse(value.modified_at_utc.parse::<u64>().unwrap_or(0)));
    Ok(values)
}

fn backup_info(path: &Path) -> AppResult<BackupInfo> {
    let metadata = fs::metadata(path).map_err(|error| {
        err(
            "BACKUP_METADATA_FAILED",
            "The backup was created, but its details could not be read.",
            error.to_string(),
        )
    })?;
    let modified = metadata
        .modified()
        .and_then(|value| {
            value
                .duration_since(UNIX_EPOCH)
                .map_err(std::io::Error::other)
        })
        .map_err(|error| {
            err(
                "BACKUP_METADATA_FAILED",
                "The backup was created, but its details could not be read.",
                error.to_string(),
            )
        })?
        .as_secs();
    Ok(BackupInfo {
        path: path.to_string_lossy().into_owned(),
        file_name: path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned(),
        size_bytes: metadata.len(),
        modified_at_utc: modified.to_string(),
    })
}

pub fn create(
    database: &Database,
    destination_directory: &Path,
    label: &str,
) -> AppResult<BackupInfo> {
    if !destination_directory.is_dir() {
        return Err(err(
            "BACKUP_DIRECTORY_FAILED",
            "The selected backup folder is unavailable.",
            destination_directory.display().to_string(),
        ));
    }
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| AppError::internal(error.to_string()))?
        .as_millis();
    let destination = destination_directory.join(format!("{label}-{stamp}.sqlite3"));
    let partial = destination.with_extension("sqlite3.partial");
    if destination.exists() || partial.exists() {
        return Err(err(
            "BACKUP_ALREADY_EXISTS",
            "A backup with the generated name already exists.",
            destination.display().to_string(),
        ));
    }
    {
        let mut target = Connection::open(&partial).map_err(|error| {
            err(
                "BACKUP_CREATE_FAILED",
                "The backup file could not be created.",
                error.to_string(),
            )
        })?;
        let backup = Backup::new(&database.connection, &mut target).map_err(|error| {
            err(
                "BACKUP_CREATE_FAILED",
                "The backup could not start.",
                error.to_string(),
            )
        })?;
        backup
            .run_to_completion(64, Duration::from_millis(10), None)
            .map_err(|error| {
                err(
                    "BACKUP_CREATE_FAILED",
                    "The backup could not be completed.",
                    error.to_string(),
                )
            })?;
    }
    if let Err(error) = preview(&partial) {
        let _ = fs::remove_file(&partial);
        return Err(error);
    }
    fs::rename(&partial, &destination).map_err(|error| {
        let _ = fs::remove_file(&partial);
        err(
            "BACKUP_FINALIZE_FAILED",
            "The backup was verified but could not be finalized.",
            error.to_string(),
        )
    })?;
    backup_info(&destination)
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
    for table in ["schema_migrations", "projects"] {
        let present = connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name=?1)",
                [table],
                |row| row.get::<_, bool>(0),
            )
            .map_err(|error| {
                err(
                    "BACKUP_INVALID",
                    "The file is not a SiteDatum backup.",
                    error.to_string(),
                )
            })?;
        if !present {
            return Err(err(
                "BACKUP_INVALID",
                "The file is not a SiteDatum backup.",
                format!("Missing required table: {table}"),
            ));
        }
    }
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
pub fn validate_file(path: &str) -> AppResult<PathBuf> {
    let canonical = Path::new(path).canonicalize().map_err(|error| {
        err(
            "BACKUP_NOT_FOUND",
            "The selected backup is unavailable.",
            error.to_string(),
        )
    })?;
    if !canonical.is_file()
        || canonical.extension().and_then(|value| value.to_str()) != Some("sqlite3")
    {
        return Err(err(
            "BACKUP_PATH_INVALID",
            "Choose a SiteDatum .sqlite3 backup file.",
            canonical.display().to_string(),
        ));
    }
    preview(&canonical)?;
    Ok(canonical)
}
pub fn validate_target(path: &str, backups: &Path) -> AppResult<PathBuf> {
    let canonical = validate_file(path)?;
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

pub fn schedule_due(settings: &BackupSettings, now_utc: u64) -> bool {
    let interval = match settings.schedule.as_str() {
        "daily" => 24 * 60 * 60,
        "weekly" => 7 * 24 * 60 * 60,
        _ => return false,
    };
    settings
        .last_success_utc
        .is_none_or(|last| now_utc.saturating_sub(last) >= interval)
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

pub fn replace_unavailable_database(
    source: &Path,
    target: &Path,
    quarantine_directory: &Path,
) -> AppResult<StartupRecoveryResult> {
    preview(source)?;
    let parent = target
        .parent()
        .ok_or_else(|| AppError::internal("Application storage is unavailable."))?;
    fs::create_dir_all(parent).map_err(|error| {
        err(
            "STARTUP_RECOVERY_FAILED",
            "Application storage could not be prepared for recovery.",
            error.to_string(),
        )
    })?;
    fs::create_dir_all(quarantine_directory).map_err(|error| {
        err(
            "STARTUP_RECOVERY_FAILED",
            "The recovery archive could not be prepared.",
            error.to_string(),
        )
    })?;
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| AppError::internal(error.to_string()))?
        .as_millis();
    let candidate = parent.join(format!("workspace-recovery-{stamp}.partial"));
    {
        let source_connection =
            Connection::open_with_flags(source, OpenFlags::SQLITE_OPEN_READ_ONLY).map_err(
                |error| {
                    err(
                        "BACKUP_OPEN_FAILED",
                        "The backup could not be opened.",
                        error.to_string(),
                    )
                },
            )?;
        let mut candidate_connection = Connection::open(&candidate).map_err(|error| {
            err(
                "STARTUP_RECOVERY_FAILED",
                "A recovered database could not be prepared.",
                error.to_string(),
            )
        })?;
        let backup =
            Backup::new(&source_connection, &mut candidate_connection).map_err(|error| {
                err(
                    "STARTUP_RECOVERY_FAILED",
                    "Recovery could not start.",
                    error.to_string(),
                )
            })?;
        backup
            .run_to_completion(64, Duration::from_millis(10), None)
            .map_err(|error| {
                err(
                    "STARTUP_RECOVERY_FAILED",
                    "The selected backup could not be recovered.",
                    error.to_string(),
                )
            })?;
    }
    {
        let recovered = Database::open(&candidate)?;
        recovered.checkpoint()?;
    }
    preview(&candidate)?;

    let sidecar = |path: &Path, suffix: &str| {
        let mut value = path.as_os_str().to_os_string();
        value.push(suffix);
        PathBuf::from(value)
    };
    let preserved = if target.exists() {
        let preserved = quarantine_directory.join(format!("unavailable-workspace-{stamp}.sqlite3"));
        fs::rename(target, &preserved).map_err(|error| {
            let _ = fs::remove_file(&candidate);
            err(
                "STARTUP_RECOVERY_FAILED",
                "The unavailable database could not be preserved before recovery.",
                error.to_string(),
            )
        })?;
        let mut moved_sidecars = Vec::new();
        for suffix in ["-wal", "-shm"] {
            let source_sidecar = sidecar(target, suffix);
            if !source_sidecar.exists() {
                continue;
            }
            let preserved_sidecar = sidecar(&preserved, suffix);
            if let Err(error) = fs::rename(&source_sidecar, &preserved_sidecar) {
                for (from, to) in moved_sidecars.iter().rev() {
                    let _ = fs::rename(to, from);
                }
                let _ = fs::rename(&preserved, target);
                let _ = fs::remove_file(&candidate);
                return Err(err(
                    "STARTUP_RECOVERY_FAILED",
                    "The unavailable database sidecar files could not be preserved.",
                    error.to_string(),
                ));
            }
            moved_sidecars.push((source_sidecar, preserved_sidecar));
        }
        Some((preserved, moved_sidecars))
    } else {
        None
    };
    if let Err(error) = fs::rename(&candidate, target) {
        if let Some((preserved, sidecars)) = preserved.as_ref() {
            for (original, archived) in sidecars.iter().rev() {
                let _ = fs::rename(archived, original);
            }
            let _ = fs::rename(preserved, target);
        }
        let _ = fs::remove_file(&candidate);
        return Err(err(
            "STARTUP_RECOVERY_FAILED",
            "The recovered database could not be activated.",
            error.to_string(),
        ));
    }
    Ok(StartupRecoveryResult {
        restored_path: target.to_string_lossy().into_owned(),
        preserved_path: preserved.map(|(path, _)| path.to_string_lossy().into_owned()),
    })
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

    #[test]
    fn created_backup_is_verified_and_does_not_reuse_a_name() {
        let root = std::env::temp_dir().join(format!("sitedatum-backup-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();
        let source_path = root.join("source.sqlite3");
        let source = Database::open(&source_path).unwrap();
        let backup = create(&source, &root, "workspace").unwrap();
        assert!(Path::new(&backup.path).is_file());
        assert_eq!(preview(Path::new(&backup.path)).unwrap().projects, 0);
        assert!(!root.join(format!("{}.partial", backup.file_name)).exists());
        drop(source);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn schedule_only_runs_when_enabled_and_due() {
        let mut settings = BackupSettings::default();
        assert!(!schedule_due(&settings, 1_000_000));
        settings.schedule = "daily".into();
        settings.destination_path = Some("C:\\Backups".into());
        assert!(schedule_due(&settings, 1_000_000));
        settings.last_success_utc = Some(1_000_000 - 60);
        assert!(!schedule_due(&settings, 1_000_000));
        settings.last_success_utc = Some(1_000_000 - 86_400);
        assert!(schedule_due(&settings, 1_000_000));
    }

    #[test]
    fn startup_recovery_preserves_the_unavailable_database() {
        let root =
            std::env::temp_dir().join(format!("sitedatum-startup-recovery-{}", Uuid::new_v4()));
        let quarantine = root.join("backups");
        std::fs::create_dir_all(&quarantine).unwrap();
        let source_path = root.join("source.sqlite3");
        let target_path = root.join("workspace.sqlite3");
        let source = Database::open(&source_path).unwrap();
        source.connection.execute("INSERT INTO projects(id,number,name,status,phase,project_path,created_at_utc,updated_at_utc) VALUES('source','P-100','Recovered project','active','engineering','C:\\Source',strftime('%Y-%m-%dT%H:%M:%fZ','now'),strftime('%Y-%m-%dT%H:%M:%fZ','now'))", []).unwrap();
        source.checkpoint().unwrap();
        drop(source);
        std::fs::write(&target_path, b"not a sqlite database").unwrap();
        let mut wal_path = target_path.as_os_str().to_os_string();
        wal_path.push("-wal");
        std::fs::write(PathBuf::from(wal_path), b"preserved sidecar").unwrap();
        let result = replace_unavailable_database(&source_path, &target_path, &quarantine).unwrap();
        let preserved_path = Path::new(result.preserved_path.as_deref().unwrap());
        assert!(preserved_path.is_file());
        let mut preserved_wal = preserved_path.as_os_str().to_os_string();
        preserved_wal.push("-wal");
        assert!(PathBuf::from(preserved_wal).is_file());
        let recovered = Database::open(&target_path).unwrap();
        let name: String = recovered
            .connection
            .query_row("SELECT name FROM projects", [], |row| row.get(0))
            .unwrap();
        assert_eq!(name, "Recovered project");
        drop(recovered);
        std::fs::remove_dir_all(root).unwrap();
    }
}

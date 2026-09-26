use std::path::Path;

use rusqlite::{Connection, OptionalExtension};

use crate::error::{AppError, AppResult};
use crate::file_record::{self, FileInput, FileMetadataInput, FileRecord};
use crate::note_contact::{self, ActivityEvent, Contact, ContactInput, Note, NoteInput};
use crate::project::{Project, ProjectInput};
use crate::project_root::ProjectRootSetting;
use crate::rfi::{self, AttachmentReference, Rfi, RfiInput};
use crate::submittal::{
    self, AttachmentReference as SubmittalAttachmentReference, Submittal, SubmittalInput,
};
use crate::task::{self, AttentionSection, Task, TaskInput};

pub struct Database {
    pub(crate) connection: Connection,
}

impl Database {
    pub fn checkpoint(&self) -> AppResult<()> {
        self.connection
            .execute_batch("PRAGMA wal_checkpoint(FULL);")
            .map_err(database_error)
    }
    pub fn open(path: &Path) -> AppResult<Self> {
        let mut connection = Connection::open(path).map_err(database_error)?;
        connection
            .execute_batch("PRAGMA foreign_keys = ON; PRAGMA journal_mode = WAL;")
            .map_err(database_error)?;
        apply_migrations(&mut connection)?;
        Ok(Self { connection })
    }
    pub fn get_project_root(&self) -> AppResult<ProjectRootSetting> {
        let path = self
            .connection
            .query_row(
                "SELECT value FROM app_settings WHERE key = 'project_root_path'",
                [],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(database_error)?;
        Ok(ProjectRootSetting { path })
    }
    pub fn save_project_root(&mut self, path: &str) -> AppResult<ProjectRootSetting> {
        let transaction = self.connection.transaction().map_err(database_error)?;
        transaction.execute("INSERT INTO app_settings (key, value, updated_at_utc) VALUES ('project_root_path', ?1, strftime('%Y-%m-%dT%H:%M:%fZ','now')) ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at_utc = excluded.updated_at_utc", [path]).map_err(database_error)?;
        transaction.execute("INSERT INTO activity_events (event_type, entity_type, entity_id, summary, occurred_at_utc) VALUES ('project_root.saved', 'setting', 'project_root_path', 'Project root updated', strftime('%Y-%m-%dT%H:%M:%fZ','now'))", []).map_err(database_error)?;
        transaction.commit().map_err(database_error)?;
        Ok(ProjectRootSetting {
            path: Some(path.to_owned()),
        })
    }
    pub fn list_projects(&self, include_archived: bool) -> AppResult<Vec<Project>> {
        let mut statement = self.connection.prepare("SELECT id, number, name, status, phase, custom_phase_name, customer, general_contractor, engineer, project_manager, superintendent, location, start_date, target_date, description, important_notes, project_path, is_pinned, archived_at_utc, created_at_utc, updated_at_utc FROM projects WHERE (?1 = 1 OR archived_at_utc IS NULL) ORDER BY is_pinned DESC, updated_at_utc DESC").map_err(database_error)?;
        let rows = statement
            .query_map([if include_archived { 1 } else { 0 }], project_from_row)
            .map_err(database_error)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(database_error)
    }
    pub fn get_project(&self, id: &str) -> AppResult<Project> {
        self.connection.query_row("SELECT id, number, name, status, phase, custom_phase_name, customer, general_contractor, engineer, project_manager, superintendent, location, start_date, target_date, description, important_notes, project_path, is_pinned, archived_at_utc, created_at_utc, updated_at_utc FROM projects WHERE id = ?1", [id], project_from_row).map_err(database_error)
    }
    pub fn insert_project(
        &mut self,
        id: &str,
        input: &ProjectInput,
        path: &str,
    ) -> AppResult<Project> {
        let tx = self.connection.transaction().map_err(database_error)?;
        tx.execute("INSERT INTO projects (id, number, name, status, phase, custom_phase_name, customer, general_contractor, engineer, project_manager, superintendent, location, start_date, target_date, description, important_notes, project_path, created_at_utc, updated_at_utc) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,strftime('%Y-%m-%dT%H:%M:%fZ','now'),strftime('%Y-%m-%dT%H:%M:%fZ','now'))", rusqlite::params![id,input.number,input.name,input.status,input.phase,input.custom_phase_name,input.customer,input.general_contractor,input.engineer,input.project_manager,input.superintendent,input.location,input.start_date,input.target_date,input.description,input.important_notes,path]).map_err(database_error)?;
        tx.execute("INSERT INTO activity_events (event_type, entity_type, entity_id, summary, occurred_at_utc) VALUES ('project.created','project',?1,?2,strftime('%Y-%m-%dT%H:%M:%fZ','now'))", rusqlite::params![id, format!("Created project {}", input.number)]).map_err(database_error)?;
        tx.commit().map_err(database_error)?;
        self.get_project(id)
    }
    pub fn set_project_flag(
        &mut self,
        id: &str,
        archived: Option<bool>,
        pinned: Option<bool>,
    ) -> AppResult<Project> {
        let tx = self.connection.transaction().map_err(database_error)?;
        if let Some(value) = pinned {
            tx.execute("UPDATE projects SET is_pinned = ?2, updated_at_utc = strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE id = ?1", rusqlite::params![id, value as i64]).map_err(database_error)?;
        }
        if let Some(value) = archived {
            tx.execute("UPDATE projects SET archived_at_utc = CASE WHEN ?2 = 1 THEN strftime('%Y-%m-%dT%H:%M:%fZ','now') ELSE NULL END, updated_at_utc = strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE id = ?1", rusqlite::params![id, value as i64]).map_err(database_error)?;
        }
        tx.commit().map_err(database_error)?;
        self.get_project(id)
    }
    pub fn update_project(&mut self, id: &str, input: &ProjectInput) -> AppResult<Project> {
        let tx = self.connection.transaction().map_err(database_error)?;
        let changed = tx.execute("UPDATE projects SET number=?2,name=?3,status=?4,phase=?5,custom_phase_name=?6,customer=?7,general_contractor=?8,engineer=?9,project_manager=?10,superintendent=?11,location=?12,start_date=?13,target_date=?14,description=?15,important_notes=?16,updated_at_utc=strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE id=?1", rusqlite::params![id,input.number,input.name,input.status,input.phase,input.custom_phase_name,input.customer,input.general_contractor,input.engineer,input.project_manager,input.superintendent,input.location,input.start_date,input.target_date,input.description,input.important_notes]).map_err(database_error)?;
        if changed == 0 {
            return Err(AppError::from_technical(
                "PROJECT_NOT_FOUND",
                "The project no longer exists.",
                "Return to Projects and refresh the list.",
                id,
            ));
        }
        tx.execute("INSERT INTO activity_events (event_type, entity_type, entity_id, summary, occurred_at_utc) VALUES ('project.updated','project',?1,?2,strftime('%Y-%m-%dT%H:%M:%fZ','now'))", rusqlite::params![id, format!("Updated project {}", input.number)]).map_err(database_error)?;
        tx.commit().map_err(database_error)?;
        self.get_project(id)
    }
    pub fn list_tasks(&self) -> AppResult<Vec<Task>> {
        let mut s=self.connection.prepare("SELECT t.id,t.project_id,p.number,p.name,t.title,t.description,t.priority,t.status,t.category,t.due_date,t.follow_up_date,t.waiting_on FROM tasks t JOIN projects p ON p.id=t.project_id WHERE p.archived_at_utc IS NULL ORDER BY t.due_date IS NULL,t.due_date,t.updated_at_utc DESC").map_err(database_error)?;
        let rows = s.query_map([], task_from_row).map_err(database_error)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(database_error)
    }
    pub fn list_attention(&self, today: &str, through: &str) -> AppResult<Vec<AttentionSection>> {
        task::validate_date(Some(today))?;
        task::validate_date(Some(through))?;
        if through < today {
            return Err(AppError::from_technical(
                "TASK_INVALID_ATTENTION_RANGE",
                "The attention date range is invalid.",
                "Refresh the view and try again.",
                format!("{today} > {through}"),
            ));
        }
        Ok(task::attention_sections(self.list_tasks()?, today, through))
    }
    pub fn create_task(&mut self, id: &str, input: &TaskInput) -> AppResult<Task> {
        task::validate(input)?;
        let tx = self.connection.transaction().map_err(database_error)?;
        tx.execute("INSERT INTO tasks (id,project_id,title,description,priority,status,category,due_date,follow_up_date,waiting_since_utc,waiting_on,created_at_utc,updated_at_utc) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,CASE WHEN ?6='waiting' THEN strftime('%Y-%m-%dT%H:%M:%fZ','now') END,?10,strftime('%Y-%m-%dT%H:%M:%fZ','now'),strftime('%Y-%m-%dT%H:%M:%fZ','now'))",rusqlite::params![id,input.project_id,input.title.trim(),input.description,input.priority,input.status,input.category,input.due_date,input.follow_up_date,input.waiting_on.as_deref().map(str::trim)]).map_err(database_error)?;
        tx.execute("INSERT INTO activity_events (event_type,entity_type,entity_id,summary,occurred_at_utc) VALUES ('task.created','task',?1,?2,strftime('%Y-%m-%dT%H:%M:%fZ','now'))",rusqlite::params![id,format!("Created task {}",input.title.trim())]).map_err(database_error)?;
        tx.commit().map_err(database_error)?;
        self.list_tasks()?
            .into_iter()
            .find(|t| t.id == id)
            .ok_or_else(|| AppError::internal("Created task could not be read."))
    }
    pub fn update_task(&mut self, id: &str, input: &TaskInput) -> AppResult<Task> {
        task::validate(input)?;
        let tx = self.connection.transaction().map_err(database_error)?;
        let incompatible_relationships:i64=tx.query_row("SELECT (SELECT COUNT(*) FROM rfi_task_relationships rel JOIN rfis r ON r.id=rel.rfi_id WHERE rel.task_id=?1 AND r.project_id<>?2)+(SELECT COUNT(*) FROM submittal_task_relationships rel JOIN submittals s ON s.id=rel.submittal_id WHERE rel.task_id=?1 AND s.project_id<>?2)",rusqlite::params![id,input.project_id],|row|row.get(0)).map_err(database_error)?;
        if incompatible_relationships > 0 {
            return Err(AppError::from_technical(
                "TASK_PROJECT_RELATIONSHIP_CONFLICT",
                "This task is linked to an RFI or submittal in its current project.",
                "Remove or update the related records before changing the task project.",
                id,
            ));
        }
        let changed=tx.execute("UPDATE tasks SET project_id=?2,title=?3,description=?4,priority=?5,status=?6,category=?7,due_date=?8,follow_up_date=?9,waiting_on=?10,waiting_since_utc=CASE WHEN ?6='waiting' THEN COALESCE(waiting_since_utc,strftime('%Y-%m-%dT%H:%M:%fZ','now')) ELSE NULL END,updated_at_utc=strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE id=?1",rusqlite::params![id,input.project_id,input.title.trim(),input.description,input.priority,input.status,input.category,input.due_date,input.follow_up_date,input.waiting_on.as_deref().map(str::trim)]).map_err(database_error)?;
        if changed == 0 {
            return Err(AppError::from_technical(
                "TASK_NOT_FOUND",
                "The task no longer exists.",
                "Refresh the task list and try again.",
                id,
            ));
        }
        tx.execute("INSERT INTO activity_events (event_type,entity_type,entity_id,summary,occurred_at_utc) VALUES ('task.updated','task',?1,?2,strftime('%Y-%m-%dT%H:%M:%fZ','now'))",rusqlite::params![id,format!("Updated task {}",input.title.trim())]).map_err(database_error)?;
        tx.commit().map_err(database_error)?;
        self.list_tasks()?
            .into_iter()
            .find(|task| task.id == id)
            .ok_or_else(|| AppError::internal("Updated task could not be read."))
    }
    pub fn set_task_status(
        &mut self,
        id: &str,
        status: &str,
        waiting_on: Option<&str>,
        follow_up: Option<&str>,
    ) -> AppResult<()> {
        task::validate_status(status, waiting_on)?;
        task::validate_date(follow_up)?;
        let tx = self.connection.transaction().map_err(database_error)?;
        let changed=tx.execute("UPDATE tasks SET status=?2,waiting_on=?3,follow_up_date=?4,waiting_since_utc=CASE WHEN ?2='waiting' THEN COALESCE(waiting_since_utc,strftime('%Y-%m-%dT%H:%M:%fZ','now')) ELSE NULL END,updated_at_utc=strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE id=?1",rusqlite::params![id,status,waiting_on.map(str::trim),follow_up]).map_err(database_error)?;
        if changed == 0 {
            return Err(AppError::from_technical(
                "TASK_NOT_FOUND",
                "The task no longer exists.",
                "Refresh the task list and try again.",
                id,
            ));
        }
        tx.execute("INSERT INTO activity_events (event_type,entity_type,entity_id,summary,occurred_at_utc) VALUES ('task.status_changed','task',?1,?2,strftime('%Y-%m-%dT%H:%M:%fZ','now'))",rusqlite::params![id,format!("Task status changed to {status}")]).map_err(database_error)?;
        tx.commit().map_err(database_error)?;
        Ok(())
    }
    pub fn list_rfis(&self) -> AppResult<Vec<Rfi>> {
        let mut statement = self.connection.prepare("SELECT r.id,r.project_id,p.number,p.name,r.number,r.subject,r.question,r.recipient,r.status,r.created_date,r.submitted_date,r.response_due_date,r.response_received_date,r.response,r.notes,r.rfi_location,r.drawing_number,r.cost_impact,r.time_delay,r.suggested_solution,r.requested_by,(SELECT task_id FROM rfi_task_relationships WHERE rfi_id=r.id LIMIT 1) FROM rfis r JOIN projects p ON p.id=r.project_id WHERE p.archived_at_utc IS NULL ORDER BY r.response_due_date IS NULL,r.response_due_date,r.updated_at_utc DESC").map_err(database_error)?;
        let rows = statement
            .query_map([], rfi_from_row)
            .map_err(database_error)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(database_error)
    }
    pub fn get_rfi(&self, id: &str) -> AppResult<Rfi> {
        self.list_rfis()?
            .into_iter()
            .find(|rfi| rfi.id == id)
            .ok_or_else(|| {
                AppError::from_technical(
                    "RFI_NOT_FOUND",
                    "The RFI no longer exists.",
                    "Return to RFIs and refresh the list.",
                    id,
                )
            })
    }
    pub fn create_rfi(&mut self, id: &str, input: &RfiInput) -> AppResult<Rfi> {
        rfi::validate(input)?;
        let tx = self.connection.transaction().map_err(database_error)?;
        write_rfi(&tx, id, input, true)?;
        tx.execute("INSERT INTO activity_events (event_type,entity_type,entity_id,summary,occurred_at_utc) VALUES ('rfi.created','rfi',?1,?2,strftime('%Y-%m-%dT%H:%M:%fZ','now'))", rusqlite::params![id, format!("Created RFI {}", input.number.trim())]).map_err(database_error)?;
        tx.commit().map_err(database_error)?;
        self.get_rfi(id)
    }
    pub fn update_rfi(&mut self, id: &str, input: &RfiInput) -> AppResult<Rfi> {
        rfi::validate(input)?;
        let tx = self.connection.transaction().map_err(database_error)?;
        let changed = write_rfi(&tx, id, input, false)?;
        if changed == 0 {
            return Err(AppError::from_technical(
                "RFI_NOT_FOUND",
                "The RFI no longer exists.",
                "Return to RFIs and refresh the list.",
                id,
            ));
        }
        tx.execute("INSERT INTO activity_events (event_type,entity_type,entity_id,summary,occurred_at_utc) VALUES ('rfi.updated','rfi',?1,?2,strftime('%Y-%m-%dT%H:%M:%fZ','now'))", rusqlite::params![id, format!("Updated RFI {}", input.number.trim())]).map_err(database_error)?;
        tx.commit().map_err(database_error)?;
        self.get_rfi(id)
    }
    pub fn list_rfi_attention(&self, today: &str, through: &str) -> AppResult<Vec<Rfi>> {
        task::validate_date(Some(today))?;
        task::validate_date(Some(through))?;
        if through < today {
            return Err(AppError::from_technical(
                "RFI_INVALID_ATTENTION_RANGE",
                "The attention date range is invalid.",
                "Refresh the view and try again.",
                format!("{today} > {through}"),
            ));
        }
        Ok(self
            .list_rfis()?
            .into_iter()
            .filter(|rfi| {
                rfi.status == "open"
                    && rfi
                        .response_due_date
                        .as_deref()
                        .is_some_and(|date| date <= through)
            })
            .collect())
    }
    pub fn list_rfi_attachments(&self, rfi_id: &str) -> AppResult<Vec<AttachmentReference>> {
        let mut statement = self.connection.prepare("SELECT id,file_path,display_name FROM rfi_attachment_references WHERE rfi_id=?1 ORDER BY created_at_utc DESC").map_err(database_error)?;
        let rows = statement
            .query_map([rfi_id], |row| {
                Ok(AttachmentReference {
                    id: row.get(0)?,
                    file_path: row.get(1)?,
                    display_name: row.get(2)?,
                })
            })
            .map_err(database_error)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(database_error)
    }
    pub fn add_rfi_attachment(
        &mut self,
        rfi_id: &str,
        file_path: &str,
    ) -> AppResult<AttachmentReference> {
        self.get_rfi(rfi_id)?;
        let path = Path::new(file_path);
        let metadata = std::fs::metadata(path).map_err(|error| {
            AppError::from_technical(
                "RFI_ATTACHMENT_UNAVAILABLE",
                "The selected attachment is unavailable.",
                "Choose an existing readable file and try again.",
                error.to_string(),
            )
        })?;
        if !metadata.is_file() {
            return Err(AppError::from_technical(
                "RFI_ATTACHMENT_NOT_FILE",
                "Choose a file to attach.",
                "Choose an existing file and try again.",
                file_path,
            ));
        }
        let name = path
            .file_name()
            .and_then(|value| value.to_str())
            .ok_or_else(|| {
                AppError::from_technical(
                    "RFI_ATTACHMENT_INVALID_NAME",
                    "The attachment name is invalid.",
                    "Choose a file with a valid Windows name.",
                    file_path,
                )
            })?
            .to_owned();
        let id = rfi::new_id();
        let tx = self.connection.transaction().map_err(database_error)?;
        tx.execute("INSERT INTO rfi_attachment_references (id,rfi_id,file_path,display_name,created_at_utc) VALUES (?1,?2,?3,?4,strftime('%Y-%m-%dT%H:%M:%fZ','now'))", rusqlite::params![id,rfi_id,file_path,name]).map_err(database_error)?;
        tx.execute("INSERT INTO activity_events (event_type,entity_type,entity_id,summary,occurred_at_utc) VALUES ('rfi.attachment_referenced','rfi',?1,?2,strftime('%Y-%m-%dT%H:%M:%fZ','now'))", rusqlite::params![rfi_id, format!("Referenced attachment {name}")]).map_err(database_error)?;
        tx.commit().map_err(database_error)?;
        Ok(AttachmentReference {
            id,
            file_path: file_path.to_owned(),
            display_name: name,
        })
    }
    pub fn remove_rfi_attachment(&mut self, id: &str) -> AppResult<()> {
        let tx = self.connection.transaction().map_err(database_error)?;
        let rfi_id: Option<String> = tx
            .query_row(
                "SELECT rfi_id FROM rfi_attachment_references WHERE id=?1",
                [id],
                |row| row.get(0),
            )
            .optional()
            .map_err(database_error)?;
        let rfi_id = rfi_id.ok_or_else(|| {
            AppError::from_technical(
                "RFI_ATTACHMENT_NOT_FOUND",
                "The attachment reference no longer exists.",
                "Refresh the RFI and try again.",
                id,
            )
        })?;
        tx.execute("DELETE FROM rfi_attachment_references WHERE id=?1", [id])
            .map_err(database_error)?;
        tx.execute("INSERT INTO activity_events (event_type,entity_type,entity_id,summary,occurred_at_utc) VALUES ('rfi.attachment_reference_removed','rfi',?1,'Removed attachment reference',strftime('%Y-%m-%dT%H:%M:%fZ','now'))", [rfi_id]).map_err(database_error)?;
        tx.commit().map_err(database_error)?;
        Ok(())
    }
    pub fn show_rfi_attachment_in_explorer(&self, id: &str) -> AppResult<()> {
        let path: String = self
            .connection
            .query_row(
                "SELECT file_path FROM rfi_attachment_references WHERE id=?1",
                [id],
                |row| row.get(0),
            )
            .optional()
            .map_err(database_error)?
            .ok_or_else(|| {
                AppError::from_technical(
                    "RFI_ATTACHMENT_NOT_FOUND",
                    "The attachment reference no longer exists.",
                    "Refresh the RFI and try again.",
                    id,
                )
            })?;
        if !Path::new(&path).is_file() {
            return Err(AppError::from_technical(
                "RFI_ATTACHMENT_MISSING",
                "The referenced attachment is unavailable.",
                "Locate the file in Explorer, then add a new reference when it is available.",
                path,
            ));
        }
        std::process::Command::new("explorer.exe")
            .arg("/select,")
            .arg(&path)
            .spawn()
            .map_err(|error| {
                AppError::from_technical(
                    "RFI_ATTACHMENT_SHOW_FAILED",
                    "The attachment could not be shown in Explorer.",
                    "Open the file path in Explorer instead.",
                    error.to_string(),
                )
            })?;
        Ok(())
    }
    pub fn list_submittals(&self) -> AppResult<Vec<Submittal>> {
        let mut s = self.connection.prepare("SELECT s.id,s.project_id,p.number,p.name,s.parent_submittal_id,s.number,s.name,s.package,s.revision,s.recipient,s.status,s.disposition,s.created_date,s.submitted_date,s.response_date,s.resubmission_required,s.notes,(SELECT task_id FROM submittal_task_relationships WHERE submittal_id=s.id LIMIT 1) FROM submittals s JOIN projects p ON p.id=s.project_id WHERE p.archived_at_utc IS NULL ORDER BY s.submitted_date IS NULL,s.submitted_date DESC,s.updated_at_utc DESC").map_err(database_error)?;
        let rows = s
            .query_map([], submittal_from_row)
            .map_err(database_error)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(database_error)
    }
    pub fn get_submittal(&self, id: &str) -> AppResult<Submittal> {
        self.list_submittals()?
            .into_iter()
            .find(|v| v.id == id)
            .ok_or_else(|| {
                AppError::from_technical(
                    "SUBMITTAL_NOT_FOUND",
                    "The submittal no longer exists.",
                    "Return to Submittals and refresh the list.",
                    id,
                )
            })
    }
    pub fn create_submittal(&mut self, id: &str, input: &SubmittalInput) -> AppResult<Submittal> {
        submittal::validate(input)?;
        let tx = self.connection.transaction().map_err(database_error)?;
        write_submittal(&tx, id, input, true)?;
        tx.execute("INSERT INTO activity_events (event_type,entity_type,entity_id,summary,occurred_at_utc) VALUES ('submittal.created','submittal',?1,?2,strftime('%Y-%m-%dT%H:%M:%fZ','now'))",rusqlite::params![id,format!("Created submittal {}",input.number.trim())]).map_err(database_error)?;
        tx.commit().map_err(database_error)?;
        self.get_submittal(id)
    }
    pub fn update_submittal(&mut self, id: &str, input: &SubmittalInput) -> AppResult<Submittal> {
        submittal::validate(input)?;
        let tx = self.connection.transaction().map_err(database_error)?;
        let changed = write_submittal(&tx, id, input, false)?;
        if changed == 0 {
            return Err(AppError::from_technical(
                "SUBMITTAL_NOT_FOUND",
                "The submittal no longer exists.",
                "Return to Submittals and refresh the list.",
                id,
            ));
        }
        tx.execute("INSERT INTO activity_events (event_type,entity_type,entity_id,summary,occurred_at_utc) VALUES ('submittal.updated','submittal',?1,?2,strftime('%Y-%m-%dT%H:%M:%fZ','now'))",rusqlite::params![id,format!("Updated submittal {}",input.number.trim())]).map_err(database_error)?;
        tx.commit().map_err(database_error)?;
        self.get_submittal(id)
    }
    pub fn list_submittal_attention(&self) -> AppResult<Vec<Submittal>> {
        Ok(self
            .list_submittals()?
            .into_iter()
            .filter(|v| ["submitted", "under_review"].contains(&v.status.as_str()))
            .collect())
    }
    pub fn list_submittal_attachments(
        &self,
        submittal_id: &str,
    ) -> AppResult<Vec<SubmittalAttachmentReference>> {
        let mut s=self.connection.prepare("SELECT id,file_path,display_name FROM submittal_attachment_references WHERE submittal_id=?1 ORDER BY created_at_utc DESC").map_err(database_error)?;
        let rows = s
            .query_map([submittal_id], |r| {
                Ok(SubmittalAttachmentReference {
                    id: r.get(0)?,
                    file_path: r.get(1)?,
                    display_name: r.get(2)?,
                })
            })
            .map_err(database_error)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(database_error)
    }
    pub fn add_submittal_attachment(
        &mut self,
        submittal_id: &str,
        file_path: &str,
    ) -> AppResult<SubmittalAttachmentReference> {
        self.get_submittal(submittal_id)?;
        let path = Path::new(file_path);
        let metadata = std::fs::metadata(path).map_err(|e| {
            AppError::from_technical(
                "SUBMITTAL_ATTACHMENT_UNAVAILABLE",
                "The selected attachment is unavailable.",
                "Choose an existing readable file and try again.",
                e.to_string(),
            )
        })?;
        if !metadata.is_file() {
            return Err(AppError::from_technical(
                "SUBMITTAL_ATTACHMENT_NOT_FILE",
                "Choose a file to attach.",
                "Choose an existing file and try again.",
                file_path,
            ));
        }
        let name = path
            .file_name()
            .and_then(|v| v.to_str())
            .ok_or_else(|| {
                AppError::from_technical(
                    "SUBMITTAL_ATTACHMENT_INVALID_NAME",
                    "The attachment name is invalid.",
                    "Choose a file with a valid Windows name.",
                    file_path,
                )
            })?
            .to_owned();
        let id = submittal::new_id();
        let tx = self.connection.transaction().map_err(database_error)?;
        tx.execute("INSERT INTO submittal_attachment_references (id,submittal_id,file_path,display_name,created_at_utc) VALUES (?1,?2,?3,?4,strftime('%Y-%m-%dT%H:%M:%fZ','now'))",rusqlite::params![id,submittal_id,file_path,name]).map_err(database_error)?;
        tx.commit().map_err(database_error)?;
        Ok(SubmittalAttachmentReference {
            id,
            file_path: file_path.to_owned(),
            display_name: name,
        })
    }
    pub fn remove_submittal_attachment(&mut self, id: &str) -> AppResult<()> {
        let changed = self
            .connection
            .execute(
                "DELETE FROM submittal_attachment_references WHERE id=?1",
                [id],
            )
            .map_err(database_error)?;
        if changed == 0 {
            return Err(AppError::from_technical(
                "SUBMITTAL_ATTACHMENT_NOT_FOUND",
                "The attachment reference no longer exists.",
                "Refresh the submittal and try again.",
                id,
            ));
        }
        Ok(())
    }
    pub fn list_files(&self) -> AppResult<Vec<FileRecord>> {
        let mut s=self.connection.prepare("SELECT f.id,f.project_id,p.number,p.name,f.file_name,f.file_path,f.operation,f.discipline,f.drawing_number,f.title,f.revision,f.revision_date,f.received_date,f.is_current FROM registered_files f JOIN projects p ON p.id=f.project_id WHERE p.archived_at_utc IS NULL ORDER BY f.updated_at_utc DESC").map_err(database_error)?;
        let rows = s
            .query_map([], |r| {
                let path: String = r.get(5)?;
                Ok(FileRecord {
                    id: r.get(0)?,
                    project_id: r.get(1)?,
                    project_number: r.get(2)?,
                    project_name: r.get(3)?,
                    file_name: r.get(4)?,
                    file_path: path.clone(),
                    operation: r.get(6)?,
                    discipline: r.get(7)?,
                    drawing_number: r.get(8)?,
                    title: r.get(9)?,
                    revision: r.get(10)?,
                    revision_date: r.get(11)?,
                    received_date: r.get(12)?,
                    is_current: r.get::<_, i64>(13)? != 0,
                    missing: !Path::new(&path).is_file(),
                })
            })
            .map_err(database_error)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(database_error)
    }
    pub fn create_file(&mut self, input: &FileInput) -> AppResult<FileRecord> {
        file_record::validate(input)?;
        let project_path: String = self
            .connection
            .query_row(
                "SELECT project_path FROM projects WHERE id=?1",
                [&input.project_id],
                |r| r.get(0),
            )
            .map_err(database_error)?;
        let final_path = file_record::transfer(input, &project_path)?;
        let name = Path::new(&final_path)
            .file_name()
            .and_then(|v| v.to_str())
            .ok_or_else(|| AppError::internal("File name unavailable."))?
            .to_owned();
        let id = file_record::id();
        let tx = self.connection.transaction().map_err(database_error)?;
        let write=tx.execute("INSERT INTO registered_files (id,project_id,file_name,file_path,operation,discipline,drawing_number,title,revision,revision_date,received_date,created_at_utc,updated_at_utc) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,strftime('%Y-%m-%dT%H:%M:%fZ','now'),strftime('%Y-%m-%dT%H:%M:%fZ','now'))",rusqlite::params![id,input.project_id,name,final_path,input.operation,file_record::clean(&input.discipline),file_record::clean(&input.drawing_number),file_record::clean(&input.title),file_record::clean(&input.revision),file_record::clean(&input.revision_date),file_record::clean(&input.received_date)]);
        if let Err(e) = write {
            return Err(AppError::from_technical("FILE_DATABASE_AFTER_OPERATION","The file operation completed, but its register record could not be saved.","Do not repeat the operation. Register the resulting file path after checking it in Explorer.",e.to_string()));
        }
        tx.commit().map_err(database_error)?;
        self.list_files()?
            .into_iter()
            .find(|f| f.id == id)
            .ok_or_else(|| AppError::internal("Registered file could not be read."))
    }
    pub fn remove_file_reference(&mut self, id: &str) -> AppResult<()> {
        let changed = self
            .connection
            .execute("DELETE FROM registered_files WHERE id=?1", [id])
            .map_err(database_error)?;
        if changed == 0 {
            return Err(AppError::from_technical(
                "FILE_REFERENCE_NOT_FOUND",
                "The file reference no longer exists.",
                "Refresh the file register and try again.",
                id,
            ));
        }
        Ok(())
    }
    pub fn locate_file(&mut self, id: &str, path: &str) -> AppResult<()> {
        if !Path::new(path).is_file() {
            return Err(AppError::from_technical(
                "FILE_LOCATE_UNAVAILABLE",
                "The selected replacement file is unavailable.",
                "Choose an existing readable file and try again.",
                path,
            ));
        }
        let changed=self.connection.execute("UPDATE registered_files SET file_path=?2,file_name=?3,updated_at_utc=strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE id=?1",rusqlite::params![id,path,Path::new(path).file_name().and_then(|v|v.to_str()).unwrap_or("file")]).map_err(database_error)?;
        if changed == 0 {
            return Err(AppError::from_technical(
                "FILE_REFERENCE_NOT_FOUND",
                "The file reference no longer exists.",
                "Refresh the file register and try again.",
                id,
            ));
        }
        Ok(())
    }
    pub fn update_file_metadata(
        &mut self,
        id: &str,
        input: &FileMetadataInput,
    ) -> AppResult<FileRecord> {
        let changed=self.connection.execute("UPDATE registered_files SET discipline=?2,drawing_number=?3,title=?4,revision=?5,revision_date=?6,received_date=?7,is_current=?8,updated_at_utc=strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE id=?1",rusqlite::params![id,file_record::clean(&input.discipline),file_record::clean(&input.drawing_number),file_record::clean(&input.title),file_record::clean(&input.revision),file_record::clean(&input.revision_date),file_record::clean(&input.received_date),input.is_current as i64]).map_err(database_error)?;
        if changed == 0 {
            return Err(AppError::from_technical(
                "FILE_REFERENCE_NOT_FOUND",
                "The file reference no longer exists.",
                "Refresh the file register and try again.",
                id,
            ));
        }
        self.list_files()?
            .into_iter()
            .find(|v| v.id == id)
            .ok_or_else(|| AppError::internal("Updated file could not be read."))
    }
    pub fn show_file_in_explorer(&self, id: &str) -> AppResult<()> {
        let path: String = self
            .connection
            .query_row(
                "SELECT file_path FROM registered_files WHERE id=?1",
                [id],
                |r| r.get(0),
            )
            .map_err(database_error)?;
        if !Path::new(&path).is_file() {
            return Err(AppError::from_technical(
                "FILE_MISSING",
                "The registered file is missing.",
                "Locate the file or remove its reference; metadata was retained.",
                path,
            ));
        }
        std::process::Command::new("explorer.exe")
            .arg("/select,")
            .arg(path)
            .spawn()
            .map_err(|e| {
                AppError::from_technical(
                    "FILE_EXPLORER_FAILED",
                    "The file could not be shown in Explorer.",
                    "Open the path manually.",
                    e.to_string(),
                )
            })?;
        Ok(())
    }
    pub fn list_notes(&self) -> AppResult<Vec<Note>> {
        let mut s=self.connection.prepare("SELECT n.id,n.project_id,p.number,p.name,n.body,n.created_at_utc FROM notes n LEFT JOIN projects p ON p.id=n.project_id ORDER BY n.updated_at_utc DESC").map_err(database_error)?;
        let rows = s
            .query_map([], |r| {
                Ok(Note {
                    id: r.get(0)?,
                    project_id: r.get(1)?,
                    project_number: r.get(2)?,
                    project_name: r.get(3)?,
                    body: r.get(4)?,
                    created_at_utc: r.get(5)?,
                })
            })
            .map_err(database_error)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(database_error)
    }
    pub fn create_note(&mut self, input: &NoteInput) -> AppResult<Note> {
        note_contact::note(input)?;
        let id = note_contact::id();
        let tx = self.connection.transaction().map_err(database_error)?;
        tx.execute("INSERT INTO notes (id,project_id,body,created_at_utc,updated_at_utc) VALUES (?1,?2,?3,strftime('%Y-%m-%dT%H:%M:%fZ','now'),strftime('%Y-%m-%dT%H:%M:%fZ','now'))",rusqlite::params![id,note_contact::clean(&input.project_id),input.body.trim()]).map_err(database_error)?;
        tx.execute("INSERT INTO activity_events (event_type,entity_type,entity_id,summary,occurred_at_utc) VALUES ('note.created','note',?1,'Created note',strftime('%Y-%m-%dT%H:%M:%fZ','now'))",[&id]).map_err(database_error)?;
        tx.commit().map_err(database_error)?;
        self.list_notes()?
            .into_iter()
            .find(|v| v.id == id)
            .ok_or_else(|| AppError::internal("Note could not be read."))
    }
    pub fn update_note(&mut self, id: &str, input: &NoteInput) -> AppResult<Note> {
        note_contact::note(input)?;
        let changed=self.connection.execute("UPDATE notes SET project_id=?2,body=?3,updated_at_utc=strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE id=?1",rusqlite::params![id,note_contact::clean(&input.project_id),input.body.trim()]).map_err(database_error)?;
        if changed == 0 {
            return Err(AppError::from_technical(
                "NOTE_NOT_FOUND",
                "The note no longer exists.",
                "Refresh notes and try again.",
                id,
            ));
        }
        self.list_notes()?
            .into_iter()
            .find(|v| v.id == id)
            .ok_or_else(|| AppError::internal("Updated note could not be read."))
    }
    pub fn delete_note(&mut self, id: &str) -> AppResult<()> {
        let changed = self
            .connection
            .execute("DELETE FROM notes WHERE id=?1", [id])
            .map_err(database_error)?;
        if changed == 0 {
            return Err(AppError::from_technical(
                "NOTE_NOT_FOUND",
                "The note no longer exists.",
                "Refresh notes and try again.",
                id,
            ));
        }
        Ok(())
    }
    pub fn list_contacts(&self) -> AppResult<Vec<Contact>> {
        let mut s=self.connection.prepare("SELECT id,name,company,email,phone,role FROM contacts ORDER BY name COLLATE NOCASE").map_err(database_error)?;
        let rows = s
            .query_map([], |r| {
                Ok(Contact {
                    id: r.get(0)?,
                    name: r.get(1)?,
                    company: r.get(2)?,
                    email: r.get(3)?,
                    phone: r.get(4)?,
                    role: r.get(5)?,
                })
            })
            .map_err(database_error)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(database_error)
    }
    pub fn create_contact(&mut self, input: &ContactInput) -> AppResult<Contact> {
        note_contact::contact(input)?;
        let id = note_contact::id();
        let tx = self.connection.transaction().map_err(database_error)?;
        tx.execute("INSERT INTO contacts (id,name,company,email,phone,role,created_at_utc,updated_at_utc) VALUES (?1,?2,?3,?4,?5,?6,strftime('%Y-%m-%dT%H:%M:%fZ','now'),strftime('%Y-%m-%dT%H:%M:%fZ','now'))",rusqlite::params![id,input.name.trim(),note_contact::clean(&input.company),note_contact::clean(&input.email),note_contact::clean(&input.phone),note_contact::clean(&input.role)]).map_err(database_error)?;
        tx.execute("INSERT INTO activity_events (event_type,entity_type,entity_id,summary,occurred_at_utc) VALUES ('contact.created','contact',?1,?2,strftime('%Y-%m-%dT%H:%M:%fZ','now'))",rusqlite::params![id,format!("Created contact {}",input.name.trim())]).map_err(database_error)?;
        tx.commit().map_err(database_error)?;
        self.list_contacts()?
            .into_iter()
            .find(|v| v.id == id)
            .ok_or_else(|| AppError::internal("Contact could not be read."))
    }
    pub fn update_contact(&mut self, id: &str, input: &ContactInput) -> AppResult<Contact> {
        note_contact::contact(input)?;
        let changed=self.connection.execute("UPDATE contacts SET name=?2,company=?3,email=?4,phone=?5,role=?6,updated_at_utc=strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE id=?1",rusqlite::params![id,input.name.trim(),note_contact::clean(&input.company),note_contact::clean(&input.email),note_contact::clean(&input.phone),note_contact::clean(&input.role)]).map_err(database_error)?;
        if changed == 0 {
            return Err(AppError::from_technical(
                "CONTACT_NOT_FOUND",
                "The contact no longer exists.",
                "Refresh contacts and try again.",
                id,
            ));
        }
        self.list_contacts()?
            .into_iter()
            .find(|v| v.id == id)
            .ok_or_else(|| AppError::internal("Updated contact could not be read."))
    }
    pub fn delete_contact(&mut self, id: &str) -> AppResult<()> {
        let changed = self
            .connection
            .execute("DELETE FROM contacts WHERE id=?1", [id])
            .map_err(database_error)?;
        if changed == 0 {
            return Err(AppError::from_technical(
                "CONTACT_NOT_FOUND",
                "The contact no longer exists.",
                "Refresh contacts and try again.",
                id,
            ));
        }
        Ok(())
    }
    pub fn list_activity(&self) -> AppResult<Vec<ActivityEvent>> {
        let mut s=self.connection.prepare("SELECT event_type,entity_type,summary,occurred_at_utc FROM activity_events ORDER BY occurred_at_utc DESC LIMIT 100").map_err(database_error)?;
        let rows = s
            .query_map([], |r| {
                Ok(ActivityEvent {
                    event_type: r.get(0)?,
                    entity_type: r.get(1)?,
                    summary: r.get(2)?,
                    occurred_at_utc: r.get(3)?,
                })
            })
            .map_err(database_error)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(database_error)
    }
}

pub(crate) fn apply_migrations(connection: &mut Connection) -> AppResult<()> {
    connection.execute_batch("CREATE TABLE IF NOT EXISTS schema_migrations (version INTEGER PRIMARY KEY, applied_at_utc TEXT NOT NULL);").map_err(database_error)?;
    for (version, sql) in [
        (1_i64, include_str!("../migrations/0001_foundation.sql")),
        (2_i64, include_str!("../migrations/0002_projects.sql")),
        (3_i64, include_str!("../migrations/0003_tasks.sql")),
        (4_i64, include_str!("../migrations/0004_rfis.sql")),
        (5_i64, include_str!("../migrations/0005_submittals.sql")),
        (6_i64, include_str!("../migrations/0006_files.sql")),
        (7_i64, include_str!("../migrations/0007_notes_contacts.sql")),
        (8_i64, include_str!("../migrations/0008_cloud_sync.sql")),
        (9_i64, include_str!("../migrations/0009_rfi_pdf_fields.sql")),
        (10_i64, include_str!("../migrations/0010_operations.sql")),
    ] {
        let applied = connection
            .query_row(
                "SELECT 1 FROM schema_migrations WHERE version = ?1",
                [version],
                |_| Ok(()),
            )
            .optional()
            .map_err(database_error)?;
        if applied.is_none() {
            let transaction = connection.transaction().map_err(database_error)?;
            transaction.execute_batch(sql).map_err(database_error)?;
            transaction.execute("INSERT INTO schema_migrations (version, applied_at_utc) VALUES (?1, strftime('%Y-%m-%dT%H:%M:%fZ','now'))", [version]).map_err(database_error)?;
            transaction.commit().map_err(database_error)?;
        }
    }
    Ok(())
}

fn project_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Project> {
    Ok(Project {
        id: row.get(0)?,
        number: row.get(1)?,
        name: row.get(2)?,
        status: row.get(3)?,
        phase: row.get(4)?,
        custom_phase_name: row.get(5)?,
        customer: row.get(6)?,
        general_contractor: row.get(7)?,
        engineer: row.get(8)?,
        project_manager: row.get(9)?,
        superintendent: row.get(10)?,
        location: row.get(11)?,
        start_date: row.get(12)?,
        target_date: row.get(13)?,
        description: row.get(14)?,
        important_notes: row.get(15)?,
        project_path: row.get(16)?,
        is_pinned: row.get::<_, i64>(17)? != 0,
        archived_at_utc: row.get(18)?,
        created_at_utc: row.get(19)?,
        updated_at_utc: row.get(20)?,
    })
}
fn task_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Task> {
    Ok(Task {
        id: row.get(0)?,
        project_id: row.get(1)?,
        project_number: row.get(2)?,
        project_name: row.get(3)?,
        title: row.get(4)?,
        description: row.get(5)?,
        priority: row.get(6)?,
        status: row.get(7)?,
        category: row.get(8)?,
        due_date: row.get(9)?,
        follow_up_date: row.get(10)?,
        waiting_on: row.get(11)?,
    })
}
fn rfi_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Rfi> {
    Ok(Rfi {
        id: row.get(0)?,
        project_id: row.get(1)?,
        project_number: row.get(2)?,
        project_name: row.get(3)?,
        number: row.get(4)?,
        subject: row.get(5)?,
        question: row.get(6)?,
        recipient: row.get(7)?,
        status: row.get(8)?,
        created_date: row.get(9)?,
        submitted_date: row.get(10)?,
        response_due_date: row.get(11)?,
        response_received_date: row.get(12)?,
        response: row.get(13)?,
        notes: row.get(14)?,
        rfi_location: row.get(15)?,
        drawing_number: row.get(16)?,
        cost_impact: row.get(17)?,
        time_delay: row.get(18)?,
        suggested_solution: row.get(19)?,
        requested_by: row.get(20)?,
        related_task_id: row.get(21)?,
    })
}

fn submittal_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Submittal> {
    Ok(Submittal {
        id: row.get(0)?,
        project_id: row.get(1)?,
        project_number: row.get(2)?,
        project_name: row.get(3)?,
        parent_submittal_id: row.get(4)?,
        number: row.get(5)?,
        name: row.get(6)?,
        package: row.get(7)?,
        revision: row
            .get::<_, String>(8)
            .ok()
            .filter(|value| !value.is_empty()),
        recipient: row.get(9)?,
        status: row.get(10)?,
        disposition: row.get(11)?,
        created_date: row.get(12)?,
        submitted_date: row.get(13)?,
        response_date: row.get(14)?,
        resubmission_required: row.get::<_, i64>(15)? != 0,
        notes: row.get(16)?,
        related_task_id: row.get(17)?,
    })
}

fn write_rfi(
    tx: &rusqlite::Transaction<'_>,
    id: &str,
    input: &RfiInput,
    creating: bool,
) -> AppResult<usize> {
    let recipient = rfi::clean(&input.recipient);
    let response = rfi::clean(&input.response);
    let notes = rfi::clean(&input.notes);
    let rfi_location = rfi::clean(&input.rfi_location);
    let drawing_number = rfi::clean(&input.drawing_number);
    let cost_impact = rfi::clean(&input.cost_impact);
    let time_delay = rfi::clean(&input.time_delay);
    let suggested_solution = rfi::clean(&input.suggested_solution);
    let requested_by = rfi::clean(&input.requested_by);
    let submitted_date = if input.status == "open" {
        input
            .submitted_date
            .clone()
            .or_else(|| Some(input.created_date.clone()))
    } else {
        input.submitted_date.clone()
    };
    let response_received_date = if ["response_received", "closed"].contains(&input.status.as_str())
    {
        input
            .response_received_date
            .clone()
            .or_else(|| Some(input.created_date.clone()))
    } else {
        input.response_received_date.clone()
    };
    let changed = if creating {
        tx.execute("INSERT INTO rfis (id,project_id,number,subject,question,recipient,status,created_date,submitted_date,response_due_date,response_received_date,response,notes,rfi_location,drawing_number,cost_impact,time_delay,suggested_solution,requested_by,created_at_utc,updated_at_utc) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,strftime('%Y-%m-%dT%H:%M:%fZ','now'),strftime('%Y-%m-%dT%H:%M:%fZ','now'))", rusqlite::params![id,input.project_id,input.number.trim(),input.subject.trim(),input.question.trim(),recipient,input.status,input.created_date,submitted_date,input.response_due_date,response_received_date,response,notes,rfi_location,drawing_number,cost_impact,time_delay,suggested_solution,requested_by]).map_err(database_error)?
    } else {
        tx.execute("UPDATE rfis SET project_id=?2,number=?3,subject=?4,question=?5,recipient=?6,status=?7,created_date=?8,submitted_date=?9,response_due_date=?10,response_received_date=?11,response=?12,notes=?13,rfi_location=?14,drawing_number=?15,cost_impact=?16,time_delay=?17,suggested_solution=?18,requested_by=?19,updated_at_utc=strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE id=?1", rusqlite::params![id,input.project_id,input.number.trim(),input.subject.trim(),input.question.trim(),recipient,input.status,input.created_date,submitted_date,input.response_due_date,response_received_date,response,notes,rfi_location,drawing_number,cost_impact,time_delay,suggested_solution,requested_by]).map_err(database_error)?
    };
    if !creating && changed == 0 {
        return Ok(0);
    }
    tx.execute("DELETE FROM rfi_task_relationships WHERE rfi_id=?1", [id])
        .map_err(database_error)?;
    if let Some(task_id) = rfi::clean(&input.related_task_id) {
        let matches_project: Option<i64> = tx
            .query_row(
                "SELECT 1 FROM tasks WHERE id=?1 AND project_id=?2",
                rusqlite::params![task_id, input.project_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(database_error)?;
        if matches_project.is_none() {
            return Err(AppError::from_technical(
                "RFI_RELATED_TASK_INVALID",
                "Choose a task from the same project.",
                "Select a task listed for this RFI's project.",
                task_id,
            ));
        }
        tx.execute("INSERT INTO rfi_task_relationships (rfi_id,task_id,created_at_utc) VALUES (?1,?2,strftime('%Y-%m-%dT%H:%M:%fZ','now'))", rusqlite::params![id,task_id]).map_err(database_error)?;
    }
    Ok(changed)
}

fn write_submittal(
    tx: &rusqlite::Transaction<'_>,
    id: &str,
    input: &SubmittalInput,
    creating: bool,
) -> AppResult<usize> {
    let parent = submittal::clean(&input.parent_submittal_id);
    let revision = submittal::clean(&input.revision).unwrap_or_default();
    if let Some(parent_id) = &parent {
        let matches: Option<i64> = tx
            .query_row(
                "SELECT 1 FROM submittals WHERE id=?1 AND project_id=?2",
                rusqlite::params![parent_id, input.project_id],
                |r| r.get(0),
            )
            .optional()
            .map_err(database_error)?;
        if matches.is_none() {
            return Err(AppError::from_technical(
                "SUBMITTAL_PARENT_INVALID",
                "Choose a prior submittal from the same project.",
                "Create the revision from an existing submittal or choose the same project.",
                parent_id,
            ));
        }
    }
    if input.status == "closed" {
        let disposition: Option<String> = if creating {
            None
        } else {
            tx.query_row(
                "SELECT disposition FROM submittals WHERE id=?1",
                [id],
                |r| r.get(0),
            )
            .optional()
            .map_err(database_error)?
            .flatten()
        };
        if disposition.is_none() {
            return Err(AppError::from_technical(
                "SUBMITTAL_DISPOSITION_REQUIRED",
                "Record an approval, revision request, or rejection before closing the submittal.",
                "Select the received disposition, save it, then close the submittal.",
                id,
            ));
        }
    }
    let disposition = submittal::disposition_for_status(&input.status);
    let changed = if creating {
        tx.execute("INSERT INTO submittals (id,project_id,parent_submittal_id,number,name,package,revision,recipient,status,disposition,created_date,submitted_date,response_date,resubmission_required,notes,created_at_utc,updated_at_utc) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,strftime('%Y-%m-%dT%H:%M:%fZ','now'),strftime('%Y-%m-%dT%H:%M:%fZ','now'))",rusqlite::params![id,input.project_id,parent,input.number.trim(),input.name.trim(),submittal::clean(&input.package),revision,submittal::clean(&input.recipient),input.status,disposition,input.created_date,input.submitted_date,input.response_date,input.resubmission_required as i64,submittal::clean(&input.notes)]).map_err(database_error)?
    } else {
        tx.execute("UPDATE submittals SET project_id=?2,parent_submittal_id=?3,number=?4,name=?5,package=?6,revision=?7,recipient=?8,status=?9,disposition=COALESCE(?10,disposition),created_date=?11,submitted_date=?12,response_date=?13,resubmission_required=?14,notes=?15,updated_at_utc=strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE id=?1",rusqlite::params![id,input.project_id,parent,input.number.trim(),input.name.trim(),submittal::clean(&input.package),revision,submittal::clean(&input.recipient),input.status,disposition,input.created_date,input.submitted_date,input.response_date,input.resubmission_required as i64,submittal::clean(&input.notes)]).map_err(database_error)?
    };
    if !creating && changed == 0 {
        return Ok(0);
    }
    tx.execute(
        "DELETE FROM submittal_task_relationships WHERE submittal_id=?1",
        [id],
    )
    .map_err(database_error)?;
    if let Some(task_id) = submittal::clean(&input.related_task_id) {
        let matches: Option<i64> = tx
            .query_row(
                "SELECT 1 FROM tasks WHERE id=?1 AND project_id=?2",
                rusqlite::params![task_id, input.project_id],
                |r| r.get(0),
            )
            .optional()
            .map_err(database_error)?;
        if matches.is_none() {
            return Err(AppError::from_technical(
                "SUBMITTAL_RELATED_TASK_INVALID",
                "Choose a task from the same project.",
                "Select a task listed for this submittal's project.",
                task_id,
            ));
        }
        tx.execute("INSERT INTO submittal_task_relationships (submittal_id,task_id,created_at_utc) VALUES (?1,?2,strftime('%Y-%m-%dT%H:%M:%fZ','now'))",rusqlite::params![id,task_id]).map_err(database_error)?;
    }
    Ok(changed)
}

fn database_error(error: rusqlite::Error) -> AppError {
    AppError::from_technical(
        "DATABASE_ERROR",
        "Local data could not be saved or read.",
        "Check available disk space and permissions, then restart the app.",
        error.to_string(),
    )
}

#[cfg(test)]
mod tests {
    use super::{Database, TaskInput};
    use crate::file_record::{FileInput, FileMetadataInput};
    use crate::note_contact::{ContactInput, NoteInput};
    use crate::rfi::RfiInput;
    use crate::submittal::SubmittalInput;

    fn test_task_input(project_id: &str) -> TaskInput {
        TaskInput {
            project_id: project_id.into(),
            title: "  Review coordination drawings  ".into(),
            priority: "high".into(),
            status: "open".into(),
            due_date: Some("2026-09-22".into()),
            follow_up_date: None,
            waiting_on: None,
            description: Some("Coordinate disciplines".into()),
            category: Some("Engineering".into()),
        }
    }

    fn insert_test_project(database: &Database) {
        database.connection.execute("INSERT INTO projects (id,number,name,project_path,created_at_utc,updated_at_utc) VALUES ('p1','P-100','Test Project','C:\\Projects\\P-100','2026-09-22T00:00:00Z','2026-09-22T00:00:00Z')", []).unwrap();
    }
    fn test_rfi_input(project_id: &str) -> RfiInput {
        RfiInput {
            project_id: project_id.into(),
            number: "RFI-012".into(),
            subject: "Clarify valve sequence".into(),
            question: "Which interlock is required?".into(),
            recipient: Some("Design Engineer".into()),
            status: "open".into(),
            created_date: "2026-09-22".into(),
            submitted_date: Some("2026-09-22".into()),
            response_due_date: Some("2026-09-25".into()),
            response_received_date: None,
            response: None,
            notes: None,
            rfi_location: Some("Mechanical room".into()),
            drawing_number: Some("M-401".into()),
            cost_impact: Some("None anticipated".into()),
            time_delay: Some("None anticipated".into()),
            suggested_solution: Some("Use sequence B.".into()),
            requested_by: Some("Project Engineer".into()),
            related_task_id: None,
        }
    }
    fn test_submittal_input(project_id: &str) -> SubmittalInput {
        SubmittalInput {
            project_id: project_id.into(),
            parent_submittal_id: None,
            number: "SUB-100".into(),
            name: "Control panel".into(),
            package: None,
            revision: None,
            recipient: Some("Architect".into()),
            status: "submitted".into(),
            created_date: "2026-09-22".into(),
            submitted_date: Some("2026-09-22".into()),
            response_date: None,
            resubmission_required: false,
            notes: None,
            related_task_id: None,
        }
    }

    #[test]
    fn persists_the_project_root_across_database_reopen() {
        let path = std::env::temp_dir().join(format!(
            "project-engineer-workspace-{}.sqlite3",
            uuid::Uuid::new_v4()
        ));
        {
            let mut database = Database::open(&path).unwrap();
            database.save_project_root("C:\\Projects").unwrap();
        }
        {
            let database = Database::open(&path).unwrap();
            assert_eq!(
                database.get_project_root().unwrap().path.as_deref(),
                Some("C:\\Projects")
            );
        }
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn task_lifecycle_is_persisted_and_activity_is_atomic() {
        let path = std::env::temp_dir().join(format!(
            "project-engineer-task-{}.sqlite3",
            uuid::Uuid::new_v4()
        ));
        let id = "task-1";
        {
            let mut database = Database::open(&path).unwrap();
            insert_test_project(&database);
            let created = database.create_task(id, &test_task_input("p1")).unwrap();
            assert_eq!(created.title, "Review coordination drawings");
            assert_eq!(created.status, "open");
            assert_eq!(
                created.description.as_deref(),
                Some("Coordinate disciplines")
            );
            assert_eq!(created.category.as_deref(), Some("Engineering"));
            let mut updated = test_task_input("p1");
            updated.title = "Release coordination drawings".into();
            updated.description = Some("Issued after final review".into());
            updated.category = Some("Release".into());
            updated.status = "waiting".into();
            updated.waiting_on = Some("Controls vendor".into());
            updated.follow_up_date = Some("2026-09-24".into());
            let saved = database.update_task(id, &updated).unwrap();
            assert_eq!(saved.title, "Release coordination drawings");
            assert_eq!(saved.status, "waiting");
            assert_eq!(
                saved.description.as_deref(),
                Some("Issued after final review")
            );
            assert_eq!(saved.category.as_deref(), Some("Release"));
            database.connection.execute("INSERT INTO projects (id,number,name,project_path,created_at_utc,updated_at_utc) VALUES ('p2','P-200','Other Project','C:\\Projects\\P-200','2026-09-22T00:00:00Z','2026-09-22T00:00:00Z')",[]).unwrap();
            database.connection.execute("INSERT INTO rfis (id,project_id,number,subject,question,status,created_date,created_at_utc,updated_at_utc) VALUES ('rfi-task-link','p1','RFI-1','Linked RFI','Question','draft','2026-09-22','2026-09-22T00:00:00Z','2026-09-22T00:00:00Z')",[]).unwrap();
            database.connection.execute("INSERT INTO rfi_task_relationships (rfi_id,task_id,created_at_utc) VALUES ('rfi-task-link',?1,'2026-09-22T00:00:00Z')",[id]).unwrap();
            updated.project_id = "p2".into();
            assert_eq!(
                database.update_task(id, &updated).err().unwrap().code,
                "TASK_PROJECT_RELATIONSHIP_CONFLICT"
            );
            database
                .set_task_status(id, "waiting", Some("Controls vendor"), Some("2026-09-24"))
                .unwrap();
            let waiting_since: Option<String> = database
                .connection
                .query_row(
                    "SELECT waiting_since_utc FROM tasks WHERE id=?1",
                    [id],
                    |row| row.get(0),
                )
                .unwrap();
            assert!(waiting_since.is_some());
            database
                .set_task_status(id, "in_progress", None, None)
                .unwrap();
            let cleared: Option<String> = database
                .connection
                .query_row(
                    "SELECT waiting_since_utc FROM tasks WHERE id=?1",
                    [id],
                    |row| row.get(0),
                )
                .unwrap();
            assert!(cleared.is_none());
            let events: i64 = database
                .connection
                .query_row(
                    "SELECT COUNT(*) FROM activity_events WHERE entity_id=?1",
                    [id],
                    |row| row.get(0),
                )
                .unwrap();
            assert_eq!(events, 4);
            assert_eq!(
                database
                    .set_task_status("missing", "completed", None, None)
                    .unwrap_err()
                    .code,
                "TASK_NOT_FOUND"
            );
        }
        {
            let database = Database::open(&path).unwrap();
            let tasks = database.list_tasks().unwrap();
            assert_eq!(tasks.len(), 1);
            assert_eq!(tasks[0].status, "in_progress");
        }
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn rfi_lifecycle_attention_relationship_and_attachment_reference_persist() {
        let path = std::env::temp_dir().join(format!(
            "project-engineer-rfi-{}.sqlite3",
            uuid::Uuid::new_v4()
        ));
        let external_file =
            std::env::temp_dir().join(format!("project-engineer-rfi-{}.txt", uuid::Uuid::new_v4()));
        std::fs::write(&external_file, "RFI supporting note").unwrap();
        {
            let mut database = Database::open(&path).unwrap();
            insert_test_project(&database);
            let task = database
                .create_task("task-rfi", &test_task_input("p1"))
                .unwrap();
            let mut input = test_rfi_input("p1");
            input.related_task_id = Some(task.id.clone());
            let created = database.create_rfi("rfi-1", &input).unwrap();
            assert_eq!(created.status, "open");
            assert_eq!(created.rfi_location.as_deref(), Some("Mechanical room"));
            assert_eq!(created.drawing_number.as_deref(), Some("M-401"));
            assert_eq!(created.requested_by.as_deref(), Some("Project Engineer"));
            assert_eq!(created.related_task_id.as_deref(), Some(task.id.as_str()));
            assert_eq!(
                database
                    .list_rfi_attention("2026-09-22", "2026-10-06")
                    .unwrap()
                    .len(),
                1
            );
            let attachment = database
                .add_rfi_attachment("rfi-1", &external_file.to_string_lossy())
                .unwrap();
            assert_eq!(database.list_rfi_attachments("rfi-1").unwrap().len(), 1);
            database.remove_rfi_attachment(&attachment.id).unwrap();
            input.status = "response_received".into();
            input.response = Some("Use sequence B.".into());
            input.response_received_date = Some("2026-09-24".into());
            database.update_rfi("rfi-1", &input).unwrap();
            assert!(database
                .list_rfi_attention("2026-09-22", "2026-10-06")
                .unwrap()
                .is_empty());
            let events: i64 = database
                .connection
                .query_row(
                    "SELECT COUNT(*) FROM activity_events WHERE entity_id='rfi-1'",
                    [],
                    |row| row.get(0),
                )
                .unwrap();
            assert_eq!(events, 4);
        }
        {
            let database = Database::open(&path).unwrap();
            let rfi = database.get_rfi("rfi-1").unwrap();
            assert_eq!(rfi.status, "response_received");
            assert_eq!(rfi.response.as_deref(), Some("Use sequence B."));
            assert_eq!(rfi.suggested_solution.as_deref(), Some("Use sequence B."));
        }
        std::fs::remove_file(external_file).unwrap();
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn submittal_lifecycle_and_revision_persist() {
        let path = std::env::temp_dir().join(format!(
            "project-engineer-submittal-{}.sqlite3",
            uuid::Uuid::new_v4()
        ));
        {
            let mut db = Database::open(&path).unwrap();
            insert_test_project(&db);
            let mut input = test_submittal_input("p1");
            let created = db.create_submittal("sub-1", &input).unwrap();
            assert_eq!(db.list_submittal_attention().unwrap().len(), 1);
            input.status = "revise_and_resubmit".into();
            input.response_date = Some("2026-09-24".into());
            input.resubmission_required = true;
            db.update_submittal(&created.id, &input).unwrap();
            let mut revision = test_submittal_input("p1");
            revision.parent_submittal_id = Some(created.id.clone());
            revision.status = "preparing".into();
            revision.revision = Some("1".into());
            db.create_submittal("sub-2", &revision).unwrap();
            input.status = "closed".into();
            db.update_submittal("sub-1", &input).unwrap();
            assert!(db.list_submittal_attention().unwrap().is_empty());
        }
        {
            let db = Database::open(&path).unwrap();
            let original = db.get_submittal("sub-1").unwrap();
            assert_eq!(original.status, "closed");
            assert_eq!(original.disposition.as_deref(), Some("revise_and_resubmit"));
            assert_eq!(
                db.get_submittal("sub-2")
                    .unwrap()
                    .parent_submittal_id
                    .as_deref(),
                Some("sub-1")
            );
        }
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn files_notes_contacts_activity_and_backup_reopen() {
        let root =
            std::env::temp_dir().join(format!("project-engineer-final-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();
        let db_path = root.join("workspace.sqlite3");
        let source = root.join("drawing.pdf");
        std::fs::write(&source, "drawing").unwrap();
        {
            let mut db = Database::open(&db_path).unwrap();
            insert_test_project(&db);
            let file = db
                .create_file(&FileInput {
                    project_id: "p1".into(),
                    source_path: source.to_string_lossy().to_string(),
                    operation: "register".into(),
                    discipline: Some("controls".into()),
                    drawing_number: Some("M-101".into()),
                    title: Some("Plan".into()),
                    revision: Some("A".into()),
                    revision_date: Some("2026-09-22".into()),
                    received_date: Some("2026-09-23".into()),
                })
                .unwrap();
            db.update_file_metadata(
                &file.id,
                &FileMetadataInput {
                    discipline: Some("mechanical".into()),
                    drawing_number: Some("M-101".into()),
                    title: Some("Plan".into()),
                    revision: Some("B".into()),
                    revision_date: Some("2026-09-24".into()),
                    received_date: Some("2026-09-25".into()),
                    is_current: false,
                },
            )
            .unwrap();
            db.create_note(&NoteInput {
                project_id: Some("p1".into()),
                body: "Field note".into(),
            })
            .unwrap();
            db.create_contact(&ContactInput {
                name: "Alex Smith".into(),
                company: Some("Engineer".into()),
                email: None,
                phone: None,
                role: Some("Designer".into()),
            })
            .unwrap();
            assert!(db
                .list_activity()
                .unwrap()
                .iter()
                .any(|event| event.event_type == "note.created"));
            db.checkpoint().unwrap();
        }
        let backup = root.join("backup.sqlite3");
        std::fs::copy(&db_path, &backup).unwrap();
        {
            let db = Database::open(&backup).unwrap();
            assert_eq!(db.list_files().unwrap()[0].revision.as_deref(), Some("B"));
            assert!(!db.list_files().unwrap()[0].is_current);
            assert_eq!(db.list_notes().unwrap().len(), 1);
            assert_eq!(db.list_contacts().unwrap().len(), 1);
        }
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn realistic_task_volume_lists_without_data_loss() {
        let path = std::env::temp_dir().join(format!(
            "project-engineer-volume-{}.sqlite3",
            uuid::Uuid::new_v4()
        ));
        {
            let mut db = Database::open(&path).unwrap();
            insert_test_project(&db);
            for index in 0..500 {
                let mut input = test_task_input("p1");
                input.title = format!("Coordination task {index:03}");
                db.create_task(&format!("task-{index:03}"), &input).unwrap();
            }
            let tasks = db.list_tasks().unwrap();
            assert_eq!(tasks.len(), 500);
            assert!(tasks
                .iter()
                .any(|task| task.title == "Coordination task 000"));
            assert!(tasks
                .iter()
                .any(|task| task.title == "Coordination task 499"));
            let events: i64 = db
                .connection
                .query_row("SELECT COUNT(*) FROM activity_events", [], |row| row.get(0))
                .unwrap();
            assert_eq!(events, 500);
        }
        {
            let db = Database::open(&path).unwrap();
            assert_eq!(db.list_tasks().unwrap().len(), 500);
        }
        std::fs::remove_file(path).unwrap();
    }
}

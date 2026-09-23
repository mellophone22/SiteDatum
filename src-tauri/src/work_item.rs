use rusqlite::params;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    error::{AppError, AppResult},
    persistence::Database,
    task,
};

pub const TYPES: [&str; 10] = [
    "meeting_minute",
    "procurement",
    "change_event",
    "transmittal",
    "milestone",
    "punch_item",
    "daily_report",
    "startup_check",
    "commissioning_check",
    "commissioning_issue",
];
pub const STATUSES: [&str; 6] = [
    "draft",
    "open",
    "in_progress",
    "waiting",
    "completed",
    "cancelled",
];

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkItemInput {
    pub project_id: String,
    pub item_type: String,
    pub number: Option<String>,
    pub title: String,
    pub description: Option<String>,
    pub status: String,
    pub priority: String,
    pub due_date: Option<String>,
    pub occurred_date: Option<String>,
    pub responsible_party: Option<String>,
    pub company: Option<String>,
    pub location: Option<String>,
    pub amount_cents: Option<i64>,
    pub checklist_total: i64,
    pub checklist_completed: i64,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct WorkItem {
    pub id: String,
    pub project_id: String,
    pub project_number: String,
    pub project_name: String,
    pub item_type: String,
    pub number: Option<String>,
    pub title: String,
    pub description: Option<String>,
    pub status: String,
    pub priority: String,
    pub due_date: Option<String>,
    pub occurred_date: Option<String>,
    pub responsible_party: Option<String>,
    pub company: Option<String>,
    pub location: Option<String>,
    pub amount_cents: Option<i64>,
    pub checklist_total: i64,
    pub checklist_completed: i64,
    pub archived_at_utc: Option<String>,
    pub created_at_utc: String,
    pub updated_at_utc: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectTemplateInput {
    pub name: String,
    pub task_titles: Vec<String>,
    pub milestone_titles: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectTemplate {
    pub id: String,
    pub name: String,
    pub task_titles: Vec<String>,
    pub milestone_titles: Vec<String>,
}

fn error(code: &'static str, message: &str) -> AppError {
    AppError::from_technical(code, message, "Correct the record and try again.", message)
}
fn clean(value: &Option<String>) -> Option<String> {
    value
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(str::to_owned)
}
pub fn validate(input: &WorkItemInput) -> AppResult<()> {
    if input.project_id.trim().is_empty() {
        return Err(error("WORK_ITEM_PROJECT_REQUIRED", "Choose a project."));
    }
    if !TYPES.contains(&input.item_type.as_str()) {
        return Err(error("WORK_ITEM_TYPE_INVALID", "Choose a valid register."));
    }
    if input.title.trim().is_empty() {
        return Err(error("WORK_ITEM_TITLE_REQUIRED", "Enter a title."));
    }
    if !STATUSES.contains(&input.status.as_str()) {
        return Err(error("WORK_ITEM_STATUS_INVALID", "Choose a valid status."));
    }
    if !["low", "medium", "high", "urgent"].contains(&input.priority.as_str()) {
        return Err(error(
            "WORK_ITEM_PRIORITY_INVALID",
            "Choose a valid priority.",
        ));
    }
    task::validate_date(input.due_date.as_deref())?;
    task::validate_date(input.occurred_date.as_deref())?;
    if input.checklist_total < 0
        || input.checklist_completed < 0
        || input.checklist_completed > input.checklist_total
    {
        return Err(error(
            "WORK_ITEM_CHECKLIST_INVALID",
            "Completed checklist steps cannot exceed total steps.",
        ));
    }
    if input.item_type == "daily_report" && input.occurred_date.is_none() {
        return Err(error(
            "DAILY_REPORT_DATE_REQUIRED",
            "Choose the daily report date.",
        ));
    }
    if input.item_type == "transmittal" && clean(&input.number).is_none() {
        return Err(error(
            "TRANSMITTAL_NUMBER_REQUIRED",
            "Enter a transmittal number.",
        ));
    }
    Ok(())
}

impl Database {
    pub fn list_work_items(&self) -> AppResult<Vec<WorkItem>> {
        let mut s=self.connection.prepare("SELECT w.id,w.project_id,p.number,p.name,w.item_type,w.number,w.title,w.description,w.status,w.priority,w.due_date,w.occurred_date,w.responsible_party,w.company,w.location,w.amount_cents,w.checklist_total,w.checklist_completed,w.archived_at_utc,w.created_at_utc,w.updated_at_utc FROM work_items w JOIN projects p ON p.id=w.project_id WHERE p.archived_at_utc IS NULL ORDER BY w.due_date IS NULL,w.due_date,w.updated_at_utc DESC").map_err(db)?;
        let rows = s.query_map([], from_row).map_err(db)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(db)
    }
    pub fn create_work_item(&mut self, input: &WorkItemInput) -> AppResult<WorkItem> {
        validate(input)?;
        let id = Uuid::new_v4().to_string();
        let tx = self.connection.transaction().map_err(db)?;
        tx.execute("INSERT INTO work_items(id,project_id,item_type,number,title,description,status,priority,due_date,occurred_date,responsible_party,company,location,amount_cents,checklist_total,checklist_completed,created_at_utc,updated_at_utc) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,strftime('%Y-%m-%dT%H:%M:%fZ','now'),strftime('%Y-%m-%dT%H:%M:%fZ','now'))",params![id,input.project_id,input.item_type,clean(&input.number),input.title.trim(),clean(&input.description),input.status,input.priority,input.due_date,input.occurred_date,clean(&input.responsible_party),clean(&input.company),clean(&input.location),input.amount_cents,input.checklist_total,input.checklist_completed]).map_err(db)?;
        tx.execute("INSERT INTO activity_events(event_type,entity_type,entity_id,summary,occurred_at_utc) VALUES('work_item.created',?1,?2,?3,strftime('%Y-%m-%dT%H:%M:%fZ','now'))",params![input.item_type,id,format!("Created {}",input.title.trim())]).map_err(db)?;
        tx.commit().map_err(db)?;
        self.list_work_items()?
            .into_iter()
            .find(|v| v.id == id)
            .ok_or_else(|| {
                error(
                    "WORK_ITEM_NOT_FOUND",
                    "The created record could not be read.",
                )
            })
    }
    pub fn update_work_item(&mut self, id: &str, input: &WorkItemInput) -> AppResult<WorkItem> {
        validate(input)?;
        let tx = self.connection.transaction().map_err(db)?;
        let n=tx.execute("UPDATE work_items SET project_id=?2,item_type=?3,number=?4,title=?5,description=?6,status=?7,priority=?8,due_date=?9,occurred_date=?10,responsible_party=?11,company=?12,location=?13,amount_cents=?14,checklist_total=?15,checklist_completed=?16,updated_at_utc=strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE id=?1",params![id,input.project_id,input.item_type,clean(&input.number),input.title.trim(),clean(&input.description),input.status,input.priority,input.due_date,input.occurred_date,clean(&input.responsible_party),clean(&input.company),clean(&input.location),input.amount_cents,input.checklist_total,input.checklist_completed]).map_err(db)?;
        if n == 0 {
            return Err(error("WORK_ITEM_NOT_FOUND", "The record no longer exists."));
        }
        tx.execute("INSERT INTO activity_events(event_type,entity_type,entity_id,summary,occurred_at_utc) VALUES('work_item.updated',?1,?2,?3,strftime('%Y-%m-%dT%H:%M:%fZ','now'))",params![input.item_type,id,format!("Updated {}",input.title.trim())]).map_err(db)?;
        tx.commit().map_err(db)?;
        self.list_work_items()?
            .into_iter()
            .find(|v| v.id == id)
            .ok_or_else(|| error("WORK_ITEM_NOT_FOUND", "The record no longer exists."))
    }
    pub fn bulk_set_work_item_status(&mut self, ids: &[String], status: &str) -> AppResult<usize> {
        if !STATUSES.contains(&status) {
            return Err(error("WORK_ITEM_STATUS_INVALID", "Choose a valid status."));
        }
        if ids.is_empty() {
            return Ok(0);
        }
        let tx = self.connection.transaction().map_err(db)?;
        let mut changed = 0;
        for id in ids {
            changed+=tx.execute("UPDATE work_items SET status=?2,updated_at_utc=strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE id=?1",params![id,status]).map_err(db)?;
            tx.execute("INSERT INTO activity_events(event_type,entity_type,entity_id,summary,occurred_at_utc) SELECT 'work_item.status_changed',item_type,id,?2,strftime('%Y-%m-%dT%H:%M:%fZ','now') FROM work_items WHERE id=?1",params![id,format!("Status changed to {status}")]).map_err(db)?;
        }
        tx.commit().map_err(db)?;
        Ok(changed)
    }
    pub fn list_project_templates(&self) -> AppResult<Vec<ProjectTemplate>> {
        let mut s=self.connection.prepare("SELECT id,name,task_titles_json,milestone_titles_json FROM project_templates ORDER BY name COLLATE NOCASE").map_err(db)?;
        let rows = s
            .query_map([], |r| {
                Ok(ProjectTemplate {
                    id: r.get(0)?,
                    name: r.get(1)?,
                    task_titles: serde_json::from_str(&r.get::<_, String>(2)?).unwrap_or_default(),
                    milestone_titles: serde_json::from_str(&r.get::<_, String>(3)?)
                        .unwrap_or_default(),
                })
            })
            .map_err(db)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(db)
    }
    pub fn create_project_template(
        &mut self,
        input: &ProjectTemplateInput,
    ) -> AppResult<ProjectTemplate> {
        let name = input.name.trim();
        if name.is_empty() {
            return Err(error("TEMPLATE_NAME_REQUIRED", "Enter a template name."));
        }
        let id = Uuid::new_v4().to_string();
        self.connection.execute("INSERT INTO project_templates(id,name,task_titles_json,milestone_titles_json,created_at_utc,updated_at_utc) VALUES(?1,?2,?3,?4,strftime('%Y-%m-%dT%H:%M:%fZ','now'),strftime('%Y-%m-%dT%H:%M:%fZ','now'))",params![id,name,serde_json::to_string(&input.task_titles).map_err(json_error)?,serde_json::to_string(&input.milestone_titles).map_err(json_error)?]).map_err(db)?;
        self.list_project_templates()?
            .into_iter()
            .find(|v| v.id == id)
            .ok_or_else(|| error("TEMPLATE_NOT_FOUND", "The template could not be read."))
    }
    pub fn apply_project_template(
        &mut self,
        template_id: &str,
        project_id: &str,
    ) -> AppResult<usize> {
        let template = self
            .list_project_templates()?
            .into_iter()
            .find(|v| v.id == template_id)
            .ok_or_else(|| error("TEMPLATE_NOT_FOUND", "The template no longer exists."))?;
        let tx = self.connection.transaction().map_err(db)?;
        let mut count = 0;
        for title in template
            .task_titles
            .iter()
            .map(|v| v.trim())
            .filter(|v| !v.is_empty())
        {
            tx.execute("INSERT INTO tasks(id,project_id,title,priority,status,created_at_utc,updated_at_utc) VALUES(?1,?2,?3,'medium','open',strftime('%Y-%m-%dT%H:%M:%fZ','now'),strftime('%Y-%m-%dT%H:%M:%fZ','now'))",params![Uuid::new_v4().to_string(),project_id,title]).map_err(db)?;
            count += 1;
        }
        for title in template
            .milestone_titles
            .iter()
            .map(|v| v.trim())
            .filter(|v| !v.is_empty())
        {
            tx.execute("INSERT INTO work_items(id,project_id,item_type,title,status,priority,created_at_utc,updated_at_utc) VALUES(?1,?2,'milestone',?3,'open','medium',strftime('%Y-%m-%dT%H:%M:%fZ','now'),strftime('%Y-%m-%dT%H:%M:%fZ','now'))",params![Uuid::new_v4().to_string(),project_id,title]).map_err(db)?;
            count += 1;
        }
        tx.execute("INSERT INTO activity_events(event_type,entity_type,entity_id,summary,occurred_at_utc) VALUES('project_template.applied','project',?1,?2,strftime('%Y-%m-%dT%H:%M:%fZ','now'))",params![project_id,format!("Applied template {} ({count} records)",template.name)]).map_err(db)?;
        tx.commit().map_err(db)?;
        Ok(count)
    }
}
fn from_row(r: &rusqlite::Row<'_>) -> rusqlite::Result<WorkItem> {
    Ok(WorkItem {
        id: r.get(0)?,
        project_id: r.get(1)?,
        project_number: r.get(2)?,
        project_name: r.get(3)?,
        item_type: r.get(4)?,
        number: r.get(5)?,
        title: r.get(6)?,
        description: r.get(7)?,
        status: r.get(8)?,
        priority: r.get(9)?,
        due_date: r.get(10)?,
        occurred_date: r.get(11)?,
        responsible_party: r.get(12)?,
        company: r.get(13)?,
        location: r.get(14)?,
        amount_cents: r.get(15)?,
        checklist_total: r.get(16)?,
        checklist_completed: r.get(17)?,
        archived_at_utc: r.get(18)?,
        created_at_utc: r.get(19)?,
        updated_at_utc: r.get(20)?,
    })
}
fn db(e: rusqlite::Error) -> AppError {
    AppError::from_technical(
        "DATABASE_ERROR",
        "The register could not be updated.",
        "Try again. If it continues, create a backup and review the local log.",
        e.to_string(),
    )
}
fn json_error(e: serde_json::Error) -> AppError {
    AppError::from_technical(
        "TEMPLATE_INVALID",
        "The template content is invalid.",
        "Correct the template and try again.",
        e.to_string(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn validates_domain_specific_requirements() {
        let mut input = WorkItemInput {
            project_id: "p".into(),
            item_type: "daily_report".into(),
            number: None,
            title: "Site report".into(),
            description: None,
            status: "open".into(),
            priority: "medium".into(),
            due_date: None,
            occurred_date: None,
            responsible_party: None,
            company: None,
            location: None,
            amount_cents: None,
            checklist_total: 0,
            checklist_completed: 0,
        };
        assert_eq!(
            validate(&input).unwrap_err().code,
            "DAILY_REPORT_DATE_REQUIRED"
        );
        input.occurred_date = Some("2026-09-22".into());
        assert!(validate(&input).is_ok());
        input.item_type = "transmittal".into();
        assert_eq!(
            validate(&input).unwrap_err().code,
            "TRANSMITTAL_NUMBER_REQUIRED"
        );
    }
    #[test]
    fn register_bulk_and_template_workflows_persist() {
        let path =
            std::env::temp_dir().join(format!("anydesk-operations-{}.sqlite3", Uuid::new_v4()));
        let mut db = Database::open(&path).unwrap();
        db.connection.execute("INSERT INTO projects(id,number,name,status,phase,project_path,created_at_utc,updated_at_utc) VALUES('p1','P-1','Test','active','engineering','C:\\Test',strftime('%Y-%m-%dT%H:%M:%fZ','now'),strftime('%Y-%m-%dT%H:%M:%fZ','now'))",[]).unwrap();
        let item = db
            .create_work_item(&WorkItemInput {
                project_id: "p1".into(),
                item_type: "punch_item".into(),
                number: Some("P-01".into()),
                title: "Repair sensor".into(),
                description: None,
                status: "open".into(),
                priority: "high".into(),
                due_date: Some("2026-10-01".into()),
                occurred_date: None,
                responsible_party: Some("Vendor".into()),
                company: None,
                location: Some("AHU-1".into()),
                amount_cents: None,
                checklist_total: 0,
                checklist_completed: 0,
            })
            .unwrap();
        assert_eq!(
            db.bulk_set_work_item_status(&[item.id], "completed")
                .unwrap(),
            1
        );
        assert_eq!(db.list_work_items().unwrap()[0].status, "completed");
        let template = db
            .create_project_template(&ProjectTemplateInput {
                name: "Standard startup".into(),
                task_titles: vec!["Review submittals".into()],
                milestone_titles: vec!["Controls complete".into()],
            })
            .unwrap();
        assert_eq!(db.apply_project_template(&template.id, "p1").unwrap(), 2);
        assert_eq!(db.list_tasks().unwrap().len(), 1);
        assert!(db
            .list_work_items()
            .unwrap()
            .iter()
            .any(|v| v.item_type == "milestone"));
        drop(db);
        std::fs::remove_file(path).unwrap();
    }
}

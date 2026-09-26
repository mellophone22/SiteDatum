use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::{AppError, AppResult};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskInput {
    pub project_id: String,
    pub title: String,
    pub priority: String,
    pub status: String,
    pub due_date: Option<String>,
    pub follow_up_date: Option<String>,
    pub waiting_on: Option<String>,
    pub description: Option<String>,
    pub category: Option<String>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Task {
    pub id: String,
    pub project_id: String,
    pub project_number: String,
    pub project_name: String,
    pub title: String,
    pub description: Option<String>,
    pub priority: String,
    pub status: String,
    pub category: Option<String>,
    pub due_date: Option<String>,
    pub follow_up_date: Option<String>,
    pub waiting_on: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AttentionSection {
    pub key: &'static str,
    pub label: &'static str,
    pub tasks: Vec<Task>,
}

pub fn validate(input: &TaskInput) -> AppResult<()> {
    if input.project_id.trim().is_empty() {
        return Err(err(
            "TASK_PROJECT_REQUIRED",
            "Choose a project for this task.",
        ));
    }
    if input.title.trim().is_empty() {
        return Err(err("TASK_REQUIRED_TITLE", "Task title is required."));
    }
    if !["low", "medium", "high", "urgent"].contains(&input.priority.as_str()) {
        return Err(err(
            "TASK_INVALID_PRIORITY",
            "Choose a valid task priority.",
        ));
    }
    validate_status(&input.status, input.waiting_on.as_deref())?;
    validate_date(input.due_date.as_deref())?;
    validate_date(input.follow_up_date.as_deref())?;
    Ok(())
}

pub fn validate_status(status: &str, waiting_on: Option<&str>) -> AppResult<()> {
    if ![
        "open",
        "in_progress",
        "waiting",
        "blocked",
        "completed",
        "cancelled",
    ]
    .contains(&status)
    {
        return Err(err("TASK_INVALID_STATUS", "Choose a valid task status."));
    }
    if status == "waiting"
        && waiting_on
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .is_none()
    {
        return Err(err(
            "TASK_WAITING_ON_REQUIRED",
            "Specify who the task is waiting on.",
        ));
    }
    Ok(())
}

pub fn validate_date(date: Option<&str>) -> AppResult<()> {
    if let Some(date) = date {
        let bytes = date.as_bytes();
        let valid_shape = bytes.len() == 10
            && bytes[4] == b'-'
            && bytes[7] == b'-'
            && bytes
                .iter()
                .enumerate()
                .all(|(index, value)| index == 4 || index == 7 || value.is_ascii_digit());
        let month = date
            .get(5..7)
            .and_then(|value| value.parse::<u32>().ok())
            .unwrap_or(0);
        let day = date
            .get(8..10)
            .and_then(|value| value.parse::<u32>().ok())
            .unwrap_or(0);
        let year = date
            .get(0..4)
            .and_then(|value| value.parse::<u32>().ok())
            .unwrap_or(0);
        let leap_year = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
        let max_day = match month {
            1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
            4 | 6 | 9 | 11 => 30,
            2 if leap_year => 29,
            2 => 28,
            _ => 0,
        };
        if !valid_shape || year < 1000 || day == 0 || day > max_day {
            return Err(err("TASK_INVALID_DATE", "Enter a valid date."));
        }
    }
    Ok(())
}

pub fn attention_sections(tasks: Vec<Task>, today: &str, through: &str) -> Vec<AttentionSection> {
    let mut overdue = Vec::new();
    let mut due_today = Vec::new();
    let mut follow_up = Vec::new();
    let mut upcoming = Vec::new();
    let mut waiting = Vec::new();
    for task in tasks {
        if task.status == "completed" || task.status == "cancelled" {
            continue;
        }
        if task.status == "waiting" {
            if task
                .follow_up_date
                .as_deref()
                .is_some_and(|date| date <= today)
            {
                follow_up.push(task);
            } else {
                waiting.push(task);
            }
            continue;
        }
        match task.due_date.as_deref() {
            Some(date) if date < today => overdue.push(task),
            Some(date) if date == today => due_today.push(task),
            Some(date) if date <= through => upcoming.push(task),
            _ => {}
        }
    }
    vec![
        section("overdue", "Overdue", overdue),
        section("today", "Today", due_today),
        section("follow_up", "Follow-up", follow_up),
        section("waiting", "Waiting", waiting),
        section("upcoming", "Upcoming", upcoming),
    ]
}

fn section(key: &'static str, label: &'static str, tasks: Vec<Task>) -> AttentionSection {
    AttentionSection { key, label, tasks }
}

pub fn id() -> String {
    Uuid::new_v4().to_string()
}

fn err(code: &'static str, msg: &str) -> AppError {
    AppError::from_technical(
        code,
        msg,
        "Correct the task information and try again.",
        msg,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn waiting_requires_a_real_waiting_on_value() {
        assert_eq!(
            validate_status("waiting", None).unwrap_err().code,
            "TASK_WAITING_ON_REQUIRED"
        );
        assert_eq!(
            validate_status("waiting", Some("  ")).unwrap_err().code,
            "TASK_WAITING_ON_REQUIRED"
        );
        assert!(validate_status("waiting", Some("Controls vendor")).is_ok());
    }

    #[test]
    fn only_valid_calendar_date_shape_is_accepted() {
        assert!(validate_date(Some("2026-09-22")).is_ok());
        assert!(validate_date(Some("2026-9-2")).is_err());
        assert!(validate_date(Some("2026-02-29")).is_err());
        assert!(validate_date(Some("2024-02-29")).is_ok());
        assert!(validate_date(Some("2026-04-31")).is_err());
    }

    fn sample(id: &str, status: &str, due: Option<&str>, follow_up: Option<&str>) -> Task {
        Task {
            id: id.into(),
            project_id: "p".into(),
            project_number: "P-1".into(),
            project_name: "Project".into(),
            title: id.into(),
            description: None,
            priority: "medium".into(),
            status: status.into(),
            category: None,
            due_date: due.map(str::to_owned),
            follow_up_date: follow_up.map(str::to_owned),
            waiting_on: None,
        }
    }

    #[test]
    fn attention_groups_are_exclusive_and_use_inclusive_fourteen_day_boundary() {
        let groups = attention_sections(
            vec![
                sample("late", "open", Some("2026-09-20"), None),
                sample("today", "blocked", Some("2026-09-22"), None),
                sample("soon", "open", Some("2026-10-06"), None),
                sample("outside", "open", Some("2026-10-07"), None),
                sample("followup", "waiting", None, Some("2026-09-22")),
                sample("waiting", "waiting", None, None),
                sample("done", "completed", Some("2026-09-20"), None),
                sample("cancelled", "cancelled", Some("2026-09-20"), None),
            ],
            "2026-09-22",
            "2026-10-06",
        );
        assert_eq!(groups[0].tasks[0].id, "late");
        assert_eq!(groups[1].tasks[0].id, "today");
        assert_eq!(groups[2].tasks[0].id, "followup");
        assert_eq!(groups[3].tasks[0].id, "waiting");
        assert_eq!(groups[4].tasks[0].id, "soon");
        assert!(groups
            .iter()
            .flat_map(|group| &group.tasks)
            .all(|task| task.id != "outside" && task.id != "done" && task.id != "cancelled"));
    }
}

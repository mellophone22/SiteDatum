use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::task::validate_date;

pub const STATUSES: [&str; 9] = [
    "draft",
    "preparing",
    "submitted",
    "under_review",
    "approved",
    "approved_as_noted",
    "revise_and_resubmit",
    "rejected",
    "closed",
];
pub const DISPOSITIONS: [&str; 4] = [
    "approved",
    "approved_as_noted",
    "revise_and_resubmit",
    "rejected",
];

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubmittalInput {
    pub project_id: String,
    pub parent_submittal_id: Option<String>,
    pub number: String,
    pub name: String,
    pub package: Option<String>,
    pub revision: Option<String>,
    pub recipient: Option<String>,
    pub status: String,
    pub created_date: String,
    pub submitted_date: Option<String>,
    pub response_date: Option<String>,
    pub resubmission_required: bool,
    pub notes: Option<String>,
    pub related_task_id: Option<String>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Submittal {
    pub id: String,
    pub project_id: String,
    pub project_number: String,
    pub project_name: String,
    pub parent_submittal_id: Option<String>,
    pub number: String,
    pub name: String,
    pub package: Option<String>,
    pub revision: Option<String>,
    pub recipient: Option<String>,
    pub status: String,
    pub disposition: Option<String>,
    pub created_date: String,
    pub submitted_date: Option<String>,
    pub response_date: Option<String>,
    pub resubmission_required: bool,
    pub notes: Option<String>,
    pub related_task_id: Option<String>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AttachmentReference {
    pub id: String,
    pub file_path: String,
    pub display_name: String,
}

pub fn validate(input: &SubmittalInput) -> AppResult<()> {
    required(
        &input.project_id,
        "SUBMITTAL_PROJECT_REQUIRED",
        "Choose a project for this submittal.",
    )?;
    required(
        &input.number,
        "SUBMITTAL_NUMBER_REQUIRED",
        "Submittal number is required.",
    )?;
    required(
        &input.name,
        "SUBMITTAL_NAME_REQUIRED",
        "Submittal name is required.",
    )?;
    if !STATUSES.contains(&input.status.as_str()) {
        return Err(error(
            "SUBMITTAL_INVALID_STATUS",
            "Choose a valid submittal status.",
        ));
    }
    validate_date(Some(&input.created_date))
        .map_err(|_| error("SUBMITTAL_INVALID_DATE", "Enter a valid submittal date."))?;
    validate_date(input.submitted_date.as_deref())
        .map_err(|_| error("SUBMITTAL_INVALID_DATE", "Enter a valid submittal date."))?;
    validate_date(input.response_date.as_deref())
        .map_err(|_| error("SUBMITTAL_INVALID_DATE", "Enter a valid submittal date."))?;
    if !["draft", "preparing"].contains(&input.status.as_str()) && clean(&input.recipient).is_none()
    {
        return Err(error(
            "SUBMITTAL_RECIPIENT_REQUIRED",
            "Enter a recipient before submitting or reviewing this submittal.",
        ));
    }
    if (input.status == "closed" || disposition_for_status(&input.status).is_some())
        && input.response_date.is_none()
    {
        return Err(error(
            "SUBMITTAL_RESPONSE_REQUIRED",
            "Record a disposition response date before recording this disposition.",
        ));
    }
    let disposition = disposition_for_status(&input.status);
    if input.resubmission_required
        && ![Some("revise_and_resubmit"), None].contains(&disposition)
        && input.status != "closed"
    {
        return Err(error(
            "SUBMITTAL_RESUBMISSION_INVALID",
            "Resubmission is only valid after Revise & Resubmit.",
        ));
    }
    if input.resubmission_required
        && !["revise_and_resubmit", "closed"].contains(&input.status.as_str())
    {
        return Err(error(
            "SUBMITTAL_RESUBMISSION_INVALID",
            "Resubmission is only valid after Revise & Resubmit.",
        ));
    }
    Ok(())
}

pub fn disposition_for_status(status: &str) -> Option<&'static str> {
    if DISPOSITIONS.contains(&status) {
        Some(match status {
            "approved" => "approved",
            "approved_as_noted" => "approved_as_noted",
            "revise_and_resubmit" => "revise_and_resubmit",
            "rejected" => "rejected",
            _ => unreachable!(),
        })
    } else {
        None
    }
}
pub fn new_id() -> String {
    Uuid::new_v4().to_string()
}
pub fn clean(value: &Option<String>) -> Option<String> {
    value
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}
pub fn required(value: &str, code: &'static str, message: &str) -> AppResult<String> {
    let value = value.trim();
    if value.is_empty() {
        Err(error(code, message))
    } else {
        Ok(value.to_owned())
    }
}
pub fn error(code: &'static str, message: &str) -> AppError {
    AppError::from_technical(
        code,
        message,
        "Correct the submittal information and try again.",
        message,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    fn input() -> SubmittalInput {
        SubmittalInput {
            project_id: "project".into(),
            parent_submittal_id: None,
            number: "SUB-01".into(),
            name: "Controls panel".into(),
            package: None,
            revision: None,
            recipient: None,
            status: "draft".into(),
            created_date: "2026-09-22".into(),
            submitted_date: None,
            response_date: None,
            resubmission_required: false,
            notes: None,
            related_task_id: None,
        }
    }
    #[test]
    fn lifecycle_requires_recipient_and_valid_resubmission_state() {
        let mut submittal = input();
        submittal.status = "submitted".into();
        assert_eq!(
            validate(&submittal).unwrap_err().code,
            "SUBMITTAL_RECIPIENT_REQUIRED"
        );
        submittal.recipient = Some("Architect".into());
        assert!(validate(&submittal).is_ok());
        submittal.resubmission_required = true;
        assert_eq!(
            validate(&submittal).unwrap_err().code,
            "SUBMITTAL_RESUBMISSION_INVALID"
        );
        submittal.status = "revise_and_resubmit".into();
        submittal.response_date = Some("2026-09-23".into());
        assert!(validate(&submittal).is_ok());
    }
}

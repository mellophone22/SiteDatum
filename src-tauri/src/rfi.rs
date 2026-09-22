use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::task::validate_date;

pub const STATUSES: [&str; 4] = ["draft", "open", "response_received", "closed"];

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RfiInput {
    pub project_id: String,
    pub number: String,
    pub subject: String,
    pub question: String,
    pub recipient: Option<String>,
    pub status: String,
    pub created_date: String,
    pub submitted_date: Option<String>,
    pub response_due_date: Option<String>,
    pub response_received_date: Option<String>,
    pub response: Option<String>,
    pub notes: Option<String>,
    pub related_task_id: Option<String>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Rfi {
    pub id: String,
    pub project_id: String,
    pub project_number: String,
    pub project_name: String,
    pub number: String,
    pub subject: String,
    pub question: String,
    pub recipient: Option<String>,
    pub status: String,
    pub created_date: String,
    pub submitted_date: Option<String>,
    pub response_due_date: Option<String>,
    pub response_received_date: Option<String>,
    pub response: Option<String>,
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

pub fn validate(input: &RfiInput) -> AppResult<()> {
    required(
        &input.project_id,
        "RFI_PROJECT_REQUIRED",
        "Choose a project for this RFI.",
    )?;
    required(
        &input.number,
        "RFI_NUMBER_REQUIRED",
        "RFI number is required.",
    )?;
    required(
        &input.subject,
        "RFI_SUBJECT_REQUIRED",
        "RFI subject is required.",
    )?;
    required(
        &input.question,
        "RFI_QUESTION_REQUIRED",
        "RFI question is required.",
    )?;
    if !STATUSES.contains(&input.status.as_str()) {
        return Err(error("RFI_INVALID_STATUS", "Choose a valid RFI status."));
    }
    validate_date(Some(&input.created_date))
        .map_err(|_| error("RFI_INVALID_DATE", "Enter a valid RFI date."))?;
    validate_date(input.submitted_date.as_deref())
        .map_err(|_| error("RFI_INVALID_DATE", "Enter a valid RFI date."))?;
    validate_date(input.response_due_date.as_deref())
        .map_err(|_| error("RFI_INVALID_DATE", "Enter a valid RFI date."))?;
    validate_date(input.response_received_date.as_deref())
        .map_err(|_| error("RFI_INVALID_DATE", "Enter a valid RFI date."))?;
    if input.status != "draft" && clean(&input.recipient).is_none() {
        return Err(error(
            "RFI_RECIPIENT_REQUIRED",
            "Enter an RFI recipient before opening it.",
        ));
    }
    if ["response_received", "closed"].contains(&input.status.as_str())
        && clean(&input.response).is_none()
    {
        return Err(error(
            "RFI_RESPONSE_REQUIRED",
            "Record the response before marking the RFI received or closed.",
        ));
    }
    Ok(())
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
        "Correct the RFI information and try again.",
        message,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    fn input() -> RfiInput {
        RfiInput {
            project_id: "project".into(),
            number: "RFI-01".into(),
            subject: "Clarify sequence".into(),
            question: "Which interlock applies?".into(),
            recipient: None,
            status: "draft".into(),
            created_date: "2026-09-22".into(),
            submitted_date: None,
            response_due_date: None,
            response_received_date: None,
            response: None,
            notes: None,
            related_task_id: None,
        }
    }
    #[test]
    fn lifecycle_rules_require_recipient_and_response() {
        let mut rfi = input();
        rfi.status = "open".into();
        assert_eq!(validate(&rfi).unwrap_err().code, "RFI_RECIPIENT_REQUIRED");
        rfi.recipient = Some("Engineer".into());
        assert!(validate(&rfi).is_ok());
        rfi.status = "response_received".into();
        assert_eq!(validate(&rfi).unwrap_err().code, "RFI_RESPONSE_REQUIRED");
        rfi.response = Some("Use sequence B.".into());
        assert!(validate(&rfi).is_ok());
    }
}

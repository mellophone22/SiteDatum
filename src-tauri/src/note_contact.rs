use crate::error::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NoteInput {
    pub project_id: Option<String>,
    pub body: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContactInput {
    pub name: String,
    pub company: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub role: Option<String>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Note {
    pub id: String,
    pub project_id: Option<String>,
    pub project_number: Option<String>,
    pub project_name: Option<String>,
    pub body: String,
    pub created_at_utc: String,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Contact {
    pub id: String,
    pub name: String,
    pub company: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub role: Option<String>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActivityEvent {
    pub event_type: String,
    pub entity_type: String,
    pub summary: String,
    pub occurred_at_utc: String,
}
pub fn id() -> String {
    Uuid::new_v4().to_string()
}
pub fn clean(v: &Option<String>) -> Option<String> {
    v.as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(str::to_owned)
}
pub fn note(input: &NoteInput) -> AppResult<()> {
    if input.body.trim().is_empty() {
        Err(AppError::from_technical(
            "NOTE_BODY_REQUIRED",
            "Enter a note.",
            "Write a note before saving.",
            "empty note",
        ))
    } else {
        Ok(())
    }
}
pub fn contact(input: &ContactInput) -> AppResult<()> {
    if input.name.trim().is_empty() {
        Err(AppError::from_technical(
            "CONTACT_NAME_REQUIRED",
            "Enter a contact name.",
            "Provide a name before saving.",
            "empty contact",
        ))
    } else {
        Ok(())
    }
}

use crate::error::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
};
use uuid::Uuid;

pub const STATUSES: [&str; 4] = ["active", "on_hold", "completed", "cancelled"];
pub const PHASES: [&str; 10] = [
    "preconstruction",
    "engineering",
    "submittals",
    "procurement",
    "construction",
    "programming",
    "startup",
    "commissioning",
    "closeout",
    "custom",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectInput {
    pub number: String,
    pub name: String,
    pub status: String,
    pub phase: String,
    pub custom_phase_name: Option<String>,
    pub customer: Option<String>,
    pub general_contractor: Option<String>,
    pub engineer: Option<String>,
    pub project_manager: Option<String>,
    pub superintendent: Option<String>,
    pub location: Option<String>,
    pub start_date: Option<String>,
    pub target_date: Option<String>,
    pub description: Option<String>,
    pub important_notes: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub id: String,
    pub number: String,
    pub name: String,
    pub status: String,
    pub phase: String,
    pub custom_phase_name: Option<String>,
    pub customer: Option<String>,
    pub general_contractor: Option<String>,
    pub engineer: Option<String>,
    pub project_manager: Option<String>,
    pub superintendent: Option<String>,
    pub location: Option<String>,
    pub start_date: Option<String>,
    pub target_date: Option<String>,
    pub description: Option<String>,
    pub important_notes: Option<String>,
    pub project_path: String,
    pub is_pinned: bool,
    pub archived_at_utc: Option<String>,
    pub created_at_utc: String,
    pub updated_at_utc: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectFolderPreview {
    pub project_path: String,
    pub folders: Vec<String>,
}

pub fn validate_input(input: &ProjectInput) -> AppResult<ProjectInput> {
    let normalized = ProjectInput {
        number: required(&input.number, "Project number")?,
        name: required(&input.name, "Project name")?,
        status: input.status.trim().to_owned(),
        phase: input.phase.trim().to_owned(),
        custom_phase_name: optional(&input.custom_phase_name),
        customer: optional(&input.customer),
        general_contractor: optional(&input.general_contractor),
        engineer: optional(&input.engineer),
        project_manager: optional(&input.project_manager),
        superintendent: optional(&input.superintendent),
        location: optional(&input.location),
        start_date: optional(&input.start_date),
        target_date: optional(&input.target_date),
        description: optional(&input.description),
        important_notes: optional(&input.important_notes),
    };
    if !STATUSES.contains(&normalized.status.as_str()) {
        return Err(invalid(
            "PROJECT_INVALID_STATUS",
            "Choose a valid project status.",
        ));
    }
    if !PHASES.contains(&normalized.phase.as_str()) {
        return Err(invalid(
            "PROJECT_INVALID_PHASE",
            "Choose a valid project phase.",
        ));
    }
    if normalized.phase == "custom" && normalized.custom_phase_name.is_none() {
        return Err(invalid(
            "PROJECT_CUSTOM_PHASE_REQUIRED",
            "Enter a name for the custom project phase.",
        ));
    }
    if normalized.phase != "custom" && normalized.custom_phase_name.is_some() {
        return Err(invalid(
            "PROJECT_CUSTOM_PHASE_UNEXPECTED",
            "Custom phase text is only allowed when Custom is selected.",
        ));
    }
    if has_invalid_filename_character(&normalized.number)
        || has_invalid_filename_character(&normalized.name)
    {
        return Err(invalid(
            "PROJECT_INVALID_FOLDER_NAME",
            "Project number and name cannot contain Windows-invalid folder characters.",
        ));
    }
    Ok(normalized)
}

pub fn preview(root: &Path, input: &ProjectInput) -> AppResult<ProjectFolderPreview> {
    let input = validate_input(input)?;
    let project_path = root.join(format!("{} - {}", input.number, input.name));
    let folders = folder_suffixes();
    Ok(ProjectFolderPreview {
        project_path: project_path.to_string_lossy().to_string(),
        folders: folders
            .iter()
            .map(|suffix| project_path.join(suffix).to_string_lossy().to_string())
            .collect(),
    })
}

pub fn create_folder_tree(root: &Path, input: &ProjectInput) -> AppResult<PathBuf> {
    let preview = preview(root, input)?;
    let path = PathBuf::from(&preview.project_path);
    if path.exists() {
        return Err(AppError::from_technical(
            "PROJECT_PATH_COLLISION",
            "A project folder already exists at the proposed path.",
            "Change the project number or name, or choose the existing project instead.",
            path.display().to_string(),
        ));
    }
    fs::create_dir(&path)
        .map_err(|error| fs_error(error, "The project folder could not be created."))?;
    for folder in folder_suffixes() {
        if let Err(error) = fs::create_dir_all(path.join(folder)) {
            let cleanup = fs::remove_dir_all(&path);
            return Err(AppError::from_technical(
                "FOLDER_CREATION_PARTIAL",
                "The project folder structure could not be completed.",
                if cleanup.is_ok() {
                    "No project record was created. Check the root location and try again."
                } else {
                    "Some folders may remain. Review the project root before retrying."
                },
                error.to_string(),
            ));
        }
    }
    Ok(path)
}

pub fn new_id() -> String {
    Uuid::new_v4().to_string()
}
fn required(value: &str, label: &str) -> AppResult<String> {
    let value = value.trim();
    if value.is_empty() {
        Err(invalid(
            "PROJECT_REQUIRED_FIELD",
            &format!("{} is required.", label),
        ))
    } else {
        Ok(value.to_owned())
    }
}
fn optional(value: &Option<String>) -> Option<String> {
    value
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}
fn invalid(code: &'static str, message: &str) -> AppError {
    AppError::from_technical(
        code,
        message,
        "Correct the highlighted project information and try again.",
        message,
    )
}
fn has_invalid_filename_character(value: &str) -> bool {
    value.chars().any(|character| {
        matches!(
            character,
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*'
        )
    }) || value.ends_with('.')
        || value.ends_with(' ')
}
fn fs_error(error: std::io::Error, message: &str) -> AppError {
    let code = if error.kind() == std::io::ErrorKind::PermissionDenied {
        "PROJECT_ROOT_ACCESS_DENIED"
    } else {
        "PROJECT_FOLDER_CREATE_FAILED"
    };
    AppError::from_technical(
        code,
        message,
        "Check that the project root is available and writable, then try again.",
        error.to_string(),
    )
}
fn folder_suffixes() -> Vec<&'static str> {
    vec![
        "01 Drawings/Controls",
        "01 Drawings/Mechanical",
        "01 Drawings/Electrical",
        "01 Drawings/Architectural",
        "02 Submittals/Created",
        "02 Submittals/Submitted",
        "02 Submittals/Approved",
        "03 RFIs/Open",
        "03 RFIs/Closed",
        "04 Specifications",
        "05 Sequences",
        "06 Field/Photos",
        "06 Field/Startup",
        "06 Field/Commissioning",
        "07 Closeout",
        "99 Archive",
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    fn input() -> ProjectInput {
        ProjectInput {
            number: "1001".into(),
            name: "North Plant".into(),
            status: "active".into(),
            phase: "engineering".into(),
            custom_phase_name: None,
            customer: None,
            general_contractor: None,
            engineer: None,
            project_manager: None,
            superintendent: None,
            location: None,
            start_date: None,
            target_date: None,
            description: None,
            important_notes: None,
        }
    }
    #[test]
    fn custom_phase_requires_name() {
        let mut value = input();
        value.phase = "custom".into();
        assert_eq!(
            validate_input(&value).unwrap_err().code,
            "PROJECT_CUSTOM_PHASE_REQUIRED"
        );
    }
    #[test]
    fn creates_documented_folder_tree_without_overwrite() {
        let root = std::env::temp_dir().join(format!("project-engineer-wp1-{}", Uuid::new_v4()));
        fs::create_dir(&root).unwrap();
        let path = create_folder_tree(&root, &input()).unwrap();
        assert!(path.join("01 Drawings/Controls").is_dir());
        assert_eq!(
            create_folder_tree(&root, &input()).unwrap_err().code,
            "PROJECT_PATH_COLLISION"
        );
        fs::remove_dir_all(root).unwrap();
    }
}

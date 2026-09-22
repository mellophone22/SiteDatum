use crate::error::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
};
use uuid::Uuid;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileInput {
    pub project_id: String,
    pub source_path: String,
    pub operation: String,
    pub discipline: Option<String>,
    pub drawing_number: Option<String>,
    pub title: Option<String>,
    pub revision: Option<String>,
    pub revision_date: Option<String>,
    pub received_date: Option<String>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileMetadataInput {
    pub discipline: Option<String>,
    pub drawing_number: Option<String>,
    pub title: Option<String>,
    pub revision: Option<String>,
    pub revision_date: Option<String>,
    pub received_date: Option<String>,
    pub is_current: bool,
}
#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct FileRecord {
    pub id: String,
    pub project_id: String,
    pub project_number: String,
    pub project_name: String,
    pub file_name: String,
    pub file_path: String,
    pub operation: String,
    pub discipline: Option<String>,
    pub drawing_number: Option<String>,
    pub title: Option<String>,
    pub revision: Option<String>,
    pub revision_date: Option<String>,
    pub received_date: Option<String>,
    pub is_current: bool,
    pub missing: bool,
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
pub fn validate(input: &FileInput) -> AppResult<()> {
    if input.project_id.trim().is_empty() {
        return Err(err("FILE_PROJECT_REQUIRED", "Choose a project."));
    }
    if !["register", "copy", "move"].contains(&input.operation.as_str()) {
        return Err(err(
            "FILE_OPERATION_INVALID",
            "Choose Register, Copy, or Move.",
        ));
    }
    let source = Path::new(&input.source_path);
    if !source.is_file() {
        return Err(err(
            "FILE_SOURCE_UNAVAILABLE",
            "The selected file is unavailable.",
        ));
    }
    Ok(())
}
pub fn destination(
    project_path: &str,
    source: &Path,
    discipline: &Option<String>,
) -> AppResult<PathBuf> {
    let folder = match clean(discipline).as_deref() {
        Some("controls") => "01 Drawings/Controls",
        Some("mechanical") => "01 Drawings/Mechanical",
        Some("electrical") => "01 Drawings/Electrical",
        Some("architectural") => "01 Drawings/Architectural",
        _ => "01 Drawings",
    };
    let name = source
        .file_name()
        .ok_or_else(|| err("FILE_NAME_INVALID", "The selected file has no usable name."))?;
    Ok(Path::new(project_path).join(folder).join(name))
}
pub fn transfer(input: &FileInput, project_path: &str) -> AppResult<String> {
    validate(input)?;
    let source = Path::new(&input.source_path);
    if input.operation == "register" {
        return Ok(source.to_string_lossy().to_string());
    }
    let destination = destination(project_path, source, &input.discipline)?;
    if destination.exists() {
        return Err(AppError::from_technical(
            "FILE_DESTINATION_COLLISION",
            "A file with that name already exists in the project folder.",
            "Rename the source or choose Register; no file was changed.",
            destination.display().to_string(),
        ));
    }
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent).map_err(|e| fs_err(e))?;
    }
    let result = if input.operation == "copy" {
        fs::copy(source, &destination).map(|_| ())
    } else {
        fs::rename(source, &destination)
    };
    result.map_err(|e| fs_err(e))?;
    Ok(destination.to_string_lossy().to_string())
}
fn err(code: &'static str, message: &str) -> AppError {
    AppError::from_technical(
        code,
        message,
        "Correct the file selection and try again.",
        message,
    )
}
fn fs_err(e: std::io::Error) -> AppError {
    AppError::from_technical(
        "FILE_OPERATION_FAILED",
        "The file operation could not be completed.",
        "Check the source, destination, and permissions. The register was not changed.",
        e.to_string(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    fn input(source: &Path, operation: &str) -> FileInput {
        FileInput {
            project_id: "p1".into(),
            source_path: source.to_string_lossy().to_string(),
            operation: operation.into(),
            discipline: Some("controls".into()),
            drawing_number: None,
            title: None,
            revision: None,
            revision_date: None,
            received_date: None,
        }
    }
    #[test]
    fn copy_move_register_and_collision_are_explicit() {
        let root = std::env::temp_dir().join(format!("project-engineer-files-{}", Uuid::new_v4()));
        let project = root.join("project");
        fs::create_dir_all(&project).unwrap();
        let source = root.join("source.pdf");
        fs::write(&source, "content").unwrap();
        let registered = transfer(&input(&source, "register"), &project.to_string_lossy()).unwrap();
        assert_eq!(registered, source.to_string_lossy());
        assert!(source.exists());
        let copied = transfer(&input(&source, "copy"), &project.to_string_lossy()).unwrap();
        assert!(Path::new(&copied).is_file());
        assert!(source.exists());
        assert_eq!(
            transfer(&input(&source, "copy"), &project.to_string_lossy())
                .unwrap_err()
                .code,
            "FILE_DESTINATION_COLLISION"
        );
        let moved_source = root.join("move.pdf");
        fs::write(&moved_source, "content").unwrap();
        let moved = transfer(&input(&moved_source, "move"), &project.to_string_lossy()).unwrap();
        assert!(Path::new(&moved).is_file());
        assert!(!moved_source.exists());
        fs::remove_dir_all(root).unwrap();
    }
}

use crate::error::{AppError, AppResult};
use serde::Serialize;
use std::{fs, path::Path};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectRootSetting {
    pub path: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectRootValidation {
    pub canonical_path: String,
    pub path_kind: PathKind,
    pub warning: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum PathKind {
    Local,
    Unc,
}

pub fn validate_project_root(input: &str) -> AppResult<ProjectRootValidation> {
    let path = input.trim();
    if path.is_empty() {
        return Err(invalid_path("Enter a local or UNC folder path."));
    }
    if path.contains('\0')
        || path
            .chars()
            .any(|character| matches!(character, '<' | '>' | '"' | '|' | '?' | '*'))
    {
        return Err(invalid_path(
            "The path contains characters Windows does not allow.",
        ));
    }
    if path
        .split(['\\', '/'])
        .any(|part| part == "." || part == "..")
    {
        return Err(invalid_path(
            "Use a fully resolved path without . or .. segments.",
        ));
    }
    let is_unc = path.starts_with("\\\\");
    if is_unc {
        let parts: Vec<_> = path
            .trim_start_matches('\\')
            .split(['\\', '/'])
            .filter(|part| !part.is_empty())
            .collect();
        if parts.len() < 2 {
            return Err(invalid_path(
                "A UNC path must include both a server and share name.",
            ));
        }
    } else if !is_windows_absolute_path(path) {
        return Err(invalid_path("Use an absolute local path such as C:\\Projects or a UNC path such as \\server\\share."));
    }
    let candidate = Path::new(path);
    let metadata = fs::symlink_metadata(candidate)
        .map_err(|error| map_fs_error(error, "The project root is unavailable."))?;
    if metadata.file_type().is_symlink() {
        return Err(AppError::from_technical(
            "ROOT_REPARSE_POINT",
            "The project root cannot be a symlink or reparse point.",
            "Choose the real local folder or UNC share instead.",
            path,
        ));
    }
    if !metadata.is_dir() {
        return Err(AppError::from_technical(
            "ROOT_NOT_DIRECTORY",
            "The selected path is not a folder.",
            "Choose an existing local folder or UNC share.",
            path,
        ));
    }
    fs::read_dir(candidate)
        .map_err(|error| map_fs_error(error, "The project root cannot be read."))?;
    let canonical_path = fs::canonicalize(candidate)
        .map_err(|error| map_fs_error(error, "The project root could not be resolved."))?;
    Ok(ProjectRootValidation { canonical_path: canonical_path.to_string_lossy().to_string(), path_kind: if is_unc { PathKind::Unc } else { PathKind::Local }, warning: (path.len() >= 248).then(|| "This path is long. File operations will be checked carefully for Windows path-length compatibility.".to_owned()) })
}

fn is_windows_absolute_path(path: &str) -> bool {
    let bytes = path.as_bytes();
    bytes.len() >= 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && matches!(bytes[2], b'\\' | b'/')
}
fn invalid_path(message: &str) -> AppError {
    AppError::from_technical(
        "ROOT_INVALID_PATH",
        message,
        "Enter an existing absolute local folder or UNC share.",
        message,
    )
}
fn map_fs_error(error: std::io::Error, message: &str) -> AppError {
    let code = match error.kind() {
        std::io::ErrorKind::PermissionDenied => "ROOT_ACCESS_DENIED",
        std::io::ErrorKind::NotFound => "ROOT_UNAVAILABLE",
        _ => "ROOT_UNAVAILABLE",
    };
    AppError::from_technical(
        code,
        message,
        "Check that the location is available and that you have permission to use it.",
        error.to_string(),
    )
}

#[cfg(test)]
mod tests {
    use super::{is_windows_absolute_path, map_fs_error, validate_project_root};

    #[test]
    fn recognizes_absolute_drive_paths() {
        assert!(is_windows_absolute_path("C:\\Projects"));
        assert!(!is_windows_absolute_path("Projects"));
        assert!(!is_windows_absolute_path("C:Projects"));
    }

    #[test]
    fn rejects_relative_and_invalid_paths_before_filesystem_access() {
        assert_eq!(
            validate_project_root("Projects").unwrap_err().code,
            "ROOT_INVALID_PATH"
        );
        assert_eq!(
            validate_project_root("C:\\Project?Root").unwrap_err().code,
            "ROOT_INVALID_PATH"
        );
    }

    #[test]
    fn reports_an_unavailable_absolute_path() {
        assert_eq!(
            validate_project_root("C:\\__project_engineer_workspace_missing__")
                .unwrap_err()
                .code,
            "ROOT_UNAVAILABLE"
        );
    }

    #[test]
    fn accepts_an_existing_local_directory() {
        let directory =
            std::env::temp_dir().join(format!("project-engineer-root-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir(&directory).unwrap();
        let validation = validate_project_root(&directory.to_string_lossy()).unwrap();
        assert!(matches!(validation.path_kind, super::PathKind::Local));
        std::fs::remove_dir(directory).unwrap();
    }

    #[test]
    fn maps_permission_denial_to_a_recoverable_error() {
        let error = map_fs_error(
            std::io::Error::from(std::io::ErrorKind::PermissionDenied),
            "The project root cannot be read.",
        );
        assert_eq!(error.code, "ROOT_ACCESS_DENIED");
    }
}

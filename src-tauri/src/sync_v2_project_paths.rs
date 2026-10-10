//! Local-only portable reference projection. Never creates/moves document files.
#![allow(dead_code)]
use crate::{
    error::{AppError, AppResult},
    persistence::Database,
};
use serde_json::Value;
use std::{
    fs,
    path::{Path, PathBuf},
};

pub(crate) fn refused() -> AppError {
    AppError::from_technical(
        "SYNC_V2_PROJECT_PATH_REFUSED",
        "A project reference could not be mapped safely on this computer.",
        "Keep the local project root and review the project reference.",
        "Local root, component, collision, or reparse-point validation failed.",
    )
}
fn reparse(metadata: &fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        metadata.file_attributes() & 0x400 != 0
    }
    #[cfg(not(windows))]
    {
        metadata.file_type().is_symlink()
    }
}
pub(crate) fn selected_root(db: &Database) -> AppResult<PathBuf> {
    let input = db
        .get_project_root()
        .map_err(|_| refused())?
        .path
        .ok_or_else(refused)?;
    let original = Path::new(&input);
    let metadata = fs::symlink_metadata(original).map_err(|_| refused())?;
    if !metadata.is_dir() || reparse(&metadata) {
        return Err(refused());
    }
    #[cfg(windows)]
    {
        let validated =
            crate::project_root::validate_project_root(&input).map_err(|_| refused())?;
        Ok(PathBuf::from(validated.canonical_path))
    }
    #[cfg(not(windows))]
    {
        // Portable CI proof path; the distributed product is Windows-only.
        if !original.is_absolute() {
            return Err(refused());
        }
        fs::canonicalize(original).map_err(|_| refused())
    }
}
pub(crate) fn component(value: &str) -> bool {
    if value.is_empty()
        || value.encode_utf16().count() > 255
        || value == "."
        || value == ".."
        || value.ends_with(['.', ' '])
        || value
            .chars()
            .any(|c| c.is_control() || "\\/:*?\"<>|".contains(c))
    {
        return false;
    }
    let stem = value
        .split('.')
        .next()
        .unwrap_or_default()
        .trim_end_matches(' ')
        .to_ascii_uppercase();
    if ["CON", "PRN", "AUX", "NUL", "CONIN$", "CONOUT$"].contains(&stem.as_str()) {
        return false;
    }
    for prefix in ["COM", "LPT"] {
        if let Some(suffix) = stem.strip_prefix(prefix) {
            if ["1", "2", "3", "4", "5", "6", "7", "8", "9", "¹", "²", "³"].contains(&suffix) {
                return false;
            }
        }
    }
    true
}
pub(crate) fn project_path(root: &Path, reference: &Value) -> AppResult<PathBuf> {
    let object = reference.as_object().ok_or_else(refused)?;
    if object.len() != 2 || object.get("kind").and_then(Value::as_str) != Some("workspace_relative")
    {
        return Err(refused());
    }
    let components = object
        .get("components")
        .and_then(Value::as_array)
        .ok_or_else(refused)?;
    // Never map a project to the shared root itself, or guess external mappings.
    if components.is_empty() || components.len() > 128 {
        return Err(refused());
    }
    let mut path = root.to_path_buf();
    for value in components {
        let value = value
            .as_str()
            .filter(|s| component(s))
            .ok_or_else(refused)?;
        path.push(value);
    }
    if path.to_string_lossy().encode_utf16().count() > 240 {
        return Err(refused());
    }
    verify_local_path(root, &path)?;
    Ok(path)
}
/// External references are deliberately non-filesystem tokens, never guessed
/// destinations. Workspace-relative references inspect metadata, not file bytes.
pub(crate) fn reference_path(root: &Path, reference: &Value, id: uuid::Uuid) -> AppResult<String> {
    let parts = reference["components"].as_array().ok_or_else(refused)?;
    if parts.is_empty() || parts.iter().any(|p| !p.as_str().is_some_and(component)) {
        return Err(refused());
    }
    if reference["kind"] == "external_name" {
        if parts.len() != 1 {
            return Err(refused());
        }
        return Ok(format!("sitedatum-unresolved:{id}"));
    }
    if reference["kind"] != "workspace_relative" {
        return Err(refused());
    }
    let mut path = root.to_path_buf();
    for part in parts {
        path.push(part.as_str().ok_or_else(refused)?);
    }
    verify_reference_path(root, &path)?;
    path.to_str().map(str::to_owned).ok_or_else(refused)
}
pub(crate) fn verify_reference_path(root: &Path, path: &Path) -> AppResult<()> {
    verify_local_path(root, path.parent().ok_or_else(refused)?)?;
    if path.to_string_lossy().encode_utf16().count() > 240 {
        return Err(refused());
    }
    match fs::symlink_metadata(&path) {
        Ok(m) if reparse(&m) || !m.is_file() => return Err(refused()),
        Ok(_) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(_) => return Err(refused()),
    }
    Ok(())
}
pub(crate) fn verify_local_path(root: &Path, path: &Path) -> AppResult<()> {
    let metadata = fs::symlink_metadata(root).map_err(|_| refused())?;
    if !metadata.is_dir() || reparse(&metadata) {
        return Err(refused());
    }
    let relative = path.strip_prefix(root).map_err(|_| refused())?;
    let mut current = root.to_path_buf();
    for segment in relative.components() {
        let std::path::Component::Normal(name) = segment else {
            return Err(refused());
        };
        current.push(name);
        match fs::symlink_metadata(&current) {
            Ok(metadata) if !metadata.is_dir() || reparse(&metadata) => return Err(refused()),
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(_) => return Err(refused()),
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn windows_escape_and_reserved_forms_are_refused() {
        for value in [
            "..",
            ".",
            "C:",
            "a/b",
            "a\\b",
            "NUL.txt",
            "CON",
            "COM1",
            "LPT².log",
            "CONIN$",
            "CONOUT$",
            "CON .txt",
            "ends.",
            "ends ",
            "a\0b",
            "",
        ] {
            assert!(!component(value), "{value:?}");
        }
        assert!(component("Fictional Project 01"));
        assert!(component("CONstruction"));
    }
    #[test]
    fn typed_mapping_stays_under_root_without_creating_any_folder() {
        let root =
            std::env::temp_dir().join(format!("sitedatum-c10-04c3-path-{}", uuid::Uuid::new_v4()));
        fs::create_dir(&root).unwrap();
        let reference = json!({"kind":"workspace_relative","components":["Fictional","Nested"]});
        assert_eq!(
            project_path(&root, &reference).unwrap(),
            root.join("Fictional").join("Nested")
        );
        assert_eq!(fs::read_dir(&root).unwrap().count(), 0);
        for reference in [
            json!({"kind":"external_name","components":["Fictional"]}),
            json!({"kind":"workspace_relative","components":[]}),
            json!({"kind":"workspace_relative","components":["NUL.txt"]}),
            json!({"kind":"workspace_relative","components":[".."]}),
        ] {
            assert!(project_path(&root, &reference).is_err());
        }
        fs::write(root.join("Obstruction"), b"Fictional sentinel").unwrap();
        assert!(project_path(
            &root,
            &json!({"kind":"workspace_relative","components":["Obstruction","Child"]})
        )
        .is_err());
        assert_eq!(
            fs::read(root.join("Obstruction")).unwrap(),
            b"Fictional sentinel"
        );
        fs::remove_file(root.join("Obstruction")).unwrap();
        fs::remove_dir(root).unwrap();
    }
    #[test]
    fn existing_reparse_child_is_refused_without_touching_target() {
        let root = std::env::temp_dir().join(format!(
            "sitedatum-c10-04c3-reparse-{}",
            uuid::Uuid::new_v4()
        ));
        fs::create_dir(&root).unwrap();
        let target = root.join("target");
        fs::create_dir(&target).unwrap();
        fs::write(target.join("sentinel"), b"Fictional sentinel").unwrap();
        let link = root.join("redirect");
        #[cfg(unix)]
        std::os::unix::fs::symlink(&target, &link).unwrap();
        #[cfg(windows)]
        {
            let output = std::process::Command::new("cmd")
                .args(["/c", "mklink", "/J"])
                .arg(&link)
                .arg(&target)
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "Could not create isolated fictional junction fixture"
            );
        }
        assert!(project_path(
            &root,
            &json!({"kind":"workspace_relative","components":["redirect","Child"]})
        )
        .is_err());
        assert_eq!(
            fs::read(target.join("sentinel")).unwrap(),
            b"Fictional sentinel"
        );
        #[cfg(unix)]
        fs::remove_file(&link).unwrap();
        #[cfg(windows)]
        fs::remove_dir(&link).unwrap();
        fs::remove_file(target.join("sentinel")).unwrap();
        fs::remove_dir(target).unwrap();
        fs::remove_dir(root).unwrap();
    }
}

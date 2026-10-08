mod cloud_auth;
mod cloud_sync;
mod data_exchange;
pub mod entitlement;
mod entitlement_enforcement;
mod error;
mod file_record;
mod licensing;
mod note_contact;
mod persistence;
mod project;
mod project_root;
mod recovery;
mod reports;
mod rfi;
mod rfi_pdf;
mod rfi_pdf_settings;
mod submittal;
mod task;
mod work_item;

use std::{fs, sync::Mutex};

use data_exchange::ImportPreview;
use entitlement::CommercialFeature;
use entitlement_enforcement::{require_feature, require_project_activation, CommercialAccess};
use error::{AppError, AppResult};
use file_record::{FileInput, FileMetadataInput, FileRecord};
use note_contact::{ActivityEvent, Contact, ContactInput, Note, NoteInput};
use persistence::Database;
use project::{Project, ProjectFolderPreview, ProjectInput};
use project_root::{validate_project_root, ProjectRootSetting, ProjectRootValidation};
use rfi::{AttachmentReference, Rfi, RfiInput};
use rfi_pdf_settings::{RfiPdfSettings, RfiTemplateMode};
use submittal::{AttachmentReference as SubmittalAttachmentReference, Submittal, SubmittalInput};
use task::{AttentionSection, Task, TaskInput};
use tauri::Manager;
use work_item::{ProjectTemplate, ProjectTemplateInput, WorkItem, WorkItemInput};

struct AppState {
    database: Mutex<Database>,
    commercial_access: Mutex<CommercialAccess>,
    log_path: std::path::PathBuf,
    database_path: std::path::PathBuf,
    startup_failure: Option<StartupFailure>,
}

#[derive(Clone)]
struct StartupFailure {
    code: String,
    message: String,
    recovery: String,
    correlation_id: String,
    database_path: std::path::PathBuf,
    backup_directory: std::path::PathBuf,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct StartupStatus {
    ready: bool,
    code: Option<String>,
    message: Option<String>,
    recovery: Option<String>,
    correlation_id: Option<String>,
    database_path: String,
    backup_directory: String,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct CloudSyncAvailability {
    available: bool,
    connected: bool,
    email: Option<String>,
}

fn legacy_cloud_sync_is_available(has_local_evidence: bool, has_stored_session: bool) -> bool {
    has_local_evidence || has_stored_session
}

#[tauri::command]
fn get_startup_status(state: tauri::State<'_, AppState>) -> StartupStatus {
    match state.startup_failure.as_ref() {
        Some(failure) => StartupStatus {
            ready: false,
            code: Some(failure.code.clone()),
            message: Some(failure.message.clone()),
            recovery: Some(failure.recovery.clone()),
            correlation_id: Some(failure.correlation_id.clone()),
            database_path: failure.database_path.to_string_lossy().into_owned(),
            backup_directory: failure.backup_directory.to_string_lossy().into_owned(),
        },
        None => StartupStatus {
            ready: true,
            code: None,
            message: None,
            recovery: None,
            correlation_id: None,
            database_path: state.database_path.to_string_lossy().into_owned(),
            backup_directory: state
                .database_path
                .parent()
                .map(|parent| parent.join("backups"))
                .unwrap_or_else(|| std::path::PathBuf::from("backups"))
                .to_string_lossy()
                .into_owned(),
        },
    }
}

#[tauri::command]
fn recover_startup_database(
    path: String,
    state: tauri::State<'_, AppState>,
) -> AppResult<recovery::StartupRecoveryResult> {
    let failure = state.startup_failure.as_ref().ok_or_else(|| {
        AppError::from_technical(
            "STARTUP_RECOVERY_NOT_REQUIRED",
            "The workspace database is already available.",
            "Continue using SiteDatum normally.",
            "No startup failure is active.",
        )
    })?;
    let source = recovery::validate_file(&path)?;
    recovery::replace_unavailable_database(
        &source,
        &failure.database_path,
        &failure.backup_directory,
    )
}

fn legacy_cloud_sync_available(state: &AppState) -> AppResult<bool> {
    let has_local_evidence = state
        .database
        .lock()
        .map_err(|_| AppError::internal("Database state is unavailable."))?
        .has_legacy_cloud_sync_access()?;
    if legacy_cloud_sync_is_available(has_local_evidence, false) {
        return Ok(true);
    }
    let has_stored_session = cloud_auth::has_stored_session()?;
    if !legacy_cloud_sync_is_available(false, has_stored_session) {
        return Ok(false);
    }
    state
        .database
        .lock()
        .map_err(|_| AppError::internal("Database state is unavailable."))?
        .preserve_legacy_cloud_sync_access()?;
    Ok(true)
}

#[cfg(test)]
mod legacy_cloud_sync_access_tests {
    use super::legacy_cloud_sync_is_available;

    #[test]
    fn fresh_installations_are_not_eligible_for_legacy_sync() {
        assert!(!legacy_cloud_sync_is_available(false, false));
    }

    #[test]
    fn prior_history_or_a_saved_session_preserves_legacy_sync() {
        assert!(legacy_cloud_sync_is_available(true, false));
        assert!(legacy_cloud_sync_is_available(false, true));
        assert!(legacy_cloud_sync_is_available(true, true));
    }
}

fn require_legacy_cloud_sync(state: &AppState) -> AppResult<()> {
    if legacy_cloud_sync_available(state)? {
        return Ok(());
    }
    Err(AppError::from_technical(
        "SYNC_DEFERRED",
        "Workspace synchronization is not available on this computer.",
        "Continue using the local workspace. No account or cloud connection is required.",
        "Legacy sync access requires an existing device credential or prior local sync state.",
    ))
}

fn current_commercial_access(state: &AppState) -> AppResult<CommercialAccess> {
    state
        .commercial_access
        .lock()
        .map(|access| *access)
        .map_err(|_| AppError::internal("Entitlement state is unavailable."))
}

fn set_commercial_access(
    state: &AppState,
    entitlement: entitlement::EffectiveEntitlement,
) -> AppResult<()> {
    let mut access = state
        .commercial_access
        .lock()
        .map_err(|_| AppError::internal("Entitlement state is unavailable."))?;
    *access = CommercialAccess::Enforced(entitlement);
    Ok(())
}

#[tauri::command]
fn get_licensing_status(
    state: tauri::State<'_, AppState>,
) -> AppResult<licensing::LicensingStatus> {
    let status = licensing::status()?;
    set_commercial_access(
        &state,
        entitlement::EffectiveEntitlement {
            plan: status.plan,
            freshness: status.freshness,
        },
    )?;
    Ok(status)
}

#[tauri::command]
fn create_licensing_account(email: String, password: String) -> AppResult<String> {
    licensing::create_account(email, password)
}

#[tauri::command]
fn sign_in_licensing(
    email: String,
    password: String,
    state: tauri::State<'_, AppState>,
) -> AppResult<licensing::AccountActionResult> {
    let result = licensing::sign_in(email, password)?;
    set_commercial_access(
        &state,
        entitlement::EffectiveEntitlement {
            plan: result.status.plan,
            freshness: result.status.freshness,
        },
    )?;
    Ok(result)
}

#[tauri::command]
fn refresh_licensing_entitlement(
    state: tauri::State<'_, AppState>,
) -> AppResult<licensing::AccountActionResult> {
    let result = licensing::refresh_entitlement()?;
    set_commercial_access(
        &state,
        entitlement::EffectiveEntitlement {
            plan: result.status.plan,
            freshness: result.status.freshness,
        },
    )?;
    Ok(result)
}

#[tauri::command]
fn get_checkout_url(plan: entitlement::Plan) -> AppResult<String> {
    licensing::checkout_url(plan)
}

#[tauri::command]
fn get_billing_portal_url() -> AppResult<String> {
    licensing::portal_url()
}

#[tauri::command]
fn sign_out_licensing(state: tauri::State<'_, AppState>) -> AppResult<licensing::LicensingStatus> {
    let status = licensing::sign_out()?;
    set_commercial_access(&state, entitlement::EffectiveEntitlement::free())?;
    Ok(status)
}

#[tauri::command]
fn get_project_root(state: tauri::State<'_, AppState>) -> AppResult<ProjectRootSetting> {
    let database = state
        .database
        .lock()
        .map_err(|_| AppError::internal("Database state is unavailable."))?;
    let result = database.get_project_root();
    if let Err(error) = &result {
        log_error(&state.log_path, error);
    }
    result
}

#[tauri::command]
fn validate_project_root_command(
    path: String,
    state: tauri::State<'_, AppState>,
) -> AppResult<ProjectRootValidation> {
    let result = validate_project_root(&path);
    if let Err(error) = &result {
        log_error(&state.log_path, error);
    }
    result
}

#[tauri::command]
fn save_project_root(
    path: String,
    state: tauri::State<'_, AppState>,
) -> AppResult<ProjectRootSetting> {
    let validation = match validate_project_root(&path) {
        Ok(validation) => validation,
        Err(error) => {
            log_error(&state.log_path, &error);
            return Err(error);
        }
    };
    let mut database = state
        .database
        .lock()
        .map_err(|_| AppError::internal("Database state is unavailable."))?;
    let saved = match database.save_project_root(&validation.canonical_path) {
        Ok(saved) => saved,
        Err(error) => {
            log_error(&state.log_path, &error);
            return Err(error);
        }
    };
    append_log(
        &state.log_path,
        "INFO",
        "PROJECT_ROOT_SAVED",
        "Project root setting saved.",
    );
    Ok(saved)
}

#[tauri::command]
fn get_rfi_pdf_settings(state: tauri::State<'_, AppState>) -> AppResult<RfiPdfSettings> {
    state
        .database
        .lock()
        .map_err(|_| AppError::internal("Database state is unavailable."))?
        .get_rfi_pdf_settings()
}

#[tauri::command]
fn save_rfi_pdf_settings(
    settings: RfiPdfSettings,
    state: tauri::State<'_, AppState>,
) -> AppResult<RfiPdfSettings> {
    if settings.template_mode == RfiTemplateMode::Custom {
        require_feature(
            current_commercial_access(&state)?,
            CommercialFeature::ProfessionalReports,
        )?;
    }
    state
        .database
        .lock()
        .map_err(|_| AppError::internal("Database state is unavailable."))?
        .save_rfi_pdf_settings(settings)
}

#[tauri::command]
fn preview_project_folder(
    input: ProjectInput,
    state: tauri::State<'_, AppState>,
) -> AppResult<ProjectFolderPreview> {
    let database = state
        .database
        .lock()
        .map_err(|_| AppError::internal("Database state is unavailable."))?;
    let root = database.get_project_root()?.path.ok_or_else(|| {
        AppError::from_technical(
            "PROJECT_ROOT_NOT_SET",
            "Set a project root before creating a project.",
            "Open Settings and choose an existing project root.",
            "No project root setting.",
        )
    })?;
    project::preview(std::path::Path::new(&root), &input)
}

#[tauri::command]
fn list_projects(
    include_archived: bool,
    state: tauri::State<'_, AppState>,
) -> AppResult<Vec<Project>> {
    state
        .database
        .lock()
        .map_err(|_| AppError::internal("Database state is unavailable."))?
        .list_projects(include_archived)
}

#[tauri::command]
fn create_project(input: ProjectInput, state: tauri::State<'_, AppState>) -> AppResult<Project> {
    let input = project::validate_input(&input)?;
    let commercial_access = current_commercial_access(&state)?;
    let mut database = state
        .database
        .lock()
        .map_err(|_| AppError::internal("Database state is unavailable."))?;
    require_project_activation(commercial_access, database.active_project_count()?)?;
    let root = database.get_project_root()?.path.ok_or_else(|| {
        AppError::from_technical(
            "PROJECT_ROOT_NOT_SET",
            "Set a project root before creating a project.",
            "Open Settings and choose an existing project root.",
            "No project root setting.",
        )
    })?;
    let path = project::create_folder_tree(std::path::Path::new(&root), &input)?;
    let id = project::new_id();
    match database.insert_project(&id, &input, &path.to_string_lossy()) {
        Ok(project) => Ok(project),
        Err(error) => {
            log_error(&state.log_path, &error);
            Err(AppError::from_technical("PROJECT_DATABASE_AFTER_FOLDER_CREATE", "The project folder was created, but the project record could not be saved.", "Do not recreate the folder. Review the root and local log, then recover the folder as a project.", error.to_string()))
        }
    }
}

#[tauri::command]
fn set_project_flag(
    id: String,
    archived: Option<bool>,
    pinned: Option<bool>,
    state: tauri::State<'_, AppState>,
) -> AppResult<Project> {
    let commercial_access = current_commercial_access(&state)?;
    let mut database = state
        .database
        .lock()
        .map_err(|_| AppError::internal("Database state is unavailable."))?;
    if archived == Some(false) && database.is_project_archived(&id)? {
        require_project_activation(commercial_access, database.active_project_count()?)?;
    }
    database.set_project_flag(&id, archived, pinned)
}

#[tauri::command]
fn get_project(id: String, state: tauri::State<'_, AppState>) -> AppResult<Project> {
    state
        .database
        .lock()
        .map_err(|_| AppError::internal("Database state is unavailable."))?
        .get_project(&id)
}

#[tauri::command]
fn update_project(
    id: String,
    input: ProjectInput,
    state: tauri::State<'_, AppState>,
) -> AppResult<Project> {
    let input = project::validate_input(&input)?;
    state
        .database
        .lock()
        .map_err(|_| AppError::internal("Database state is unavailable."))?
        .update_project(&id, &input)
}

#[tauri::command]
fn open_project_folder(id: String, state: tauri::State<'_, AppState>) -> AppResult<()> {
    let project = state
        .database
        .lock()
        .map_err(|_| AppError::internal("Database state is unavailable."))?
        .get_project(&id)?;
    if !std::path::Path::new(&project.project_path).is_dir() {
        return Err(AppError::from_technical(
            "PROJECT_FOLDER_MISSING",
            "The project folder is unavailable.",
            "Check the folder location before trying again.",
            project.project_path,
        ));
    }
    std::process::Command::new("explorer.exe")
        .arg(&project.project_path)
        .spawn()
        .map_err(|error| {
            AppError::from_technical(
                "PROJECT_FOLDER_OPEN_FAILED",
                "The project folder could not be opened.",
                "Try opening the path in Explorer.",
                error.to_string(),
            )
        })?;
    Ok(())
}
#[tauri::command]
fn list_tasks(state: tauri::State<'_, AppState>) -> AppResult<Vec<Task>> {
    state
        .database
        .lock()
        .map_err(|_| AppError::internal("Database state is unavailable."))?
        .list_tasks()
}
#[tauri::command]
fn list_work_items(state: tauri::State<'_, AppState>) -> AppResult<Vec<WorkItem>> {
    state
        .database
        .lock()
        .map_err(|_| AppError::internal("Database state is unavailable."))?
        .list_work_items()
}
#[tauri::command]
fn create_work_item(
    input: WorkItemInput,
    state: tauri::State<'_, AppState>,
) -> AppResult<WorkItem> {
    state
        .database
        .lock()
        .map_err(|_| AppError::internal("Database state is unavailable."))?
        .create_work_item(&input)
}
#[tauri::command]
fn update_work_item(
    id: String,
    input: WorkItemInput,
    state: tauri::State<'_, AppState>,
) -> AppResult<WorkItem> {
    state
        .database
        .lock()
        .map_err(|_| AppError::internal("Database state is unavailable."))?
        .update_work_item(&id, &input)
}
#[tauri::command]
fn bulk_set_work_item_status(
    ids: Vec<String>,
    status: String,
    state: tauri::State<'_, AppState>,
) -> AppResult<usize> {
    require_feature(
        current_commercial_access(&state)?,
        CommercialFeature::BulkOperations,
    )?;
    state
        .database
        .lock()
        .map_err(|_| AppError::internal("Database state is unavailable."))?
        .bulk_set_work_item_status(&ids, &status)
}
#[tauri::command]
fn list_project_templates(state: tauri::State<'_, AppState>) -> AppResult<Vec<ProjectTemplate>> {
    state
        .database
        .lock()
        .map_err(|_| AppError::internal("Database state is unavailable."))?
        .list_project_templates()
}
#[tauri::command]
fn create_project_template(
    input: ProjectTemplateInput,
    state: tauri::State<'_, AppState>,
) -> AppResult<ProjectTemplate> {
    require_feature(
        current_commercial_access(&state)?,
        CommercialFeature::ProjectTemplates,
    )?;
    state
        .database
        .lock()
        .map_err(|_| AppError::internal("Database state is unavailable."))?
        .create_project_template(&input)
}
#[tauri::command]
fn apply_project_template(
    template_id: String,
    project_id: String,
    state: tauri::State<'_, AppState>,
) -> AppResult<usize> {
    require_feature(
        current_commercial_access(&state)?,
        CommercialFeature::ProjectTemplates,
    )?;
    state
        .database
        .lock()
        .map_err(|_| AppError::internal("Database state is unavailable."))?
        .apply_project_template(&template_id, &project_id)
}
#[tauri::command]
fn preview_work_item_import(
    path: String,
    project_id: String,
    item_type: String,
    state: tauri::State<'_, AppState>,
) -> AppResult<ImportPreview> {
    require_feature(
        current_commercial_access(&state)?,
        CommercialFeature::SpreadsheetImport,
    )?;
    state
        .database
        .lock()
        .map_err(|_| AppError::internal("Database state is unavailable."))?
        .preview_work_item_import(&path, &project_id, &item_type)
}
#[tauri::command]
fn import_work_items(
    path: String,
    project_id: String,
    item_type: String,
    state: tauri::State<'_, AppState>,
) -> AppResult<usize> {
    require_feature(
        current_commercial_access(&state)?,
        CommercialFeature::SpreadsheetImport,
    )?;
    state
        .database
        .lock()
        .map_err(|_| AppError::internal("Database state is unavailable."))?
        .import_work_items(&path, &project_id, &item_type)
}
#[tauri::command]
fn export_work_items(
    path: String,
    ids: Vec<String>,
    state: tauri::State<'_, AppState>,
) -> AppResult<usize> {
    if std::path::Path::new(&path)
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("xlsx"))
    {
        require_feature(
            current_commercial_access(&state)?,
            CommercialFeature::SpreadsheetExport,
        )?;
    }
    state
        .database
        .lock()
        .map_err(|_| AppError::internal("Database state is unavailable."))?
        .export_work_items(&path, &ids)
}
#[tauri::command]
fn list_attention(
    today: String,
    through: String,
    state: tauri::State<'_, AppState>,
) -> AppResult<Vec<AttentionSection>> {
    state
        .database
        .lock()
        .map_err(|_| AppError::internal("Database state is unavailable."))?
        .list_attention(&today, &through)
}
#[tauri::command]
fn create_task(input: TaskInput, state: tauri::State<'_, AppState>) -> AppResult<Task> {
    task::validate(&input)?;
    let mut db = state
        .database
        .lock()
        .map_err(|_| AppError::internal("Database state is unavailable."))?;
    db.create_task(&task::id(), &input)
}
#[tauri::command]
fn update_task(id: String, input: TaskInput, state: tauri::State<'_, AppState>) -> AppResult<Task> {
    task::validate(&input)?;
    state
        .database
        .lock()
        .map_err(|_| AppError::internal("Database state is unavailable."))?
        .update_task(&id, &input)
}
#[tauri::command]
fn set_task_status(
    id: String,
    status: String,
    waiting_on: Option<String>,
    follow_up_date: Option<String>,
    state: tauri::State<'_, AppState>,
) -> AppResult<()> {
    state
        .database
        .lock()
        .map_err(|_| AppError::internal("Database state is unavailable."))?
        .set_task_status(
            &id,
            &status,
            waiting_on.as_deref(),
            follow_up_date.as_deref(),
        )
}
#[tauri::command]
fn list_rfis(state: tauri::State<'_, AppState>) -> AppResult<Vec<Rfi>> {
    state
        .database
        .lock()
        .map_err(|_| AppError::internal("Database state is unavailable."))?
        .list_rfis()
}
#[tauri::command]
fn get_rfi(id: String, state: tauri::State<'_, AppState>) -> AppResult<Rfi> {
    state
        .database
        .lock()
        .map_err(|_| AppError::internal("Database state is unavailable."))?
        .get_rfi(&id)
}
#[tauri::command]
fn create_rfi(input: RfiInput, state: tauri::State<'_, AppState>) -> AppResult<Rfi> {
    rfi::validate(&input)?;
    state
        .database
        .lock()
        .map_err(|_| AppError::internal("Database state is unavailable."))?
        .create_rfi(&rfi::new_id(), &input)
}
#[tauri::command]
fn update_rfi(id: String, input: RfiInput, state: tauri::State<'_, AppState>) -> AppResult<Rfi> {
    rfi::validate(&input)?;
    state
        .database
        .lock()
        .map_err(|_| AppError::internal("Database state is unavailable."))?
        .update_rfi(&id, &input)
}
#[tauri::command]
fn get_rfi_pdf_default_path(id: String, state: tauri::State<'_, AppState>) -> AppResult<String> {
    let database = state
        .database
        .lock()
        .map_err(|_| AppError::internal("Database state is unavailable."))?;
    let rfi = database.get_rfi(&id)?;
    let project = database.get_project(&rfi.project_id)?;
    Ok(rfi_pdf::suggested_path(&rfi, &project)
        .to_string_lossy()
        .into_owned())
}
#[tauri::command]
fn export_rfi_pdf(
    id: String,
    output_path: String,
    state: tauri::State<'_, AppState>,
) -> AppResult<String> {
    require_feature(
        current_commercial_access(&state)?,
        CommercialFeature::ProfessionalReports,
    )?;
    let mut database = state
        .database
        .lock()
        .map_err(|_| AppError::internal("Database state is unavailable."))?;
    let rfi = database.get_rfi(&id)?;
    let project = database.get_project(&rfi.project_id)?;
    let settings = database.get_rfi_pdf_settings()?;
    let destination = std::path::Path::new(&output_path);
    rfi_pdf::write(&rfi, &project, &settings, destination)?;
    if let Err(error) = database.add_rfi_attachment(&id, &output_path) {
        return Err(AppError::from_technical(
            "RFI_PDF_REFERENCE_FAILED",
            "The PDF was created, but SiteDatum could not add it to the RFI attachments.",
            "The PDF remains at the selected location. Reference it manually from the RFI if needed.",
            format!("{} | {}", destination.display(), error.technical_detail()),
        ));
    }
    Ok(destination.to_string_lossy().into_owned())
}
#[tauri::command]
fn list_rfi_attention(
    today: String,
    through: String,
    state: tauri::State<'_, AppState>,
) -> AppResult<Vec<Rfi>> {
    state
        .database
        .lock()
        .map_err(|_| AppError::internal("Database state is unavailable."))?
        .list_rfi_attention(&today, &through)
}
#[tauri::command]
fn list_rfi_attachments(
    rfi_id: String,
    state: tauri::State<'_, AppState>,
) -> AppResult<Vec<AttachmentReference>> {
    state
        .database
        .lock()
        .map_err(|_| AppError::internal("Database state is unavailable."))?
        .list_rfi_attachments(&rfi_id)
}
#[tauri::command]
fn add_rfi_attachment(
    rfi_id: String,
    file_path: String,
    state: tauri::State<'_, AppState>,
) -> AppResult<AttachmentReference> {
    state
        .database
        .lock()
        .map_err(|_| AppError::internal("Database state is unavailable."))?
        .add_rfi_attachment(&rfi_id, &file_path)
}
#[tauri::command]
fn remove_rfi_attachment(id: String, state: tauri::State<'_, AppState>) -> AppResult<()> {
    state
        .database
        .lock()
        .map_err(|_| AppError::internal("Database state is unavailable."))?
        .remove_rfi_attachment(&id)
}
#[tauri::command]
fn show_rfi_attachment_in_explorer(id: String, state: tauri::State<'_, AppState>) -> AppResult<()> {
    state
        .database
        .lock()
        .map_err(|_| AppError::internal("Database state is unavailable."))?
        .show_rfi_attachment_in_explorer(&id)
}
#[tauri::command]
fn list_submittals(state: tauri::State<'_, AppState>) -> AppResult<Vec<Submittal>> {
    state
        .database
        .lock()
        .map_err(|_| AppError::internal("Database state is unavailable."))?
        .list_submittals()
}
#[tauri::command]
fn get_submittal(id: String, state: tauri::State<'_, AppState>) -> AppResult<Submittal> {
    state
        .database
        .lock()
        .map_err(|_| AppError::internal("Database state is unavailable."))?
        .get_submittal(&id)
}
#[tauri::command]
fn create_submittal(
    input: SubmittalInput,
    state: tauri::State<'_, AppState>,
) -> AppResult<Submittal> {
    submittal::validate(&input)?;
    state
        .database
        .lock()
        .map_err(|_| AppError::internal("Database state is unavailable."))?
        .create_submittal(&submittal::new_id(), &input)
}
#[tauri::command]
fn update_submittal(
    id: String,
    input: SubmittalInput,
    state: tauri::State<'_, AppState>,
) -> AppResult<Submittal> {
    submittal::validate(&input)?;
    state
        .database
        .lock()
        .map_err(|_| AppError::internal("Database state is unavailable."))?
        .update_submittal(&id, &input)
}
#[tauri::command]
fn list_submittal_attention(state: tauri::State<'_, AppState>) -> AppResult<Vec<Submittal>> {
    state
        .database
        .lock()
        .map_err(|_| AppError::internal("Database state is unavailable."))?
        .list_submittal_attention()
}
#[tauri::command]
fn list_submittal_attachments(
    submittal_id: String,
    state: tauri::State<'_, AppState>,
) -> AppResult<Vec<SubmittalAttachmentReference>> {
    state
        .database
        .lock()
        .map_err(|_| AppError::internal("Database state is unavailable."))?
        .list_submittal_attachments(&submittal_id)
}
#[tauri::command]
fn add_submittal_attachment(
    submittal_id: String,
    file_path: String,
    state: tauri::State<'_, AppState>,
) -> AppResult<SubmittalAttachmentReference> {
    state
        .database
        .lock()
        .map_err(|_| AppError::internal("Database state is unavailable."))?
        .add_submittal_attachment(&submittal_id, &file_path)
}
#[tauri::command]
fn remove_submittal_attachment(id: String, state: tauri::State<'_, AppState>) -> AppResult<()> {
    state
        .database
        .lock()
        .map_err(|_| AppError::internal("Database state is unavailable."))?
        .remove_submittal_attachment(&id)
}
#[tauri::command]
fn list_files(state: tauri::State<'_, AppState>) -> AppResult<Vec<FileRecord>> {
    state
        .database
        .lock()
        .map_err(|_| AppError::internal("Database state is unavailable."))?
        .list_files()
}
#[tauri::command]
fn create_file(input: FileInput, state: tauri::State<'_, AppState>) -> AppResult<FileRecord> {
    state
        .database
        .lock()
        .map_err(|_| AppError::internal("Database state is unavailable."))?
        .create_file(&input)
}
#[tauri::command]
fn remove_file_reference(id: String, state: tauri::State<'_, AppState>) -> AppResult<()> {
    state
        .database
        .lock()
        .map_err(|_| AppError::internal("Database state is unavailable."))?
        .remove_file_reference(&id)
}
#[tauri::command]
fn locate_file(id: String, path: String, state: tauri::State<'_, AppState>) -> AppResult<()> {
    state
        .database
        .lock()
        .map_err(|_| AppError::internal("Database state is unavailable."))?
        .locate_file(&id, &path)
}
#[tauri::command]
fn update_file_metadata(
    id: String,
    input: FileMetadataInput,
    state: tauri::State<'_, AppState>,
) -> AppResult<FileRecord> {
    state
        .database
        .lock()
        .map_err(|_| AppError::internal("Database state is unavailable."))?
        .update_file_metadata(&id, &input)
}
#[tauri::command]
fn show_file_in_explorer(id: String, state: tauri::State<'_, AppState>) -> AppResult<()> {
    state
        .database
        .lock()
        .map_err(|_| AppError::internal("Database state is unavailable."))?
        .show_file_in_explorer(&id)
}
#[tauri::command]
fn list_notes(state: tauri::State<'_, AppState>) -> AppResult<Vec<Note>> {
    state
        .database
        .lock()
        .map_err(|_| AppError::internal("Database state is unavailable."))?
        .list_notes()
}
#[tauri::command]
fn create_note(input: NoteInput, state: tauri::State<'_, AppState>) -> AppResult<Note> {
    state
        .database
        .lock()
        .map_err(|_| AppError::internal("Database state is unavailable."))?
        .create_note(&input)
}
#[tauri::command]
fn update_note(id: String, input: NoteInput, state: tauri::State<'_, AppState>) -> AppResult<Note> {
    state
        .database
        .lock()
        .map_err(|_| AppError::internal("Database state is unavailable."))?
        .update_note(&id, &input)
}
#[tauri::command]
fn delete_note(id: String, state: tauri::State<'_, AppState>) -> AppResult<()> {
    state
        .database
        .lock()
        .map_err(|_| AppError::internal("Database state is unavailable."))?
        .delete_note(&id)
}
#[tauri::command]
fn list_contacts(state: tauri::State<'_, AppState>) -> AppResult<Vec<Contact>> {
    state
        .database
        .lock()
        .map_err(|_| AppError::internal("Database state is unavailable."))?
        .list_contacts()
}
#[tauri::command]
fn create_contact(input: ContactInput, state: tauri::State<'_, AppState>) -> AppResult<Contact> {
    state
        .database
        .lock()
        .map_err(|_| AppError::internal("Database state is unavailable."))?
        .create_contact(&input)
}
#[tauri::command]
fn update_contact(
    id: String,
    input: ContactInput,
    state: tauri::State<'_, AppState>,
) -> AppResult<Contact> {
    state
        .database
        .lock()
        .map_err(|_| AppError::internal("Database state is unavailable."))?
        .update_contact(&id, &input)
}
#[tauri::command]
fn delete_contact(id: String, state: tauri::State<'_, AppState>) -> AppResult<()> {
    state
        .database
        .lock()
        .map_err(|_| AppError::internal("Database state is unavailable."))?
        .delete_contact(&id)
}
#[tauri::command]
fn list_activity(state: tauri::State<'_, AppState>) -> AppResult<Vec<ActivityEvent>> {
    state
        .database
        .lock()
        .map_err(|_| AppError::internal("Database state is unavailable."))?
        .list_activity()
}
#[tauri::command]
fn create_local_backup(state: tauri::State<'_, AppState>) -> AppResult<String> {
    let parent = state
        .database_path
        .parent()
        .ok_or_else(|| AppError::internal("Application storage is unavailable."))?;
    let backups = parent.join("backups");
    fs::create_dir_all(&backups).map_err(|e| {
        AppError::from_technical(
            "BACKUP_DIRECTORY_FAILED",
            "Backup storage could not be prepared.",
            "Check local disk permissions and try again.",
            e.to_string(),
        )
    })?;
    let database = state
        .database
        .lock()
        .map_err(|_| AppError::internal("Database state is unavailable."))?;
    Ok(recovery::create(&database, &backups, "workspace")?.path)
}
#[tauri::command]
fn list_local_backups(state: tauri::State<'_, AppState>) -> AppResult<Vec<recovery::BackupInfo>> {
    let parent = state
        .database_path
        .parent()
        .ok_or_else(|| AppError::internal("Application storage is unavailable."))?;
    recovery::list(&parent.join("backups"))
}
#[tauri::command]
fn preview_local_backup(
    path: String,
    state: tauri::State<'_, AppState>,
) -> AppResult<recovery::BackupPreview> {
    let parent = state
        .database_path
        .parent()
        .ok_or_else(|| AppError::internal("Application storage is unavailable."))?;
    let target = recovery::validate_target(&path, &parent.join("backups"))?;
    recovery::preview(&target)
}
#[tauri::command]
fn restore_local_backup(path: String, state: tauri::State<'_, AppState>) -> AppResult<()> {
    create_sync_safety_backup(&state)?;
    let parent = state
        .database_path
        .parent()
        .ok_or_else(|| AppError::internal("Application storage is unavailable."))?;
    let target = recovery::validate_target(&path, &parent.join("backups"))?;
    let mut database = state
        .database
        .lock()
        .map_err(|_| AppError::internal("Database state is unavailable."))?;
    recovery::restore(&mut database, &target)
}

#[tauri::command]
fn get_backup_settings(state: tauri::State<'_, AppState>) -> AppResult<recovery::BackupSettings> {
    state
        .database
        .lock()
        .map_err(|_| AppError::internal("Database state is unavailable."))?
        .get_backup_settings()
}

fn validate_backup_directory(path: &str) -> AppResult<std::path::PathBuf> {
    let canonical = std::path::Path::new(path).canonicalize().map_err(|error| {
        AppError::from_technical(
            "BACKUP_DIRECTORY_FAILED",
            "The selected backup folder is unavailable.",
            "Choose an existing local folder or connected drive and try again.",
            error.to_string(),
        )
    })?;
    if !canonical.is_dir() {
        return Err(AppError::from_technical(
            "BACKUP_DIRECTORY_FAILED",
            "The selected backup location is not a folder.",
            "Choose an existing folder and try again.",
            canonical.display().to_string(),
        ));
    }
    Ok(canonical)
}

fn validate_external_backup_directory(
    path: &str,
    state: &AppState,
) -> AppResult<std::path::PathBuf> {
    let canonical = validate_backup_directory(path)?;
    let app_storage = state
        .database_path
        .parent()
        .ok_or_else(|| AppError::internal("Application storage is unavailable."))?
        .canonicalize()
        .map_err(|error| AppError::internal(error.to_string()))?;
    if canonical.starts_with(&app_storage) {
        return Err(AppError::from_technical(
            "BACKUP_DESTINATION_NOT_EXTERNAL",
            "Choose a backup folder outside SiteDatum's application storage.",
            "Use another local folder, connected drive, or user-controlled synchronized folder.",
            canonical.display().to_string(),
        ));
    }
    Ok(canonical)
}

#[tauri::command]
fn save_backup_settings(
    mut settings: recovery::BackupSettings,
    state: tauri::State<'_, AppState>,
) -> AppResult<recovery::BackupSettings> {
    if let Some(path) = settings.destination_path.as_deref() {
        settings.destination_path = Some(
            validate_external_backup_directory(path, &state)?
                .to_string_lossy()
                .into_owned(),
        );
    }
    state
        .database
        .lock()
        .map_err(|_| AppError::internal("Database state is unavailable."))?
        .save_backup_settings(settings)
}

#[tauri::command]
fn create_external_backup(
    directory: String,
    state: tauri::State<'_, AppState>,
) -> AppResult<recovery::BackupInfo> {
    let directory = validate_external_backup_directory(&directory, &state)?;
    let database = state
        .database
        .lock()
        .map_err(|_| AppError::internal("Database state is unavailable."))?;
    recovery::create(&database, &directory, "workspace")
}

#[tauri::command]
fn list_configured_backups(
    state: tauri::State<'_, AppState>,
) -> AppResult<Vec<recovery::BackupInfo>> {
    let parent = state
        .database_path
        .parent()
        .ok_or_else(|| AppError::internal("Application storage is unavailable."))?;
    let mut backups = recovery::list(&parent.join("backups"))?;
    let settings = state
        .database
        .lock()
        .map_err(|_| AppError::internal("Database state is unavailable."))?
        .get_backup_settings()?;
    if let Some(directory) = settings.destination_path {
        let directory = std::path::PathBuf::from(directory);
        if directory.is_dir() {
            backups.extend(recovery::list(&directory)?);
        }
    }
    backups
        .sort_by_key(|value| std::cmp::Reverse(value.modified_at_utc.parse::<u64>().unwrap_or(0)));
    backups.dedup_by(|a, b| a.path == b.path);
    Ok(backups)
}

#[tauri::command]
fn preview_backup_file(path: String) -> AppResult<recovery::BackupPreview> {
    let path = recovery::validate_file(&path)?;
    recovery::preview(&path)
}

#[tauri::command]
fn restore_backup_file(path: String, state: tauri::State<'_, AppState>) -> AppResult<()> {
    let target = recovery::validate_file(&path)?;
    create_sync_safety_backup(&state)?;
    let mut database = state
        .database
        .lock()
        .map_err(|_| AppError::internal("Database state is unavailable."))?;
    recovery::restore(&mut database, &target)
}

#[tauri::command]
fn run_scheduled_backup(
    state: tauri::State<'_, AppState>,
) -> AppResult<Option<recovery::BackupInfo>> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|error| AppError::internal(error.to_string()))?
        .as_secs();
    let mut database = state
        .database
        .lock()
        .map_err(|_| AppError::internal("Database state is unavailable."))?;
    let mut settings = database.get_backup_settings()?;
    if !recovery::schedule_due(&settings, now) {
        return Ok(None);
    }
    let directory = validate_external_backup_directory(
        settings.destination_path.as_deref().unwrap_or_default(),
        &state,
    )?;
    let backup = recovery::create(&database, &directory, "workspace-scheduled")?;
    settings.last_success_utc = Some(now);
    database.save_backup_settings(settings)?;
    Ok(Some(backup))
}
#[tauri::command]
fn export_operational_report(
    report_type: String,
    project_id: Option<String>,
    output_path: String,
    state: tauri::State<'_, AppState>,
) -> AppResult<String> {
    require_feature(
        current_commercial_access(&state)?,
        CommercialFeature::ProfessionalReports,
    )?;
    state
        .database
        .lock()
        .map_err(|_| AppError::internal("Database state is unavailable."))?
        .export_operational_report(&report_type, project_id.as_deref(), &output_path)
}

#[tauri::command]
fn sign_in_cloud_with_password(
    email: String,
    password: String,
    state: tauri::State<'_, AppState>,
) -> AppResult<cloud_auth::CloudAuthStatus> {
    require_legacy_cloud_sync(&state)?;
    let result = cloud_auth::sign_in_with_password(email, password);
    if let Err(error) = &result {
        log_error(&state.log_path, error);
    }
    result
}

#[tauri::command]
fn get_cloud_sync_availability(
    state: tauri::State<'_, AppState>,
) -> AppResult<CloudSyncAvailability> {
    if !legacy_cloud_sync_available(&state)? {
        return Ok(CloudSyncAvailability {
            available: false,
            connected: false,
            email: None,
        });
    }
    match cloud_auth::status() {
        Ok(status) => Ok(CloudSyncAvailability {
            available: true,
            connected: status.connected,
            email: status.email,
        }),
        Err(error) => {
            log_error(&state.log_path, &error);
            Ok(CloudSyncAvailability {
                available: true,
                connected: false,
                email: None,
            })
        }
    }
}

#[tauri::command]
fn disconnect_cloud(state: tauri::State<'_, AppState>) -> AppResult<()> {
    require_legacy_cloud_sync(&state)?;
    let result = cloud_auth::disconnect();
    if let Err(error) = &result {
        log_error(&state.log_path, error);
    }
    result
}

#[tauri::command]
fn sync_cloud_workspace(state: tauri::State<'_, AppState>) -> AppResult<cloud_sync::SyncResult> {
    require_legacy_cloud_sync(&state)?;
    create_sync_safety_backup(&state)?;
    let mut database = state
        .database
        .lock()
        .map_err(|_| AppError::internal("Database state is unavailable."))?;
    let result = cloud_sync::sync_now(&mut database);
    if let Err(error) = &result {
        log_error(&state.log_path, error);
    }
    result
}

fn create_sync_safety_backup(state: &AppState) -> AppResult<()> {
    let parent = state
        .database_path
        .parent()
        .ok_or_else(|| AppError::internal("Application storage is unavailable."))?;
    let backups = parent.join("backups");
    fs::create_dir_all(&backups).map_err(|error| {
        AppError::from_technical(
            "SYNC_BACKUP_FAILED",
            "A safety backup could not be created before syncing.",
            "Check local disk space and permissions, then try again.",
            error.to_string(),
        )
    })?;
    let database = state
        .database
        .lock()
        .map_err(|_| AppError::internal("Database state is unavailable."))?;
    recovery::create(&database, &backups, "safety")?;
    Ok(())
}

#[tauri::command]
fn list_cloud_conflicts(
    state: tauri::State<'_, AppState>,
) -> AppResult<Vec<cloud_sync::SyncConflict>> {
    require_legacy_cloud_sync(&state)?;
    state
        .database
        .lock()
        .map_err(|_| AppError::internal("Database state is unavailable."))?
        .list_sync_conflicts()
}

#[tauri::command]
fn resolve_cloud_conflict(
    id: String,
    choice: String,
    state: tauri::State<'_, AppState>,
) -> AppResult<cloud_sync::SyncResult> {
    require_legacy_cloud_sync(&state)?;
    create_sync_safety_backup(&state)?;
    let mut database = state
        .database
        .lock()
        .map_err(|_| AppError::internal("Database state is unavailable."))?;
    let result = cloud_sync::resolve_conflict(&mut database, &id, &choice);
    if let Err(error) = &result {
        log_error(&state.log_path, error);
    }
    result
}

fn append_log(path: &std::path::Path, level: &str, event: &str, detail: &str) {
    use std::io::Write;
    if let Ok(mut file) = fs::OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(file, "{}\t{}\t{}", level, event, detail);
    }
}

fn log_error(path: &std::path::Path, error: &AppError) {
    append_log(
        path,
        "ERROR",
        error.code,
        &format!(
            "{} | {} | {}",
            error.correlation_id,
            error.message,
            error.technical_detail()
        ),
    );
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let local_data = app.path().app_local_data_dir().map_err(|error| {
                AppError::from_technical(
                    "APP_STORAGE_UNAVAILABLE",
                    "Application storage could not be prepared.",
                    "Check available disk space and permissions, then restart the app.",
                    error.to_string(),
                )
            })?;
            fs::create_dir_all(&local_data).map_err(|error| {
                AppError::from_technical(
                    "APP_STORAGE_UNAVAILABLE",
                    "Application storage could not be prepared.",
                    "Check available disk space and permissions, then restart the app.",
                    error.to_string(),
                )
            })?;
            let log_path = local_data.join("workspace.log");
            let database_path = local_data.join("workspace.sqlite3");
            let backup_directory = local_data.join("backups");
            let (database, startup_failure) = match Database::open(&database_path) {
                Ok(database) => {
                    append_log(&log_path, "INFO", "APPLICATION_STARTED", "Application storage and migrations are ready.");
                    (database, None)
                }
                Err(error) => {
                    log_error(&log_path, &error);
                    let failure = StartupFailure {
                        code: error.code.to_owned(),
                        message: error.message.clone(),
                        recovery: "Choose a verified SiteDatum backup below. The unavailable database will be preserved before recovery.".to_owned(),
                        correlation_id: error.correlation_id.clone(),
                        database_path: database_path.clone(),
                        backup_directory,
                    };
                    (Database::open_in_memory()?, Some(failure))
                }
            };
            app.manage(AppState {
                database: Mutex::new(database),
                commercial_access: Mutex::new(CommercialAccess::Enforced(
                    licensing::effective_entitlement(),
                )),
                log_path,
                database_path,
                startup_failure,
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_startup_status,
            recover_startup_database,
            get_project_root,
            validate_project_root_command,
            save_project_root,
            get_rfi_pdf_settings,
            save_rfi_pdf_settings,
            preview_project_folder,
            list_projects,
            create_project,
            set_project_flag,
            get_project,
            update_project,
            open_project_folder,
            list_tasks,
            list_work_items,
            create_work_item,
            update_work_item,
            bulk_set_work_item_status,
            list_project_templates,
            create_project_template,
            apply_project_template,
            preview_work_item_import,
            import_work_items,
            export_work_items,
            list_attention,
            create_task,
            update_task,
            set_task_status,
            list_rfis,
            get_rfi,
            create_rfi,
            update_rfi,
            get_rfi_pdf_default_path,
            export_rfi_pdf,
            list_rfi_attention,
            list_rfi_attachments,
            add_rfi_attachment,
            remove_rfi_attachment,
            show_rfi_attachment_in_explorer,
            list_submittals,
            get_submittal,
            create_submittal,
            update_submittal,
            list_submittal_attention,
            list_submittal_attachments,
            add_submittal_attachment,
            remove_submittal_attachment,
            list_files,
            create_file,
            remove_file_reference,
            locate_file,
            update_file_metadata,
            show_file_in_explorer,
            list_notes,
            create_note,
            update_note,
            delete_note,
            list_contacts,
            create_contact,
            update_contact,
            delete_contact,
            list_activity,
            create_local_backup,
            list_local_backups,
            preview_local_backup,
            restore_local_backup,
            get_backup_settings,
            save_backup_settings,
            create_external_backup,
            list_configured_backups,
            preview_backup_file,
            restore_backup_file,
            run_scheduled_backup,
            export_operational_report,
            sign_in_cloud_with_password,
            get_cloud_sync_availability,
            disconnect_cloud,
            sync_cloud_workspace,
            list_cloud_conflicts,
            resolve_cloud_conflict,
            get_licensing_status,
            create_licensing_account,
            sign_in_licensing,
            refresh_licensing_entitlement,
            get_checkout_url,
            get_billing_portal_url,
            sign_out_licensing
        ])
        .run(tauri::generate_context!())
        .expect("failed to run SiteDatum");
}

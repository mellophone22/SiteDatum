use crate::error::{AppError, AppResult};
use keyring::Entry;
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};

const PROJECT_URL: &str = "https://jblmxjowguphehuozfsg.supabase.co";
const PUBLISHABLE_KEY: &str = "sb_publishable_2ZDEAuL6CSD6SK5omAi-6Q_fkFSTBcU";
const CREDENTIAL_SERVICE: &str = "com.cabre.project-engineer-workspace.anydesk-sync";
const CREDENTIAL_ACCOUNT: &str = "supabase-session";

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CloudAuthStatus {
    pub connected: bool,
    pub email: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AuthResponse {
    access_token: String,
    refresh_token: String,
    user: AuthUser,
}

#[derive(Debug, Deserialize, Serialize)]
struct AuthUser {
    email: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct StoredSession {
    access_token: String,
    refresh_token: String,
    email: String,
}

fn session_entry() -> AppResult<Entry> {
    Entry::new(CREDENTIAL_SERVICE, CREDENTIAL_ACCOUNT).map_err(|error| {
        AppError::from_technical(
            "CLOUD_CREDENTIAL_STORE_UNAVAILABLE",
            "Secure Windows credential storage is unavailable.",
            "Check Windows Credential Manager, then try connecting again.",
            error.to_string(),
        )
    })
}

fn stored_session() -> AppResult<StoredSession> {
    let value = match session_entry()?.get_password() {
        Ok(value) => value,
        Err(keyring::Error::NoEntry) => {
            return Err(AppError::from_technical(
                "CLOUD_SIGN_IN_REQUIRED",
                "Connect this computer to the cloud workspace before syncing.",
                "Open Settings and sign in with your email and password.",
                "No cloud session exists in Windows Credential Manager.",
            ));
        }
        Err(error) => {
            return Err(AppError::from_technical(
                "CLOUD_CREDENTIAL_STORE_UNAVAILABLE",
                "The saved cloud session could not be read securely.",
                "Check Windows Credential Manager, then try again.",
                error.to_string(),
            ));
        }
    };
    serde_json::from_str::<StoredSession>(&value).map_err(|error| {
        AppError::from_technical(
            "CLOUD_CREDENTIAL_INVALID",
            "The saved cloud session is invalid.",
            "Disconnect and sign in again.",
            error.to_string(),
        )
    })
}

fn save_session(session: &StoredSession) -> AppResult<()> {
    let serialized =
        serde_json::to_string(session).map_err(|error| AppError::internal(error.to_string()))?;
    session_entry()?.set_password(&serialized).map_err(|error| {
        AppError::from_technical(
            "CLOUD_CREDENTIAL_SAVE_FAILED",
            "SiteDatum could not save the secure cloud session.",
            "Check Windows Credential Manager, then sign in again.",
            error.to_string(),
        )
    })
}

fn auth_error(code: &'static str, error: reqwest::Error) -> AppError {
    AppError::from_technical(
        code,
        "SiteDatum could not reach the cloud workspace.",
        "Check your connection and try again. Your local workspace is unchanged.",
        error.to_string(),
    )
}

pub fn sign_in_with_password(email: String, password: String) -> AppResult<CloudAuthStatus> {
    let email = email.trim().to_ascii_lowercase();
    if !email.contains('@') || email.starts_with('@') || email.ends_with('@') {
        return Err(AppError::from_technical(
            "CLOUD_EMAIL_INVALID",
            "Enter a valid email address.",
            "Use the confirmed Supabase user created for SiteDatum.",
            email,
        ));
    }
    if password.len() < 8 {
        return Err(AppError::from_technical(
            "CLOUD_PASSWORD_INVALID",
            "Enter your SiteDatum cloud password.",
            "The password must contain at least eight characters.",
            "Password shorter than eight characters.",
        ));
    }

    let response = Client::new()
        .post(format!("{PROJECT_URL}/auth/v1/token?grant_type=password"))
        .header("apikey", PUBLISHABLE_KEY)
        .json(&serde_json::json!({ "email": email, "password": password }))
        .send()
        .map_err(|error| auth_error("CLOUD_SIGN_IN_FAILED", error))?;
    if !response.status().is_success() {
        return Err(AppError::from_technical(
            "CLOUD_SIGN_IN_FAILED",
            "The email or password was not accepted.",
            "Confirm the user in Supabase Authentication, then try again.",
            response.text().unwrap_or_default(),
        ));
    }

    let authenticated: AuthResponse = response.json().map_err(|error| {
        AppError::from_technical(
            "CLOUD_SIGN_IN_FAILED",
            "SiteDatum could not finish cloud sign-in.",
            "Try signing in again. Your local workspace is unchanged.",
            error.to_string(),
        )
    })?;
    let email = authenticated.user.email.unwrap_or(email);
    save_session(&StoredSession {
        access_token: authenticated.access_token,
        refresh_token: authenticated.refresh_token,
        email: email.clone(),
    })?;

    Ok(CloudAuthStatus {
        connected: true,
        email: Some(email),
    })
}

pub fn refreshed_access_token() -> AppResult<String> {
    let session = stored_session()?;
    let response = Client::new()
        .post(format!(
            "{PROJECT_URL}/auth/v1/token?grant_type=refresh_token"
        ))
        .header("apikey", PUBLISHABLE_KEY)
        .json(&serde_json::json!({ "refresh_token": session.refresh_token }))
        .send()
        .map_err(|error| auth_error("CLOUD_SESSION_REFRESH_FAILED", error))?;
    if !response.status().is_success() {
        return Err(AppError::from_technical(
            "CLOUD_SIGN_IN_REQUIRED",
            "Your cloud session expired.",
            "Open Settings, disconnect this computer, and sign in again.",
            response.text().unwrap_or_default(),
        ));
    }

    let refreshed: AuthResponse = response.json().map_err(|error| {
        AppError::from_technical(
            "CLOUD_SESSION_REFRESH_FAILED",
            "SiteDatum could not refresh the cloud session.",
            "Check your connection and try again.",
            error.to_string(),
        )
    })?;
    let refreshed_session = StoredSession {
        access_token: refreshed.access_token,
        refresh_token: refreshed.refresh_token,
        email: refreshed.user.email.unwrap_or(session.email),
    };
    save_session(&refreshed_session)?;
    Ok(refreshed_session.access_token)
}

pub fn status() -> AppResult<CloudAuthStatus> {
    let entry = session_entry()?;
    match entry.get_password() {
        Ok(value) => match serde_json::from_str::<StoredSession>(&value) {
            Ok(session) => Ok(CloudAuthStatus {
                connected: true,
                email: Some(session.email),
            }),
            Err(error) => Err(AppError::from_technical(
                "CLOUD_CREDENTIAL_INVALID",
                "The saved cloud session is invalid.",
                "Disconnect and sign in again.",
                error.to_string(),
            )),
        },
        Err(keyring::Error::NoEntry) => Ok(CloudAuthStatus {
            connected: false,
            email: None,
        }),
        Err(error) => Err(AppError::from_technical(
            "CLOUD_CREDENTIAL_STORE_UNAVAILABLE",
            "Secure Windows credential storage is unavailable.",
            "Check Windows Credential Manager, then try again.",
            error.to_string(),
        )),
    }
}

pub fn disconnect() -> AppResult<()> {
    let entry = session_entry()?;
    match entry.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(error) => Err(AppError::from_technical(
            "CLOUD_CREDENTIAL_DELETE_FAILED",
            "SiteDatum could not disconnect the cloud session.",
            "Try again or remove the SiteDatum cloud credential from Windows Credential Manager.",
            error.to_string(),
        )),
    }
}

#[cfg(all(test, target_os = "windows"))]
mod tests {
    use keyring::credential::CredentialPersistence;

    #[test]
    fn windows_keyring_uses_persistent_native_store() {
        assert!(matches!(
            keyring::default::default_credential_builder().persistence(),
            CredentialPersistence::UntilDelete
        ));
    }
}

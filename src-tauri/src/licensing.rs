use crate::entitlement::{
    evaluate_entitlement, EffectiveEntitlement, EntitlementEvidence, EntitlementFreshness, Plan,
    SubscriptionStatus,
};
use crate::error::{AppError, AppResult};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use keyring::Entry;
use reqwest::blocking::{Client, Response};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

#[cfg(not(feature = "commercial-production"))]
const PROJECT_URL: &str = "https://lirkgkiwbffhsmlrfsbp.supabase.co";
#[cfg(feature = "commercial-production")]
const PROJECT_URL: &str = env!("SITEDATUM_LICENSING_PROJECT_URL");

#[cfg(not(feature = "commercial-production"))]
const PUBLISHABLE_KEY: &str = "sb_publishable_SLXAweYOV-GwggvQp3NnNw_aaKLNs_L";
#[cfg(feature = "commercial-production")]
const PUBLISHABLE_KEY: &str = env!("SITEDATUM_LICENSING_PUBLISHABLE_KEY");

#[cfg(not(feature = "commercial-production"))]
const ENTITLEMENT_PUBLIC_KEY_B64: &str = "EG2rGjHrrOM3gUikZvU3s8PCul9IgRFUXwmgVFQ9NSA=";
#[cfg(feature = "commercial-production")]
const ENTITLEMENT_PUBLIC_KEY_B64: &str = env!("SITEDATUM_ENTITLEMENT_PUBLIC_KEY_B64");

#[cfg(not(feature = "commercial-production"))]
const ENTITLEMENT_KEY_ID: &str = "test-2026-09-30-1";
#[cfg(feature = "commercial-production")]
const ENTITLEMENT_KEY_ID: &str = env!("SITEDATUM_ENTITLEMENT_KEY_ID");
const CREDENTIAL_SERVICE: &str = "com.cabre.project-engineer-workspace.sitedatum-licensing";
const SESSION_ACCOUNT: &str = "supabase-session";
const ENTITLEMENT_ACCOUNT: &str = "signed-entitlement";
const DEVICE_ACCOUNT: &str = "device-identity";

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

#[derive(Debug, Deserialize, Serialize)]
struct StoredSession {
    access_token: String,
    refresh_token: String,
    email: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct EntitlementResponse {
    token: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct EntitlementClaims {
    schema_version: u8,
    issuer: String,
    key_id: String,
    #[serde(rename = "subjectId")]
    _subject_id: Uuid,
    device_id: Uuid,
    plan: Plan,
    subscription_status: SubscriptionStatus,
    issued_at_utc: i64,
    refresh_after_utc: i64,
    paid_through_utc: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LicensingStatus {
    pub connected: bool,
    pub email: Option<String>,
    pub plan: Plan,
    pub subscription_status: Option<SubscriptionStatus>,
    pub freshness: EntitlementFreshness,
    pub paid_through_utc: Option<i64>,
    pub refresh_after_utc: Option<i64>,
    pub device_id: Option<Uuid>,
    pub active_project_limit: Option<usize>,
    pub is_pro: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountActionResult {
    pub status: LicensingStatus,
    pub message: String,
}

#[derive(Debug, Deserialize)]
struct HostedUrlResponse {
    url: String,
}

#[derive(Debug, Deserialize)]
struct ServiceErrorResponse {
    code: Option<String>,
}

fn service_error_code(body: &str) -> Option<String> {
    serde_json::from_str::<ServiceErrorResponse>(body)
        .ok()
        .and_then(|response| response.code)
}

fn now_utc() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

fn entry(account: &str) -> AppResult<Entry> {
    Entry::new(CREDENTIAL_SERVICE, account).map_err(|error| {
        AppError::from_technical(
            "LICENSING_CREDENTIAL_STORE_UNAVAILABLE",
            "Secure Windows credential storage is unavailable.",
            "Check Windows Credential Manager, then try again.",
            error.to_string(),
        )
    })
}

fn read_credential(account: &str) -> AppResult<Option<String>> {
    match entry(account)?.get_password() {
        Ok(value) => Ok(Some(value)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(error) => Err(AppError::from_technical(
            "LICENSING_CREDENTIAL_STORE_UNAVAILABLE",
            "SiteDatum could not read the saved subscription credential.",
            "Check Windows Credential Manager, then try again.",
            error.to_string(),
        )),
    }
}

fn write_credential(account: &str, value: &str) -> AppResult<()> {
    entry(account)?.set_password(value).map_err(|error| {
        AppError::from_technical(
            "LICENSING_CREDENTIAL_SAVE_FAILED",
            "SiteDatum could not save the subscription credential securely.",
            "Check Windows Credential Manager, then try again.",
            error.to_string(),
        )
    })
}

fn delete_credential(account: &str) -> AppResult<()> {
    match entry(account)?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(error) => Err(AppError::from_technical(
            "LICENSING_CREDENTIAL_DELETE_FAILED",
            "SiteDatum could not remove the saved subscription credential.",
            "Try again or remove the SiteDatum licensing entries in Windows Credential Manager.",
            error.to_string(),
        )),
    }
}

fn stored_session() -> AppResult<StoredSession> {
    let value = read_credential(SESSION_ACCOUNT)?.ok_or_else(|| {
        AppError::from_technical(
            "LICENSING_SIGN_IN_REQUIRED",
            "Sign in to manage or recover SiteDatum Pro.",
            "Open Settings, then sign in under Account & Subscription.",
            "No licensing session exists in Windows Credential Manager.",
        )
    })?;
    serde_json::from_str(&value).map_err(|error| {
        AppError::from_technical(
            "LICENSING_CREDENTIAL_INVALID",
            "The saved subscription session is invalid.",
            "Sign out, then sign in again.",
            error.to_string(),
        )
    })
}

fn save_session(session: &StoredSession) -> AppResult<()> {
    let value =
        serde_json::to_string(session).map_err(|error| AppError::internal(error.to_string()))?;
    write_credential(SESSION_ACCOUNT, &value)
}

fn auth_request_error(code: &'static str, error: reqwest::Error) -> AppError {
    AppError::from_technical(
        code,
        "SiteDatum could not reach the licensing service.",
        "Check your connection and try again. Your local workspace is unchanged.",
        error.to_string(),
    )
}

fn validate_email(email: &str) -> AppResult<String> {
    let email = email.trim().to_ascii_lowercase();
    if !email.contains('@') || email.starts_with('@') || email.ends_with('@') {
        return Err(AppError::from_technical(
            "LICENSING_EMAIL_INVALID",
            "Enter a valid email address.",
            "Use the email address associated with your SiteDatum purchase.",
            email,
        ));
    }
    Ok(email)
}

fn parse_auth_response(response: Response, fallback_email: String) -> AppResult<StoredSession> {
    if !response.status().is_success() {
        return Err(AppError::from_technical(
            "LICENSING_SIGN_IN_FAILED",
            "The email or password was not accepted.",
            "Confirm your email, check the password, and try again.",
            response.text().unwrap_or_default(),
        ));
    }
    let authenticated: AuthResponse = response.json().map_err(|error| {
        AppError::from_technical(
            "LICENSING_SIGN_IN_FAILED",
            "SiteDatum could not finish signing in.",
            "Try signing in again. Your local workspace is unchanged.",
            error.to_string(),
        )
    })?;
    Ok(StoredSession {
        access_token: authenticated.access_token,
        refresh_token: authenticated.refresh_token,
        email: authenticated.user.email.unwrap_or(fallback_email),
    })
}

pub fn sign_in(email: String, password: String) -> AppResult<AccountActionResult> {
    let email = validate_email(&email)?;
    if password.len() < 12 {
        return Err(AppError::from_technical(
            "LICENSING_PASSWORD_INVALID",
            "Enter your SiteDatum account password.",
            "The password must contain at least twelve characters.",
            "Password shorter than twelve characters.",
        ));
    }
    let response = Client::new()
        .post(format!("{PROJECT_URL}/auth/v1/token?grant_type=password"))
        .header("apikey", PUBLISHABLE_KEY)
        .json(&serde_json::json!({ "email": email, "password": password }))
        .send()
        .map_err(|error| auth_request_error("LICENSING_SIGN_IN_FAILED", error))?;
    let session = parse_auth_response(response, email)?;
    save_session(&session)?;
    Ok(AccountActionResult {
        status: status()?,
        message: "Account connected securely on this computer.".into(),
    })
}

pub fn create_account(email: String, password: String) -> AppResult<String> {
    let email = validate_email(&email)?;
    if password.len() < 12 {
        return Err(AppError::from_technical(
            "LICENSING_PASSWORD_INVALID",
            "Choose a password with at least twelve characters.",
            "Use a unique password, then try again.",
            "Password shorter than twelve characters.",
        ));
    }
    let response = Client::new()
        .post(format!("{PROJECT_URL}/auth/v1/signup"))
        .header("apikey", PUBLISHABLE_KEY)
        .json(&serde_json::json!({ "email": email, "password": password }))
        .send()
        .map_err(|error| auth_request_error("LICENSING_ACCOUNT_CREATE_FAILED", error))?;
    if !response.status().is_success() {
        return Err(AppError::from_technical(
            "LICENSING_ACCOUNT_CREATE_FAILED",
            "SiteDatum could not create the account.",
            "Check the email and password, then try again.",
            response.text().unwrap_or_default(),
        ));
    }
    Ok("Check your email to confirm the account, then return here to sign in.".into())
}

fn refreshed_access_token() -> AppResult<String> {
    let session = stored_session()?;
    let response = Client::new()
        .post(format!(
            "{PROJECT_URL}/auth/v1/token?grant_type=refresh_token"
        ))
        .header("apikey", PUBLISHABLE_KEY)
        .json(&serde_json::json!({ "refresh_token": session.refresh_token }))
        .send()
        .map_err(|error| auth_request_error("LICENSING_SESSION_REFRESH_FAILED", error))?;
    let refreshed = parse_auth_response(response, session.email)?;
    let token = refreshed.access_token.clone();
    save_session(&refreshed)?;
    Ok(token)
}

fn device_fingerprint_hash() -> AppResult<String> {
    let identity = match read_credential(DEVICE_ACCOUNT)? {
        Some(value) => value,
        None => {
            let value = Uuid::new_v4().to_string();
            write_credential(DEVICE_ACCOUNT, &value)?;
            value
        }
    };
    Ok(format!("{:x}", Sha256::digest(identity.as_bytes())))
}

fn call_function<T: for<'de> Deserialize<'de>>(
    function: &str,
    body: serde_json::Value,
    error_code: &'static str,
) -> AppResult<T> {
    let access_token = refreshed_access_token()?;
    let response = Client::new()
        .post(format!("{PROJECT_URL}/functions/v1/{function}"))
        .header("apikey", PUBLISHABLE_KEY)
        .bearer_auth(access_token)
        .json(&body)
        .send()
        .map_err(|error| auth_request_error(error_code, error))?;
    if !response.status().is_success() {
        let technical_detail = response.text().unwrap_or_default();
        if error_code == "ENTITLEMENT_REFRESH_FAILED"
            && service_error_code(&technical_detail).as_deref() == Some("PRO_SUBSCRIPTION_REQUIRED")
        {
            return Err(AppError::from_technical(
                "ENTITLEMENT_PRO_NOT_FOUND",
                "No SiteDatum Pro subscription was found for this account.",
                "SiteDatum Free remains available. Choose a Pro plan only when you are ready to subscribe.",
                technical_detail,
            ));
        }
        return Err(AppError::from_technical(
            error_code,
            "The licensing service could not complete this request.",
            "Try again. If the problem continues, use the billing portal or contact SiteDatum support.",
            technical_detail,
        ));
    }
    response.json().map_err(|error| {
        AppError::from_technical(
            error_code,
            "The licensing service returned an invalid response.",
            "Try again. Your local workspace is unchanged.",
            error.to_string(),
        )
    })
}

pub fn refresh_entitlement() -> AppResult<AccountActionResult> {
    let response: EntitlementResponse = match call_function(
        "licensing-entitlement",
        serde_json::json!({ "deviceFingerprintHash": device_fingerprint_hash()? }),
        "ENTITLEMENT_REFRESH_FAILED",
    ) {
        Ok(response) => response,
        Err(error) if error.code == "ENTITLEMENT_PRO_NOT_FOUND" => {
            return Ok(AccountActionResult {
                status: status()?,
                message: "No Pro subscription is linked to this account. SiteDatum Free remains available.".into(),
            });
        }
        Err(error) => return Err(error),
    };
    verify_token(&response.token)?;
    write_credential(ENTITLEMENT_ACCOUNT, &response.token)?;
    Ok(AccountActionResult {
        status: status()?,
        message: "Subscription status verified and saved securely on this computer.".into(),
    })
}

pub fn checkout_url(plan: Plan) -> AppResult<String> {
    if !matches!(plan, Plan::ProMonthly | Plan::ProAnnual) {
        return Err(AppError::from_technical(
            "CHECKOUT_PLAN_INVALID",
            "Choose a monthly or annual Pro plan.",
            "Select one of the available Pro plans and try again.",
            format!("invalid checkout plan: {plan:?}"),
        ));
    }
    let response: HostedUrlResponse = call_function(
        "stripe-checkout",
        serde_json::json!({ "plan": plan }),
        "CHECKOUT_UNAVAILABLE",
    )?;
    validate_hosted_url(response.url, "checkout.stripe.com")
}

pub fn portal_url() -> AppResult<String> {
    let response: HostedUrlResponse = call_function(
        "stripe-portal",
        serde_json::json!({}),
        "BILLING_PORTAL_UNAVAILABLE",
    )?;
    validate_hosted_url(response.url, "billing.stripe.com")
}

fn validate_hosted_url(value: String, expected_host: &str) -> AppResult<String> {
    let parsed =
        reqwest::Url::parse(&value).map_err(|error| AppError::internal(error.to_string()))?;
    if parsed.scheme() != "https" || parsed.host_str() != Some(expected_host) {
        return Err(AppError::from_technical(
            "HOSTED_BILLING_URL_INVALID",
            "SiteDatum refused an unexpected billing address.",
            "Try again. If the problem continues, contact SiteDatum support.",
            value,
        ));
    }
    Ok(parsed.to_string())
}

fn verify_token(token: &str) -> AppResult<EntitlementClaims> {
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 || parts[0] != "sd1" {
        return Err(invalid_entitlement("Token shape is invalid."));
    }
    let payload = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(parts[1])
        .map_err(|error| invalid_entitlement(error.to_string()))?;
    let signature_bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(parts[2])
        .map_err(|error| invalid_entitlement(error.to_string()))?;
    let signature = Signature::from_slice(&signature_bytes)
        .map_err(|error| invalid_entitlement(error.to_string()))?;
    let public_bytes = STANDARD
        .decode(ENTITLEMENT_PUBLIC_KEY_B64)
        .map_err(|error| invalid_entitlement(error.to_string()))?;
    let public_key = VerifyingKey::from_bytes(
        &public_bytes
            .try_into()
            .map_err(|_| invalid_entitlement("Public key length is invalid."))?,
    )
    .map_err(|error| invalid_entitlement(error.to_string()))?;
    public_key
        .verify(format!("sd1.{}", parts[1]).as_bytes(), &signature)
        .map_err(|error| invalid_entitlement(error.to_string()))?;
    let claims: EntitlementClaims =
        serde_json::from_slice(&payload).map_err(|error| invalid_entitlement(error.to_string()))?;
    if claims.schema_version != 1
        || claims.issuer != "sitedatum-licensing"
        || claims.key_id != ENTITLEMENT_KEY_ID
        || claims.issued_at_utc > claims.refresh_after_utc
        || claims.refresh_after_utc > claims.paid_through_utc
    {
        return Err(invalid_entitlement("Entitlement claims failed validation."));
    }
    Ok(claims)
}

fn invalid_entitlement(technical: impl Into<String>) -> AppError {
    AppError::from_technical(
        "ENTITLEMENT_INVALID",
        "The saved SiteDatum Pro entitlement could not be verified.",
        "Refresh the entitlement while online or sign in again.",
        technical.into(),
    )
}

fn free_status(email: Option<String>) -> LicensingStatus {
    LicensingStatus {
        connected: email.is_some(),
        email,
        plan: Plan::Free,
        subscription_status: None,
        freshness: EntitlementFreshness::Free,
        paid_through_utc: None,
        refresh_after_utc: None,
        device_id: None,
        active_project_limit: Some(3),
        is_pro: false,
    }
}

pub fn status() -> AppResult<LicensingStatus> {
    let email = read_credential(SESSION_ACCOUNT)?
        .and_then(|value| serde_json::from_str::<StoredSession>(&value).ok())
        .map(|session| session.email);
    let Some(token) = read_credential(ENTITLEMENT_ACCOUNT)? else {
        return Ok(free_status(email));
    };
    let claims = match verify_token(&token) {
        Ok(claims) => claims,
        Err(error) if error.code == "ENTITLEMENT_INVALID" => {
            eprintln!(
                "Discarding an unverifiable cached entitlement and continuing with the Free policy: {}",
                error.correlation_id
            );
            delete_credential(ENTITLEMENT_ACCOUNT)?;
            return Ok(free_status(email));
        }
        Err(error) => return Err(error),
    };
    let now = now_utc();
    let entitlement = evaluate_entitlement(
        EntitlementEvidence {
            plan: claims.plan,
            subscription_status: claims.subscription_status,
            paid_through_utc: Some(claims.paid_through_utc),
            last_verified_utc: Some(claims.issued_at_utc),
            verification_available: now <= claims.refresh_after_utc,
        },
        now,
    );
    Ok(LicensingStatus {
        connected: email.is_some(),
        email,
        plan: entitlement.plan,
        subscription_status: Some(claims.subscription_status),
        freshness: entitlement.freshness,
        paid_through_utc: Some(claims.paid_through_utc),
        refresh_after_utc: Some(claims.refresh_after_utc),
        device_id: Some(claims.device_id),
        active_project_limit: entitlement.active_project_limit(),
        is_pro: entitlement.is_pro(),
    })
}

pub fn effective_entitlement() -> EffectiveEntitlement {
    status()
        .map(|value| EffectiveEntitlement {
            plan: value.plan,
            freshness: value.freshness,
        })
        .unwrap_or_else(|_| EffectiveEntitlement::free())
}

pub fn sign_out() -> AppResult<LicensingStatus> {
    delete_credential(SESSION_ACCOUNT)?;
    delete_credential(ENTITLEMENT_ACCOUNT)?;
    status()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hosted_urls_are_restricted_to_expected_https_hosts() {
        assert!(validate_hosted_url(
            "https://checkout.stripe.com/c/pay/cs_test_123".into(),
            "checkout.stripe.com"
        )
        .is_ok());
        assert!(validate_hosted_url(
            "http://checkout.stripe.com/test".into(),
            "checkout.stripe.com"
        )
        .is_err());
        assert!(
            validate_hosted_url("https://example.com/test".into(), "checkout.stripe.com").is_err()
        );
    }

    #[test]
    fn free_status_keeps_account_optional_and_limits_activation() {
        let anonymous = free_status(None);
        assert!(!anonymous.connected);
        assert_eq!(anonymous.active_project_limit, Some(3));
        assert!(!anonymous.is_pro);
    }

    #[test]
    fn recognizes_a_free_account_without_a_pro_subscription() {
        assert_eq!(
            service_error_code(r#"{"code":"PRO_SUBSCRIPTION_REQUIRED","requestId":"test"}"#)
                .as_deref(),
            Some("PRO_SUBSCRIPTION_REQUIRED")
        );
        assert_eq!(service_error_code("not json"), None);
    }
}

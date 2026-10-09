//! Native C10-03D first-device enrollment boundary.
//!
//! This module is intentionally not exposed as a Tauri command. It owns only a
//! device identifier and Ed25519 private seed in Windows Credential Manager;
//! it never receives project records, paths, documents, or database content.

use std::{io::Read, time::Duration};

use base64::{
    engine::general_purpose::{STANDARD_NO_PAD, URL_SAFE_NO_PAD},
    Engine as _,
};
use ed25519_dalek::{Signer, SigningKey};
use keyring::Entry;
use reqwest::{blocking::Client, header::CONTENT_LENGTH, StatusCode, Url};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use uuid::Uuid;
use zeroize::{Zeroize, Zeroizing};

use crate::error::{AppError, AppResult};

const CREDENTIAL_SERVICE: &str = "com.sitedatum.sync-v2.device-identity";
const CREDENTIAL_ACCOUNT: &str = "device-identity-v1";
const RECORD_SCHEMA: &str = "sitedatum.sync-v2.device-identity.v1";
const PROOF_LABEL: &[u8] = b"sitedatum.sync-v2.initial-device-possession.v1";
const KEY_BYTES: usize = 32;
const CHALLENGE_BYTES: usize = 32;
const MAX_RESPONSE_BYTES: u64 = 8 * 1024;

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case", deny_unknown_fields)]
enum EnrollmentState {
    PendingNew,
    PendingProof {
        owner_id: String,
        auth_session_id: String,
        enrollment_id: String,
        challenge_b64url: String,
        expires_at_unix: u64,
    },
    Active,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredDeviceIdentity {
    schema: String,
    key_version: u32,
    device_id: String,
    private_seed_b64: String,
    enrollment: EnrollmentState,
}

impl Drop for StoredDeviceIdentity {
    fn drop(&mut self) {
        self.private_seed_b64.zeroize();
    }
}

impl StoredDeviceIdentity {
    fn generate() -> AppResult<Self> {
        let mut seed = Zeroizing::new([0_u8; KEY_BYTES]);
        getrandom::fill(seed.as_mut()).map_err(|error| {
            AppError::from_technical(
                "SYNC_V2_DEVICE_KEY_GENERATION_FAILED",
                "SiteDatum could not create this computer's Sync identity.",
                "Try again. Your local workspace is unchanged.",
                format!("Operating-system random generation failed: {error}"),
            )
        })?;
        Ok(Self {
            schema: RECORD_SCHEMA.to_owned(),
            key_version: 1,
            device_id: Uuid::new_v4().to_string(),
            private_seed_b64: STANDARD_NO_PAD.encode(seed.as_ref()),
            enrollment: EnrollmentState::PendingNew,
        })
    }

    fn validate(&self) -> AppResult<()> {
        if self.schema != RECORD_SCHEMA || self.key_version != 1 {
            return Err(credential_invalid(
                "Unsupported device-identity schema or key version.",
            ));
        }
        parse_credential_uuid(&self.device_id, "Stored device identifier is invalid.")?;
        let seed = Zeroizing::new(
            STANDARD_NO_PAD
                .decode(&self.private_seed_b64)
                .map_err(|_| credential_invalid("Stored device key encoding is invalid."))?,
        );
        if seed.len() != KEY_BYTES {
            return Err(credential_invalid("Stored device key length is invalid."));
        }
        if let EnrollmentState::PendingProof {
            owner_id,
            auth_session_id,
            enrollment_id,
            challenge_b64url,
            ..
        } = &self.enrollment
        {
            parse_credential_uuid(owner_id, "Stored owner identifier is invalid.")?;
            parse_credential_uuid(
                auth_session_id,
                "Stored Auth session identifier is invalid.",
            )?;
            parse_credential_uuid(enrollment_id, "Stored enrollment identifier is invalid.")?;
            decode_fixed_base64url::<CHALLENGE_BYTES>(
                challenge_b64url,
                "Stored enrollment challenge is invalid.",
            )?;
        }
        Ok(())
    }

    fn signing_key(&self) -> AppResult<SigningKey> {
        self.validate()?;
        let decoded = Zeroizing::new(
            STANDARD_NO_PAD
                .decode(&self.private_seed_b64)
                .map_err(|_| credential_invalid("Stored device key encoding is invalid."))?,
        );
        let mut seed = Zeroizing::new([0_u8; KEY_BYTES]);
        seed.copy_from_slice(decoded.as_slice());
        Ok(SigningKey::from_bytes(&seed))
    }
}

fn credential_invalid(detail: &'static str) -> AppError {
    AppError::from_technical(
        "SYNC_V2_DEVICE_CREDENTIAL_INVALID",
        "This computer's saved Sync identity is invalid.",
        "Keep Sync disabled and contact SiteDatum support. Your local workspace is unchanged.",
        detail,
    )
}

trait DeviceCredentialStore {
    fn load(&self) -> AppResult<Option<Zeroizing<String>>>;
    fn save(&self, value: &str) -> AppResult<()>;
}

struct WindowsDeviceCredentialStore {
    account: String,
}

impl WindowsDeviceCredentialStore {
    fn production() -> Self {
        Self {
            account: CREDENTIAL_ACCOUNT.to_owned(),
        }
    }

    #[cfg(test)]
    fn fictional(account: String) -> Self {
        Self { account }
    }

    fn entry(&self) -> AppResult<Entry> {
        Entry::new(CREDENTIAL_SERVICE, &self.account).map_err(|error| {
            AppError::from_technical(
                "SYNC_V2_DEVICE_CREDENTIAL_UNAVAILABLE",
                "Secure Windows storage is unavailable for this computer's Sync identity.",
                "Keep Sync disabled and check Windows Credential Manager.",
                error.to_string(),
            )
        })
    }

    #[cfg(test)]
    fn delete(&self) {
        if let Ok(entry) = self.entry() {
            let _ = entry.delete_credential();
        }
    }
}

impl DeviceCredentialStore for WindowsDeviceCredentialStore {
    fn load(&self) -> AppResult<Option<Zeroizing<String>>> {
        match self.entry()?.get_password() {
            Ok(value) => Ok(Some(Zeroizing::new(value))),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(error) => Err(AppError::from_technical(
                "SYNC_V2_DEVICE_CREDENTIAL_UNAVAILABLE",
                "SiteDatum could not read this computer's secure Sync identity.",
                "Keep Sync disabled and check Windows Credential Manager.",
                error.to_string(),
            )),
        }
    }

    fn save(&self, value: &str) -> AppResult<()> {
        self.entry()?.set_password(value).map_err(|error| {
            AppError::from_technical(
                "SYNC_V2_DEVICE_CREDENTIAL_SAVE_FAILED",
                "SiteDatum could not save this computer's secure Sync identity.",
                "Keep Sync disabled and check Windows Credential Manager, then try again.",
                error.to_string(),
            )
        })
    }
}

fn load_identity(store: &impl DeviceCredentialStore) -> AppResult<Option<StoredDeviceIdentity>> {
    let Some(serialized) = store.load()? else {
        return Ok(None);
    };
    let identity: StoredDeviceIdentity = serde_json::from_str(serialized.as_str())
        .map_err(|_| credential_invalid("Stored device-identity JSON is invalid."))?;
    identity.validate()?;
    Ok(Some(identity))
}

fn save_identity(
    store: &impl DeviceCredentialStore,
    identity: &StoredDeviceIdentity,
) -> AppResult<()> {
    identity.validate()?;
    let serialized =
        Zeroizing::new(serde_json::to_string(identity).map_err(|_| {
            AppError::internal("The Sync device identity could not be serialized.")
        })?);
    store.save(serialized.as_str())
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct BeginResponse {
    owner_id: String,
    session_id: String,
    enrollment_id: String,
    challenge: String,
    expires_at_unix: u64,
    request_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CompleteResponse {
    device_id: String,
    request_id: String,
}

trait EnrollmentTransport {
    fn begin(
        &self,
        access_token: &str,
        device_id: Uuid,
        public_key: &[u8; 32],
    ) -> AppResult<BeginResponse>;
    fn complete(
        &self,
        access_token: &str,
        enrollment_id: Uuid,
        signature: &[u8; 64],
    ) -> AppResult<CompleteResponse>;
}

struct HttpEnrollmentTransport {
    endpoint: Url,
    publishable_key: String,
    client: Client,
}

impl HttpEnrollmentTransport {
    #[allow(dead_code)]
    fn new(api_url: &str, publishable_key: String) -> AppResult<Self> {
        let base = Url::parse(api_url).map_err(|_| invalid_service_configuration())?;
        let loopback = matches!(base.host_str(), Some("127.0.0.1" | "localhost" | "::1"));
        if base.username() != ""
            || base.password().is_some()
            || base.query().is_some()
            || base.fragment().is_some()
            || (base.scheme() != "https" && !(cfg!(debug_assertions) && loopback))
            || publishable_key.trim().is_empty()
        {
            return Err(invalid_service_configuration());
        }
        let endpoint = base
            .join("/functions/v1/sync-device-enrollment")
            .map_err(|_| invalid_service_configuration())?;
        let client = Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_secs(15))
            .build()
            .map_err(|_| invalid_service_configuration())?;
        Ok(Self {
            endpoint,
            publishable_key,
            client,
        })
    }

    fn send<T: DeserializeOwned>(&self, access_token: &str, body: &impl Serialize) -> AppResult<T> {
        if access_token.trim().is_empty() {
            return Err(sign_in_required());
        }
        let response = self
            .client
            .post(self.endpoint.clone())
            .header("apikey", &self.publishable_key)
            .bearer_auth(access_token)
            .json(body)
            .send()
            .map_err(|error| transport_unavailable(error.to_string()))?;

        match response.status() {
            status if status.is_success() => {}
            StatusCode::UNAUTHORIZED => return Err(sign_in_required()),
            StatusCode::CONFLICT => {
                return Err(AppError::from_technical(
                    "SYNC_V2_ENROLLMENT_REJECTED",
                    "This computer could not be enrolled for Sync.",
                    "Keep Sync disabled and try again. An existing device requires the approved transfer flow.",
                    "The trusted enrollment bridge refused the request.",
                ));
            }
            StatusCode::TOO_MANY_REQUESTS => {
                return Err(AppError::from_technical(
                    "SYNC_V2_ENROLLMENT_RATE_LIMITED",
                    "Too many Sync enrollment attempts were made.",
                    "Wait before trying again. Your local workspace is unchanged.",
                    "The trusted enrollment bridge rate limit was reached.",
                ));
            }
            _ => {
                return Err(transport_unavailable(
                    "The trusted enrollment bridge returned an unavailable response.",
                ))
            }
        }

        if response
            .headers()
            .get(CONTENT_LENGTH)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.parse::<u64>().ok())
            .is_some_and(|length| length > MAX_RESPONSE_BYTES)
        {
            return Err(transport_unavailable(
                "The trusted enrollment response exceeded its size limit.",
            ));
        }
        let mut bytes = Vec::new();
        response
            .take(MAX_RESPONSE_BYTES + 1)
            .read_to_end(&mut bytes)
            .map_err(|error| transport_unavailable(error.to_string()))?;
        if bytes.len() as u64 > MAX_RESPONSE_BYTES {
            return Err(transport_unavailable(
                "The trusted enrollment response exceeded its size limit.",
            ));
        }
        serde_json::from_slice(&bytes)
            .map_err(|_| transport_unavailable("The trusted enrollment response was invalid."))
    }
}

impl EnrollmentTransport for HttpEnrollmentTransport {
    fn begin(
        &self,
        access_token: &str,
        device_id: Uuid,
        public_key: &[u8; 32],
    ) -> AppResult<BeginResponse> {
        self.send(
            access_token,
            &serde_json::json!({
                "action": "begin",
                "deviceId": device_id.to_string(),
                "publicKey": URL_SAFE_NO_PAD.encode(public_key),
            }),
        )
    }

    fn complete(
        &self,
        access_token: &str,
        enrollment_id: Uuid,
        signature: &[u8; 64],
    ) -> AppResult<CompleteResponse> {
        self.send(
            access_token,
            &serde_json::json!({
                "action": "complete",
                "enrollmentId": enrollment_id.to_string(),
                "signature": URL_SAFE_NO_PAD.encode(signature),
            }),
        )
    }
}

fn invalid_service_configuration() -> AppError {
    AppError::from_technical(
        "SYNC_V2_SERVICE_CONFIGURATION_INVALID",
        "The Sync enrollment service is not configured safely.",
        "Keep Sync disabled and contact SiteDatum support.",
        "The enrollment URL or public client identifier failed local validation.",
    )
}

fn transport_unavailable(detail: impl Into<String>) -> AppError {
    AppError::from_technical(
        "SYNC_V2_ENROLLMENT_UNAVAILABLE",
        "SiteDatum could not finish this computer's Sync enrollment.",
        "Check your connection and try again. Your local workspace is unchanged.",
        detail,
    )
}

fn sign_in_required() -> AppError {
    AppError::from_technical(
        "SYNC_V2_SIGN_IN_REQUIRED",
        "Your Sync sign-in is no longer valid.",
        "Sign in again before enrolling this computer.",
        "The trusted enrollment bridge rejected the user access token.",
    )
}

#[derive(Debug, PartialEq, Eq)]
struct DeviceEnrollmentResult {
    device_id: String,
    already_active: bool,
}

#[allow(dead_code)]
fn enroll_initial_device(
    transport: &impl EnrollmentTransport,
    access_token: &str,
) -> AppResult<DeviceEnrollmentResult> {
    enroll_initial_device_with_store(
        &WindowsDeviceCredentialStore::production(),
        transport,
        access_token,
    )
}

fn enroll_initial_device_with_store(
    store: &impl DeviceCredentialStore,
    transport: &impl EnrollmentTransport,
    access_token: &str,
) -> AppResult<DeviceEnrollmentResult> {
    let mut identity = match load_identity(store)? {
        Some(identity) => identity,
        None => {
            let identity = StoredDeviceIdentity::generate()?;
            save_identity(store, &identity)?;
            identity
        }
    };
    let device_id = parse_uuid(&identity.device_id, "Stored device identifier is invalid.")?;

    if matches!(identity.enrollment, EnrollmentState::Active) {
        return Ok(DeviceEnrollmentResult {
            device_id: device_id.to_string(),
            already_active: true,
        });
    }

    if matches!(identity.enrollment, EnrollmentState::PendingNew) {
        let signing_key = identity.signing_key()?;
        let public_key = signing_key.verifying_key().to_bytes();
        drop(signing_key);
        let begin = transport.begin(access_token, device_id, &public_key)?;
        validate_request_id(&begin.request_id)?;
        parse_uuid(&begin.owner_id, "Enrollment owner identifier is invalid.")?;
        parse_uuid(
            &begin.session_id,
            "Enrollment Auth session identifier is invalid.",
        )?;
        parse_uuid(&begin.enrollment_id, "Enrollment identifier is invalid.")?;
        decode_fixed_base64url::<CHALLENGE_BYTES>(
            &begin.challenge,
            "Enrollment challenge is invalid.",
        )?;
        identity.enrollment = EnrollmentState::PendingProof {
            owner_id: begin.owner_id,
            auth_session_id: begin.session_id,
            enrollment_id: begin.enrollment_id,
            challenge_b64url: begin.challenge,
            expires_at_unix: begin.expires_at_unix,
        };
        save_identity(store, &identity)?;
    }

    let EnrollmentState::PendingProof {
        owner_id,
        auth_session_id,
        enrollment_id,
        challenge_b64url,
        expires_at_unix,
    } = &identity.enrollment
    else {
        return Err(credential_invalid("Stored enrollment state is invalid."));
    };
    let enrollment_uuid = parse_uuid(enrollment_id, "Enrollment identifier is invalid.")?;
    let signing_key = identity.signing_key()?;
    let public_key = signing_key.verifying_key().to_bytes();
    let message = build_proof_message(
        parse_uuid(owner_id, "Enrollment owner identifier is invalid.")?,
        parse_uuid(
            auth_session_id,
            "Enrollment Auth session identifier is invalid.",
        )?,
        device_id,
        enrollment_uuid,
        &public_key,
        &decode_fixed_base64url::<CHALLENGE_BYTES>(
            challenge_b64url,
            "Enrollment challenge is invalid.",
        )?,
        *expires_at_unix,
    );
    let signature = signing_key.sign(&message).to_bytes();
    drop(signing_key);

    let completed = match transport.complete(access_token, enrollment_uuid, &signature) {
        Ok(completed) => completed,
        Err(error) if error.code == "SYNC_V2_ENROLLMENT_REJECTED" => {
            identity.enrollment = EnrollmentState::PendingNew;
            save_identity(store, &identity)?;
            return Err(error);
        }
        Err(error) => return Err(error),
    };
    validate_request_id(&completed.request_id)?;
    let completed_device = parse_uuid(
        &completed.device_id,
        "Completed enrollment device identifier is invalid.",
    )?;
    if completed_device != device_id {
        return Err(transport_unavailable(
            "The trusted enrollment response named an unexpected device.",
        ));
    }

    identity.enrollment = EnrollmentState::Active;
    save_identity(store, &identity)?;
    Ok(DeviceEnrollmentResult {
        device_id: device_id.to_string(),
        already_active: false,
    })
}

fn validate_request_id(value: &str) -> AppResult<()> {
    parse_uuid(value, "Enrollment request identifier is invalid.").map(|_| ())
}

fn parse_uuid(value: &str, detail: &'static str) -> AppResult<Uuid> {
    Uuid::parse_str(value).map_err(|_| transport_unavailable(detail))
}

fn parse_credential_uuid(value: &str, detail: &'static str) -> AppResult<Uuid> {
    Uuid::parse_str(value).map_err(|_| credential_invalid(detail))
}

fn decode_fixed_base64url<const N: usize>(value: &str, detail: &'static str) -> AppResult<[u8; N]> {
    let decoded = URL_SAFE_NO_PAD
        .decode(value)
        .map_err(|_| transport_unavailable(detail))?;
    decoded
        .try_into()
        .map_err(|_| transport_unavailable(detail))
}

fn append_field(output: &mut Vec<u8>, field: &[u8]) {
    output.extend_from_slice(&(field.len() as u32).to_be_bytes());
    output.extend_from_slice(field);
}

fn build_proof_message(
    owner_id: Uuid,
    auth_session_id: Uuid,
    device_id: Uuid,
    enrollment_id: Uuid,
    public_key: &[u8; 32],
    challenge: &[u8; 32],
    expires_at_unix: u64,
) -> Vec<u8> {
    let mut output = Vec::with_capacity(181);
    append_field(&mut output, PROOF_LABEL);
    append_field(&mut output, owner_id.as_bytes());
    append_field(&mut output, auth_session_id.as_bytes());
    append_field(&mut output, device_id.as_bytes());
    append_field(&mut output, enrollment_id.as_bytes());
    append_field(&mut output, public_key);
    append_field(&mut output, challenge);
    output.extend_from_slice(&expires_at_unix.to_be_bytes());
    output
}

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, collections::VecDeque};

    use ed25519_dalek::{Signature, Verifier, VerifyingKey};
    use sha2::Digest;

    use super::*;

    #[derive(Default)]
    struct MemoryStore(RefCell<Option<String>>);

    impl DeviceCredentialStore for MemoryStore {
        fn load(&self) -> AppResult<Option<Zeroizing<String>>> {
            Ok(self.0.borrow().clone().map(Zeroizing::new))
        }

        fn save(&self, value: &str) -> AppResult<()> {
            *self.0.borrow_mut() = Some(value.to_owned());
            Ok(())
        }
    }

    struct FakeTransport {
        owner_id: Uuid,
        session_id: Uuid,
        enrollment_id: Uuid,
        challenge: [u8; 32],
        public_key: RefCell<Option<[u8; 32]>>,
        device_id: RefCell<Option<Uuid>>,
        complete_results: RefCell<VecDeque<Result<(), &'static str>>>,
        begin_calls: RefCell<u32>,
        complete_calls: RefCell<u32>,
    }

    impl FakeTransport {
        fn successful() -> Self {
            Self {
                owner_id: Uuid::parse_str("91000000-0000-4000-8000-000000000001").unwrap(),
                session_id: Uuid::parse_str("92000000-0000-4000-8000-000000000001").unwrap(),
                enrollment_id: Uuid::parse_str("93000000-0000-4000-8000-000000000001").unwrap(),
                challenge: [0x93; 32],
                public_key: RefCell::new(None),
                device_id: RefCell::new(None),
                complete_results: RefCell::new(VecDeque::from([Ok(())])),
                begin_calls: RefCell::new(0),
                complete_calls: RefCell::new(0),
            }
        }
    }

    impl EnrollmentTransport for FakeTransport {
        fn begin(
            &self,
            _access_token: &str,
            device_id: Uuid,
            public_key: &[u8; 32],
        ) -> AppResult<BeginResponse> {
            *self.begin_calls.borrow_mut() += 1;
            *self.public_key.borrow_mut() = Some(*public_key);
            *self.device_id.borrow_mut() = Some(device_id);
            Ok(BeginResponse {
                owner_id: self.owner_id.to_string(),
                session_id: self.session_id.to_string(),
                enrollment_id: self.enrollment_id.to_string(),
                challenge: URL_SAFE_NO_PAD.encode(self.challenge),
                expires_at_unix: 1_800_000_000,
                request_id: Uuid::new_v4().to_string(),
            })
        }

        fn complete(
            &self,
            _access_token: &str,
            enrollment_id: Uuid,
            signature: &[u8; 64],
        ) -> AppResult<CompleteResponse> {
            *self.complete_calls.borrow_mut() += 1;
            assert_eq!(enrollment_id, self.enrollment_id);
            let public_key = self.public_key.borrow().unwrap();
            let device_id = self.device_id.borrow().unwrap();
            let message = build_proof_message(
                self.owner_id,
                self.session_id,
                device_id,
                self.enrollment_id,
                &public_key,
                &self.challenge,
                1_800_000_000,
            );
            VerifyingKey::from_bytes(&public_key)
                .unwrap()
                .verify(&message, &Signature::from_bytes(signature))
                .unwrap();

            match self
                .complete_results
                .borrow_mut()
                .pop_front()
                .unwrap_or(Ok(()))
            {
                Ok(()) => Ok(CompleteResponse {
                    device_id: device_id.to_string(),
                    request_id: Uuid::new_v4().to_string(),
                }),
                Err("unavailable") => Err(transport_unavailable("Simulated response loss.")),
                Err(_) => Err(AppError::from_technical(
                    "SYNC_V2_ENROLLMENT_REJECTED",
                    "This computer could not be enrolled for Sync.",
                    "Keep Sync disabled and try again.",
                    "Simulated explicit rejection.",
                )),
            }
        }
    }

    #[test]
    fn fresh_identity_enrolls_and_then_short_circuits_as_active() {
        let store = MemoryStore::default();
        let transport = FakeTransport::successful();
        let enrolled =
            enroll_initial_device_with_store(&store, &transport, "fictional-token").unwrap();
        assert!(!enrolled.already_active);
        assert_eq!(*transport.begin_calls.borrow(), 1);
        assert_eq!(*transport.complete_calls.borrow(), 1);

        let resumed = enroll_initial_device_with_store(&store, &transport, "").unwrap();
        assert!(resumed.already_active);
        assert_eq!(resumed.device_id, enrolled.device_id);
        assert_eq!(*transport.begin_calls.borrow(), 1);
        assert_eq!(*transport.complete_calls.borrow(), 1);
    }

    #[test]
    fn response_loss_retains_key_and_retries_the_same_proof() {
        let store = MemoryStore::default();
        let mut transport = FakeTransport::successful();
        transport.complete_results = RefCell::new(VecDeque::from([Err("unavailable"), Ok(())]));

        let first =
            enroll_initial_device_with_store(&store, &transport, "fictional-token").unwrap_err();
        assert_eq!(first.code, "SYNC_V2_ENROLLMENT_UNAVAILABLE");
        let pending = load_identity(&store).unwrap().unwrap();
        assert!(matches!(
            pending.enrollment,
            EnrollmentState::PendingProof { .. }
        ));

        let completed =
            enroll_initial_device_with_store(&store, &transport, "fictional-token").unwrap();
        assert!(!completed.already_active);
        assert_eq!(*transport.begin_calls.borrow(), 1);
        assert_eq!(*transport.complete_calls.borrow(), 2);
    }

    #[test]
    fn explicit_rejection_resets_context_but_keeps_the_same_device_key() {
        let store = MemoryStore::default();
        let mut transport = FakeTransport::successful();
        transport.complete_results = RefCell::new(VecDeque::from([Err("rejected")]));
        let error =
            enroll_initial_device_with_store(&store, &transport, "fictional-token").unwrap_err();
        assert_eq!(error.code, "SYNC_V2_ENROLLMENT_REJECTED");

        let identity = load_identity(&store).unwrap().unwrap();
        assert!(matches!(identity.enrollment, EnrollmentState::PendingNew));
        assert_eq!(
            identity.device_id,
            transport.device_id.borrow().unwrap().to_string()
        );
        assert_eq!(
            identity.signing_key().unwrap().verifying_key().to_bytes(),
            transport.public_key.borrow().unwrap()
        );
    }

    #[test]
    fn production_transport_requires_https_and_a_public_identifier() {
        assert!(HttpEnrollmentTransport::new("http://example.com", "public".into()).is_err());
        assert!(HttpEnrollmentTransport::new("https://example.com", "".into()).is_err());
        assert!(HttpEnrollmentTransport::new("https://example.com", "public".into()).is_ok());
    }

    #[test]
    fn canonical_message_matches_the_bridge_vector() {
        let public_key = std::array::from_fn(|index| index as u8);
        let message = build_proof_message(
            Uuid::parse_str("61000000-0000-4000-8000-000000000001").unwrap(),
            Uuid::parse_str("62000000-0000-4000-8000-000000000001").unwrap(),
            Uuid::parse_str("63000000-0000-4000-8000-000000000001").unwrap(),
            Uuid::parse_str("64000000-0000-4000-8000-000000000001").unwrap(),
            &public_key,
            &[0xA5; 32],
            1_800_000_000,
        );
        let digest = sha2::Sha256::digest(message);
        let actual = digest
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        assert_eq!(
            actual,
            "1fb13514a81b5f847ed102855decd6c9d6a49761fbc48b16dcdfb8c5665897c4"
        );
    }

    #[test]
    #[ignore = "writes and deletes one uniquely named fictional Windows credential"]
    fn production_device_store_round_trips_fictional_identity() {
        let store =
            WindowsDeviceCredentialStore::fictional(format!("fictional-{}", Uuid::new_v4()));
        store.delete();
        let identity = StoredDeviceIdentity::generate().unwrap();
        let expected_device = identity.device_id.clone();
        let expected_public = identity.signing_key().unwrap().verifying_key().to_bytes();
        save_identity(&store, &identity).unwrap();
        let reopened = load_identity(&store).unwrap().unwrap();
        assert_eq!(reopened.device_id, expected_device);
        assert_eq!(
            reopened.signing_key().unwrap().verifying_key().to_bytes(),
            expected_public
        );
        store.delete();
        assert!(load_identity(&store).unwrap().is_none());
    }

    #[cfg(target_os = "windows")]
    #[test]
    #[ignore = "uses the disposable local Supabase stack and one fictional Windows credential"]
    fn native_client_enrolls_through_the_local_trusted_bridge() {
        #[derive(Deserialize)]
        struct CreatedUser {
            id: String,
        }
        #[derive(Deserialize)]
        struct SignedIn {
            access_token: String,
        }
        struct Cleanup {
            api_url: String,
            secret_key: String,
            user_id: String,
            store: WindowsDeviceCredentialStore,
        }
        impl Drop for Cleanup {
            fn drop(&mut self) {
                self.store.delete();
                let _ = Client::new()
                    .delete(format!(
                        "{}/auth/v1/admin/users/{}",
                        self.api_url, self.user_id
                    ))
                    .header("apikey", &self.secret_key)
                    .bearer_auth(&self.secret_key)
                    .send();
            }
        }

        let api_url = std::env::var("SYNC_TEST_API_URL").expect("local API URL is required");
        let publishable_key =
            std::env::var("SYNC_TEST_PUBLISHABLE_KEY").expect("local publishable key is required");
        let secret_key =
            std::env::var("SYNC_TEST_SECRET_KEY").expect("local secret key is required");
        let email = format!("native-enrollment-{}@example.invalid", Uuid::new_v4());
        let password = format!("{}Aa1!", Uuid::new_v4());
        let client = Client::new();
        let created_response = client
            .post(format!("{api_url}/auth/v1/admin/users"))
            .header("apikey", &secret_key)
            .bearer_auth(&secret_key)
            .json(&serde_json::json!({
                "email": email,
                "password": password,
                "email_confirm": true,
            }))
            .send()
            .expect("fictional Auth user creation request succeeds");
        assert_eq!(created_response.status(), StatusCode::OK);
        let created: CreatedUser = created_response
            .json()
            .expect("fictional Auth user response is valid");
        let store = WindowsDeviceCredentialStore::fictional(format!(
            "fictional-native-enrollment-{}",
            Uuid::new_v4()
        ));
        store.delete();
        let cleanup = Cleanup {
            api_url: api_url.clone(),
            secret_key: secret_key.clone(),
            user_id: created.id,
            store,
        };

        let sign_in_response = client
            .post(format!("{api_url}/auth/v1/token?grant_type=password"))
            .header("apikey", &publishable_key)
            .json(&serde_json::json!({ "email": email, "password": password }))
            .send()
            .expect("fictional Auth sign-in request succeeds");
        assert_eq!(sign_in_response.status(), StatusCode::OK);
        let signed_in: SignedIn = sign_in_response
            .json()
            .expect("fictional Auth sign-in response is valid");
        let transport = HttpEnrollmentTransport::new(&api_url, publishable_key)
            .expect("local proof endpoint is accepted in a debug test build");

        let enrolled =
            enroll_initial_device_with_store(&cleanup.store, &transport, &signed_in.access_token)
                .expect("native client completes trusted enrollment");
        assert!(!enrolled.already_active);
        let reopened = enroll_initial_device_with_store(&cleanup.store, &transport, "")
            .expect("active local identity reopens without another network request");
        assert!(reopened.already_active);
        assert_eq!(reopened.device_id, enrolled.device_id);
    }
}

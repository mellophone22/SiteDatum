//! Native C10-03F approved-device workspace-key transfer boundary.
//!
//! This module is deliberately not registered as a Tauri command. It owns a
//! Windows-protected X25519 transfer key, versioned canonical messages, and
//! authenticated HPKE sealing/opening for one 32-byte workspace key. It never
//! receives project records, paths, documents, or database content.

#![allow(dead_code)]

use std::{io::Read, time::Duration};

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use ed25519_dalek::{Signer, SigningKey};
use hpke::{
    aead::ChaCha20Poly1305, kdf::HkdfSha256, kem::X25519HkdfSha256, setup_receiver, setup_sender,
    Deserializable, Kem as KemTrait, OpModeR, OpModeS, Serializable,
};
use keyring::Entry;
use reqwest::{blocking::Client, header::CONTENT_LENGTH, StatusCode, Url};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use uuid::Uuid;
use zeroize::{Zeroize, Zeroizing};

use crate::error::{AppError, AppResult};

type Kem = X25519HkdfSha256;
type Kdf = HkdfSha256;
type HpkeAead = ChaCha20Poly1305;

const CREDENTIAL_SERVICE: &str = "com.sitedatum.sync-v2.transfer-identity";
const CREDENTIAL_ACCOUNT: &str = "device-transfer-v1";
const CREDENTIAL_SCHEMA: &str = "sitedatum.sync-v2.transfer-identity.v1";
const TRANSFER_INFO: &[u8] = b"sitedatum.sync-v2.device-transfer.hpke.v1";
const TRANSFER_CONTEXT_LABEL: &[u8] = b"sitedatum.sync-v2.device-transfer.context.v1";
const COMPARISON_LABEL: &[u8] = b"sitedatum.sync-v2.device-transfer.comparison.v1";
const REGISTER_LABEL: &[u8] = b"sitedatum.sync-v2.transfer-key-registration.v1";
const TARGET_LABEL: &[u8] = b"sitedatum.sync-v2.approved-target-possession.v1";
const SOURCE_LABEL: &[u8] = b"sitedatum.sync-v2.approved-source-transfer.v1";
const KEY_BYTES: usize = 32;
const CIPHERTEXT_BYTES: usize = 48;
const MAX_RESPONSE_BYTES: u64 = 16 * 1024;
const COMPARISON_MODULUS: u64 = 1_000_000;

#[derive(Clone, Debug, PartialEq, Eq)]
struct TransferContext {
    owner_id: Uuid,
    workspace_id: Uuid,
    source_device_id: Uuid,
    target_device_id: Uuid,
    enrollment_id: Uuid,
    workspace_key_version: u32,
    expires_at_unix: u64,
}

impl TransferContext {
    fn authenticated_bytes(&self) -> Vec<u8> {
        let mut output = Vec::with_capacity(192);
        append_field(&mut output, TRANSFER_CONTEXT_LABEL);
        append_field(&mut output, self.owner_id.as_bytes());
        append_field(&mut output, self.workspace_id.as_bytes());
        append_field(&mut output, self.source_device_id.as_bytes());
        append_field(&mut output, self.target_device_id.as_bytes());
        append_field(&mut output, self.enrollment_id.as_bytes());
        output.extend_from_slice(&self.workspace_key_version.to_be_bytes());
        output.extend_from_slice(&self.expires_at_unix.to_be_bytes());
        output
    }
}

#[derive(Clone)]
struct ApprovedEnrollmentContext {
    owner_id: Uuid,
    source_session_id: Uuid,
    target_session_id: Uuid,
    source_device_id: Uuid,
    target_device_id: Uuid,
    enrollment_id: Uuid,
    target_proof_public_key: [u8; KEY_BYTES],
    source_transfer_public_key: [u8; KEY_BYTES],
    target_transfer_public_key: [u8; KEY_BYTES],
    challenge: [u8; KEY_BYTES],
    expires_at_unix: u64,
}

struct TransferKeyPair {
    private_key: <Kem as KemTrait>::PrivateKey,
    public_key: <Kem as KemTrait>::PublicKey,
}

impl TransferKeyPair {
    fn generate() -> Self {
        let (private_key, public_key) = Kem::gen_keypair();
        Self {
            private_key,
            public_key,
        }
    }

    fn from_private_bytes(bytes: &[u8; KEY_BYTES]) -> AppResult<Self> {
        let private_key = <Kem as KemTrait>::PrivateKey::from_bytes(bytes)
            .map_err(|_| credential_invalid("Stored X25519 private key is invalid."))?;
        let public_key = Kem::sk_to_pk(&private_key);
        Ok(Self {
            private_key,
            public_key,
        })
    }

    fn public_bytes(&self) -> [u8; KEY_BYTES] {
        array32(self.public_key.to_bytes().as_slice())
    }

    fn private_bytes(&self) -> Zeroizing<[u8; KEY_BYTES]> {
        Zeroizing::new(array32(self.private_key.to_bytes().as_slice()))
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredTransferIdentity {
    schema: String,
    key_version: u32,
    device_id: String,
    private_key_base64url: String,
}

impl Drop for StoredTransferIdentity {
    fn drop(&mut self) {
        self.private_key_base64url.zeroize();
    }
}

impl StoredTransferIdentity {
    fn generate(device_id: Uuid) -> Self {
        let pair = TransferKeyPair::generate();
        let private_key = pair.private_bytes();
        Self {
            schema: CREDENTIAL_SCHEMA.to_owned(),
            key_version: 1,
            device_id: device_id.to_string(),
            private_key_base64url: URL_SAFE_NO_PAD.encode(private_key.as_ref()),
        }
    }

    fn key_pair(&self, expected_device_id: Uuid) -> AppResult<TransferKeyPair> {
        if self.schema != CREDENTIAL_SCHEMA
            || self.key_version != 1
            || Uuid::parse_str(&self.device_id).ok() != Some(expected_device_id)
        {
            return Err(credential_invalid(
                "Stored transfer identity has an unexpected schema or device binding.",
            ));
        }
        let decoded = Zeroizing::new(
            URL_SAFE_NO_PAD
                .decode(&self.private_key_base64url)
                .map_err(|_| credential_invalid("Stored transfer key encoding is invalid."))?,
        );
        let mut private = Zeroizing::new([0_u8; KEY_BYTES]);
        if decoded.len() != KEY_BYTES {
            return Err(credential_invalid("Stored transfer key length is invalid."));
        }
        private.copy_from_slice(decoded.as_slice());
        TransferKeyPair::from_private_bytes(&private)
    }
}

trait TransferCredentialStore {
    fn load(&self) -> AppResult<Option<Zeroizing<String>>>;
    fn save(&self, value: &str) -> AppResult<()>;
}

struct WindowsTransferCredentialStore {
    account: String,
}

impl WindowsTransferCredentialStore {
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
                "SYNC_V2_TRANSFER_CREDENTIAL_UNAVAILABLE",
                "Secure Windows storage is unavailable for this computer's Sync transfer key.",
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

impl TransferCredentialStore for WindowsTransferCredentialStore {
    fn load(&self) -> AppResult<Option<Zeroizing<String>>> {
        match self.entry()?.get_password() {
            Ok(value) => Ok(Some(Zeroizing::new(value))),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(error) => Err(AppError::from_technical(
                "SYNC_V2_TRANSFER_CREDENTIAL_UNAVAILABLE",
                "SiteDatum could not read this computer's protected Sync transfer key.",
                "Keep Sync disabled and check Windows Credential Manager.",
                error.to_string(),
            )),
        }
    }

    fn save(&self, value: &str) -> AppResult<()> {
        self.entry()?.set_password(value).map_err(|error| {
            AppError::from_technical(
                "SYNC_V2_TRANSFER_CREDENTIAL_SAVE_FAILED",
                "SiteDatum could not protect this computer's Sync transfer key.",
                "Keep Sync disabled and check Windows Credential Manager.",
                error.to_string(),
            )
        })
    }
}

#[allow(dead_code)]
fn load_or_create_transfer_key(device_id: Uuid) -> AppResult<TransferKeyPair> {
    load_or_create_transfer_key_with_store(&WindowsTransferCredentialStore::production(), device_id)
}

fn load_or_create_transfer_key_with_store(
    store: &impl TransferCredentialStore,
    device_id: Uuid,
) -> AppResult<TransferKeyPair> {
    if let Some(serialized) = store.load()? {
        let stored: StoredTransferIdentity = serde_json::from_str(serialized.as_str())
            .map_err(|_| credential_invalid("Stored transfer identity JSON is invalid."))?;
        return stored.key_pair(device_id);
    }
    let stored = StoredTransferIdentity::generate(device_id);
    let pair = stored.key_pair(device_id)?;
    let serialized =
        Zeroizing::new(serde_json::to_string(&stored).map_err(|_| {
            AppError::internal("The Sync transfer identity could not be serialized.")
        })?);
    store.save(serialized.as_str())?;
    Ok(pair)
}

struct SealedWorkspaceKey {
    encapsulated_key: [u8; KEY_BYTES],
    ciphertext: [u8; CIPHERTEXT_BYTES],
    comparison_code: String,
}

fn seal_workspace_key(
    context: &TransferContext,
    workspace_key: &[u8; KEY_BYTES],
    source: &TransferKeyPair,
    target_public_key: &[u8; KEY_BYTES],
) -> AppResult<SealedWorkspaceKey> {
    let target_public_key = <Kem as KemTrait>::PublicKey::from_bytes(target_public_key)
        .map_err(|_| transfer_invalid("Target X25519 public key is invalid."))?;
    let (encapsulated, mut sender) = setup_sender::<HpkeAead, Kdf, Kem>(
        &OpModeS::Auth((source.private_key.clone(), source.public_key.clone())),
        &target_public_key,
        TRANSFER_INFO,
    )
    .map_err(|_| transfer_invalid("HPKE sender setup failed."))?;
    let ciphertext = sender
        .seal(workspace_key, &context.authenticated_bytes())
        .map_err(|_| transfer_invalid("Workspace-key sealing failed."))?;
    let ciphertext: [u8; CIPHERTEXT_BYTES] = ciphertext
        .try_into()
        .map_err(|_| transfer_invalid("Sealed workspace-key length is invalid."))?;
    Ok(SealedWorkspaceKey {
        encapsulated_key: array32(encapsulated.to_bytes().as_slice()),
        ciphertext,
        comparison_code: comparison_code(&sender, context)?,
    })
}

fn open_workspace_key(
    context: &TransferContext,
    sealed: &SealedWorkspaceKey,
    source_public_key: &[u8; KEY_BYTES],
    target: &TransferKeyPair,
    owner_entered_code: &str,
    now_unix: u64,
) -> AppResult<Zeroizing<[u8; KEY_BYTES]>> {
    if now_unix >= context.expires_at_unix {
        return Err(transfer_invalid("The approved-device transfer expired."));
    }
    let source_public_key = <Kem as KemTrait>::PublicKey::from_bytes(source_public_key)
        .map_err(|_| transfer_invalid("Source X25519 public key is invalid."))?;
    let encapsulated = <Kem as KemTrait>::EncappedKey::from_bytes(&sealed.encapsulated_key)
        .map_err(|_| transfer_invalid("HPKE encapsulated key is invalid."))?;
    let mut receiver = setup_receiver::<HpkeAead, Kdf, Kem>(
        &OpModeR::Auth(source_public_key),
        &target.private_key,
        &encapsulated,
        TRANSFER_INFO,
    )
    .map_err(|_| transfer_invalid("HPKE receiver setup failed."))?;
    let expected_code = comparison_code(&receiver, context)?;
    if !constant_time_code_eq(owner_entered_code, &expected_code) {
        return Err(transfer_invalid(
            "The device comparison code did not match.",
        ));
    }
    let plaintext = Zeroizing::new(
        receiver
            .open(&sealed.ciphertext, &context.authenticated_bytes())
            .map_err(|_| transfer_invalid("The sealed workspace key did not authenticate."))?,
    );
    if plaintext.len() != KEY_BYTES {
        return Err(transfer_invalid(
            "Recovered workspace-key length is invalid.",
        ));
    }
    let mut result = Zeroizing::new([0_u8; KEY_BYTES]);
    result.copy_from_slice(plaintext.as_slice());
    Ok(result)
}

fn transfer_key_registration_message(
    owner_id: Uuid,
    session_id: Uuid,
    device_id: Uuid,
    transfer_public_key: &[u8; KEY_BYTES],
) -> Vec<u8> {
    let mut output = Vec::with_capacity(128);
    append_field(&mut output, REGISTER_LABEL);
    append_field(&mut output, owner_id.as_bytes());
    append_field(&mut output, session_id.as_bytes());
    append_field(&mut output, device_id.as_bytes());
    append_field(&mut output, transfer_public_key);
    output
}

fn target_proof_message(context: &ApprovedEnrollmentContext) -> Vec<u8> {
    let mut output = Vec::with_capacity(256);
    append_field(&mut output, TARGET_LABEL);
    append_field(&mut output, context.owner_id.as_bytes());
    append_field(&mut output, context.target_session_id.as_bytes());
    append_field(&mut output, context.source_device_id.as_bytes());
    append_field(&mut output, context.target_device_id.as_bytes());
    append_field(&mut output, context.enrollment_id.as_bytes());
    append_field(&mut output, &context.target_proof_public_key);
    append_field(&mut output, &context.target_transfer_public_key);
    append_field(&mut output, &context.challenge);
    output.extend_from_slice(&context.expires_at_unix.to_be_bytes());
    output
}

fn source_approval_message(
    context: &ApprovedEnrollmentContext,
    transfer: &TransferContext,
    sealed: &SealedWorkspaceKey,
) -> AppResult<Vec<u8>> {
    if context.owner_id != transfer.owner_id
        || context.source_device_id != transfer.source_device_id
        || context.target_device_id != transfer.target_device_id
        || context.enrollment_id != transfer.enrollment_id
        || context.expires_at_unix != transfer.expires_at_unix
    {
        return Err(transfer_invalid(
            "Enrollment and workspace-key transfer contexts do not match.",
        ));
    }
    let mut output = Vec::with_capacity(512);
    append_field(&mut output, SOURCE_LABEL);
    append_field(&mut output, context.owner_id.as_bytes());
    append_field(&mut output, context.source_session_id.as_bytes());
    append_field(&mut output, context.target_session_id.as_bytes());
    append_field(&mut output, context.source_device_id.as_bytes());
    append_field(&mut output, context.target_device_id.as_bytes());
    append_field(&mut output, context.enrollment_id.as_bytes());
    append_field(&mut output, &context.target_proof_public_key);
    append_field(&mut output, &context.source_transfer_public_key);
    append_field(&mut output, &context.target_transfer_public_key);
    append_field(&mut output, &context.challenge);
    append_field(&mut output, transfer.workspace_id.as_bytes());
    output.extend_from_slice(&transfer.workspace_key_version.to_be_bytes());
    output.extend_from_slice(&transfer.expires_at_unix.to_be_bytes());
    append_field(&mut output, &sealed.encapsulated_key);
    append_field(&mut output, &sealed.ciphertext);
    Ok(output)
}

fn sign_target_proof(signing_key: &SigningKey, context: &ApprovedEnrollmentContext) -> [u8; 64] {
    signing_key.sign(&target_proof_message(context)).to_bytes()
}

fn sign_source_approval(
    signing_key: &SigningKey,
    context: &ApprovedEnrollmentContext,
    transfer: &TransferContext,
    sealed: &SealedWorkspaceKey,
) -> AppResult<[u8; 64]> {
    Ok(signing_key
        .sign(&source_approval_message(context, transfer, sealed)?)
        .to_bytes())
}

fn append_field(output: &mut Vec<u8>, field: &[u8]) {
    output.extend_from_slice(&(field.len() as u32).to_be_bytes());
    output.extend_from_slice(field);
}

fn array32(bytes: &[u8]) -> [u8; KEY_BYTES] {
    let mut output = [0_u8; KEY_BYTES];
    output.copy_from_slice(bytes);
    output
}

fn comparison_exporter_context(context: &TransferContext) -> Vec<u8> {
    let mut output = Vec::with_capacity(256);
    append_field(&mut output, COMPARISON_LABEL);
    output.extend_from_slice(&context.authenticated_bytes());
    output
}

trait HpkeExporter {
    fn export_secret(&self, context: &[u8], output: &mut [u8]) -> Result<(), hpke::HpkeError>;
}

impl HpkeExporter for hpke::aead::AeadCtxS<HpkeAead, Kdf, Kem> {
    fn export_secret(&self, context: &[u8], output: &mut [u8]) -> Result<(), hpke::HpkeError> {
        self.export(context, output)
    }
}

impl HpkeExporter for hpke::aead::AeadCtxR<HpkeAead, Kdf, Kem> {
    fn export_secret(&self, context: &[u8], output: &mut [u8]) -> Result<(), hpke::HpkeError> {
        self.export(context, output)
    }
}

fn comparison_code<C: HpkeExporter>(context: &C, transfer: &TransferContext) -> AppResult<String> {
    let mut secret = Zeroizing::new([0_u8; KEY_BYTES]);
    context
        .export_secret(&comparison_exporter_context(transfer), secret.as_mut())
        .map_err(|_| transfer_invalid("HPKE comparison export failed."))?;
    let number =
        u64::from_be_bytes(secret[..8].try_into().expect("fixed slice")) % COMPARISON_MODULUS;
    Ok(format!("{number:06}"))
}

fn constant_time_code_eq(left: &str, right: &str) -> bool {
    left.len() == 6
        && right.len() == 6
        && left
            .as_bytes()
            .iter()
            .zip(right.as_bytes())
            .fold(0_u8, |difference, (left, right)| {
                difference | (left ^ right)
            })
            == 0
}

fn credential_invalid(detail: impl Into<String>) -> AppError {
    AppError::from_technical(
        "SYNC_V2_TRANSFER_CREDENTIAL_INVALID",
        "This computer's saved Sync transfer identity is invalid.",
        "Keep Sync disabled and contact SiteDatum support. Your local workspace is unchanged.",
        detail,
    )
}

fn transfer_invalid(detail: impl Into<String>) -> AppError {
    AppError::from_technical(
        "SYNC_V2_DEVICE_TRANSFER_INVALID",
        "The approved-computer key transfer could not be verified.",
        "Keep Sync disabled and start a new device approval. Your local workspace is unchanged.",
        detail,
    )
}

struct HttpApprovedDeviceTransport {
    endpoint: Url,
    publishable_key: String,
    client: Client,
}

impl HttpApprovedDeviceTransport {
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
            .join("/functions/v1/sync-approved-device")
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

    #[allow(dead_code)]
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
                return Err(transfer_invalid("The trusted bridge refused the transfer."))
            }
            StatusCode::TOO_MANY_REQUESTS => {
                return Err(AppError::from_technical(
                    "SYNC_V2_TRANSFER_RATE_LIMITED",
                    "Too many approved-computer transfer attempts were made.",
                    "Wait before trying again. Your local workspace is unchanged.",
                    "The trusted bridge rate limit was reached.",
                ))
            }
            _ => {
                return Err(transport_unavailable(
                    "The trusted bridge returned an unavailable response.",
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
                "The trusted bridge response exceeded its size limit.",
            ));
        }
        let mut bytes = Vec::new();
        response
            .take(MAX_RESPONSE_BYTES + 1)
            .read_to_end(&mut bytes)
            .map_err(|error| transport_unavailable(error.to_string()))?;
        if bytes.len() as u64 > MAX_RESPONSE_BYTES {
            return Err(transport_unavailable(
                "The trusted bridge response exceeded its size limit.",
            ));
        }
        serde_json::from_slice(&bytes)
            .map_err(|_| transport_unavailable("The trusted bridge response was invalid."))
    }
}

fn invalid_service_configuration() -> AppError {
    AppError::from_technical(
        "SYNC_V2_SERVICE_CONFIGURATION_INVALID",
        "The approved-computer service is not configured safely.",
        "Keep Sync disabled and contact SiteDatum support.",
        "The bridge URL or public client identifier failed local validation.",
    )
}

fn transport_unavailable(detail: impl Into<String>) -> AppError {
    AppError::from_technical(
        "SYNC_V2_TRANSFER_UNAVAILABLE",
        "SiteDatum could not finish the approved-computer transfer.",
        "Check your connection and try again. Your local workspace is unchanged.",
        detail,
    )
}

fn sign_in_required() -> AppError {
    AppError::from_technical(
        "SYNC_V2_SIGN_IN_REQUIRED",
        "Your Sync sign-in is no longer valid.",
        "Sign in again before approving another computer.",
        "The trusted bridge rejected the user access token.",
    )
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use ed25519_dalek::{Signature, Verifier, VerifyingKey};
    use sha2::{Digest, Sha256};

    use super::*;

    #[derive(Default)]
    struct MemoryStore(RefCell<Option<String>>);

    impl TransferCredentialStore for MemoryStore {
        fn load(&self) -> AppResult<Option<Zeroizing<String>>> {
            Ok(self.0.borrow().clone().map(Zeroizing::new))
        }

        fn save(&self, value: &str) -> AppResult<()> {
            *self.0.borrow_mut() = Some(value.to_owned());
            Ok(())
        }
    }

    fn enrollment() -> ApprovedEnrollmentContext {
        ApprovedEnrollmentContext {
            owner_id: Uuid::parse_str("81000000-0000-4000-8000-000000000001").unwrap(),
            source_session_id: Uuid::parse_str("82000000-0000-4000-8000-000000000001").unwrap(),
            target_session_id: Uuid::parse_str("82000000-0000-4000-8000-000000000002").unwrap(),
            source_device_id: Uuid::parse_str("83000000-0000-4000-8000-000000000001").unwrap(),
            target_device_id: Uuid::parse_str("83000000-0000-4000-8000-000000000002").unwrap(),
            enrollment_id: Uuid::parse_str("84000000-0000-4000-8000-000000000001").unwrap(),
            target_proof_public_key: [0x11; 32],
            source_transfer_public_key: [0x22; 32],
            target_transfer_public_key: [0x33; 32],
            challenge: [0x44; 32],
            expires_at_unix: 1_800_000_000,
        }
    }

    fn transfer(context: &ApprovedEnrollmentContext) -> TransferContext {
        TransferContext {
            owner_id: context.owner_id,
            workspace_id: Uuid::parse_str("85000000-0000-4000-8000-000000000001").unwrap(),
            source_device_id: context.source_device_id,
            target_device_id: context.target_device_id,
            enrollment_id: context.enrollment_id,
            workspace_key_version: 7,
            expires_at_unix: context.expires_at_unix,
        }
    }

    #[test]
    fn protected_transfer_key_reopens_for_the_exact_device() {
        let store = MemoryStore::default();
        let device_id = Uuid::new_v4();
        let first = load_or_create_transfer_key_with_store(&store, device_id).unwrap();
        let reopened = load_or_create_transfer_key_with_store(&store, device_id).unwrap();
        assert_eq!(first.public_bytes(), reopened.public_bytes());
        assert_eq!(
            first.private_bytes().as_ref(),
            reopened.private_bytes().as_ref()
        );
        assert!(load_or_create_transfer_key_with_store(&store, Uuid::new_v4()).is_err());
    }

    #[test]
    fn authenticated_hpke_recovers_only_the_exact_workspace_key_and_code() {
        let enrollment = enrollment();
        let transfer = transfer(&enrollment);
        let source = TransferKeyPair::generate();
        let target = TransferKeyPair::generate();
        let workspace_key = [0x77; 32];
        let sealed =
            seal_workspace_key(&transfer, &workspace_key, &source, &target.public_bytes()).unwrap();
        let recovered = open_workspace_key(
            &transfer,
            &sealed,
            &source.public_bytes(),
            &target,
            &sealed.comparison_code,
            transfer.expires_at_unix - 1,
        )
        .unwrap();
        assert_eq!(recovered.as_ref(), &workspace_key);
        assert!(open_workspace_key(
            &transfer,
            &sealed,
            &source.public_bytes(),
            &target,
            "000000",
            transfer.expires_at_unix - 1,
        )
        .is_err());
    }

    #[test]
    fn canonical_messages_match_the_edge_vectors() {
        let enrollment = enrollment();
        let transfer = transfer(&enrollment);
        let sealed = SealedWorkspaceKey {
            encapsulated_key: [0x55; 32],
            ciphertext: [0x66; 48],
            comparison_code: "123456".to_owned(),
        };
        let registration = transfer_key_registration_message(
            enrollment.owner_id,
            enrollment.source_session_id,
            enrollment.source_device_id,
            &enrollment.source_transfer_public_key,
        );
        assert_eq!(
            hex_digest(&registration),
            "f4a5e11da7f7595b6e56523316116d5ba05d59df84974273663d7d752f487abd"
        );
        assert_eq!(
            hex_digest(&target_proof_message(&enrollment)),
            "56dbd5a7174016582cbd5bb11ce79702327ec265cfe71a185406d2e6a60dfd07"
        );
        assert_eq!(
            hex_digest(&source_approval_message(&enrollment, &transfer, &sealed).unwrap()),
            "89088c7f8d0dbe52558cd62dc83f814c64bf184b436c37d210aaca31ab402b8b"
        );
    }

    #[test]
    fn target_and_source_signatures_bind_the_exact_context_and_envelope() {
        let mut enrollment = enrollment();
        let target_signing = SigningKey::from_bytes(&[0x31; 32]);
        enrollment.target_proof_public_key = target_signing.verifying_key().to_bytes();
        let target_signature = sign_target_proof(&target_signing, &enrollment);
        VerifyingKey::from_bytes(&enrollment.target_proof_public_key)
            .unwrap()
            .verify(
                &target_proof_message(&enrollment),
                &Signature::from_bytes(&target_signature),
            )
            .unwrap();

        let transfer = transfer(&enrollment);
        let sealed = SealedWorkspaceKey {
            encapsulated_key: [0x55; 32],
            ciphertext: [0x66; 48],
            comparison_code: "123456".to_owned(),
        };
        let source_signing = SigningKey::from_bytes(&[0x32; 32]);
        let source_signature =
            sign_source_approval(&source_signing, &enrollment, &transfer, &sealed).unwrap();
        VerifyingKey::from_bytes(&source_signing.verifying_key().to_bytes())
            .unwrap()
            .verify(
                &source_approval_message(&enrollment, &transfer, &sealed).unwrap(),
                &Signature::from_bytes(&source_signature),
            )
            .unwrap();
        let mut changed = sealed;
        changed.ciphertext[0] ^= 1;
        assert!(source_signing
            .verifying_key()
            .verify(
                &source_approval_message(&enrollment, &transfer, &changed).unwrap(),
                &Signature::from_bytes(&source_signature),
            )
            .is_err());
    }

    #[test]
    fn production_transport_refuses_untrusted_origins() {
        assert!(HttpApprovedDeviceTransport::new("http://example.com", "public".into()).is_err());
        assert!(HttpApprovedDeviceTransport::new("https://example.com", "".into()).is_err());
        assert!(HttpApprovedDeviceTransport::new("https://example.com", "public".into()).is_ok());
    }

    #[cfg(target_os = "windows")]
    #[test]
    #[ignore = "writes and deletes one uniquely named fictional Windows credential"]
    fn production_transfer_store_round_trips_fictional_key() {
        let store =
            WindowsTransferCredentialStore::fictional(format!("fictional-{}", Uuid::new_v4()));
        store.delete();
        let device_id = Uuid::new_v4();
        let first = load_or_create_transfer_key_with_store(&store, device_id).unwrap();
        let reopened = load_or_create_transfer_key_with_store(&store, device_id).unwrap();
        assert_eq!(first.public_bytes(), reopened.public_bytes());
        store.delete();
    }

    fn hex_digest(value: &[u8]) -> String {
        Sha256::digest(value)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect()
    }
}

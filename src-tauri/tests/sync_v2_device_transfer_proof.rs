//! C10-02C approved-device transfer and total-device-loss recovery proof.
//!
//! This integration test is deliberately unreachable from the Tauri library,
//! commands, SQLite workspace, and customer UI. It uses fictional identifiers
//! and in-memory keys only. Sync remains disabled.

use std::collections::HashSet;

use chacha20poly1305::{
    aead::{Aead, KeyInit, Payload},
    Key, XChaCha20Poly1305, XNonce,
};
use hkdf::Hkdf;
use hpke::{
    aead::ChaCha20Poly1305, kdf::HkdfSha256, kem::X25519HkdfSha256, single_shot_open,
    single_shot_seal, Deserializable, Kem as KemTrait, OpModeR, OpModeS, Serializable,
};
use sha2::{Digest, Sha256};
use uuid::Uuid;
use zeroize::Zeroizing;

type Kem = X25519HkdfSha256;
type Kdf = HkdfSha256;
type HpkeAead = ChaCha20Poly1305;

const TRANSFER_INFO: &[u8] = b"sitedatum.sync-v2.device-transfer.hpke.v1";
const TRANSFER_CONTEXT_LABEL: &[u8] = b"sitedatum.sync-v2.device-transfer.context.v1";
const COMPARISON_LABEL: &[u8] = b"sitedatum.sync-v2.device-transfer.comparison.v1";
const RECOVERY_LABEL: &[u8] = b"sitedatum.sync-v2.total-loss-recovery.v1";
const WORKSPACE_KEY_BYTES: usize = 32;
const COMPARISON_MODULUS: u64 = 1_000_000;

#[derive(Clone)]
struct TransferContext {
    owner_id: String,
    workspace_id: String,
    source_device_id: String,
    target_device_id: String,
    enrollment_id: String,
    workspace_key_version: u32,
    expires_at_unix: u64,
}

impl TransferContext {
    fn authenticated_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(256);
        push_field(&mut bytes, TRANSFER_CONTEXT_LABEL);
        push_field(&mut bytes, self.owner_id.as_bytes());
        push_field(&mut bytes, self.workspace_id.as_bytes());
        push_field(&mut bytes, self.source_device_id.as_bytes());
        push_field(&mut bytes, self.target_device_id.as_bytes());
        push_field(&mut bytes, self.enrollment_id.as_bytes());
        bytes.extend_from_slice(&self.workspace_key_version.to_be_bytes());
        bytes.extend_from_slice(&self.expires_at_unix.to_be_bytes());
        bytes
    }
}

struct ServerVisibleTransfer {
    context: TransferContext,
    target_public_key: [u8; 32],
    encapsulated_key: [u8; 32],
    ciphertext: Vec<u8>,
}

#[derive(Debug, PartialEq, Eq)]
enum TransferError {
    Expired,
    RevokedDevice,
    Replay,
    ComparisonMismatch,
    EnvelopeInvalid,
    WorkspaceKeyLength,
}

#[derive(Default)]
struct EnrollmentState {
    consumed: HashSet<String>,
    revoked_devices: HashSet<String>,
}

impl EnrollmentState {
    fn revoke_device(&mut self, device_id: &str) {
        self.revoked_devices.insert(device_id.to_owned());
    }

    fn accept(
        &mut self,
        transfer: &ServerVisibleTransfer,
        target_private_key: &<Kem as KemTrait>::PrivateKey,
        target_public_key: &<Kem as KemTrait>::PublicKey,
        owner_entered_code: &str,
        now_unix: u64,
    ) -> Result<Zeroizing<[u8; WORKSPACE_KEY_BYTES]>, TransferError> {
        let enrollment_id = &transfer.context.enrollment_id;
        if self.consumed.contains(enrollment_id) {
            return Err(TransferError::Replay);
        }

        // Every attempted approval consumes the enrollment. A typo requires a
        // new enrollment instead of permitting online guessing of the short code.
        self.consumed.insert(enrollment_id.clone());

        // Expiry is exclusive: an enrollment is no longer valid at the exact
        // expires_at instant.
        if now_unix >= transfer.context.expires_at_unix {
            return Err(TransferError::Expired);
        }
        if self
            .revoked_devices
            .contains(&transfer.context.target_device_id)
        {
            return Err(TransferError::RevokedDevice);
        }

        let own_public_key = array32(target_public_key.to_bytes().as_slice());
        if own_public_key != transfer.target_public_key {
            return Err(TransferError::ComparisonMismatch);
        }
        let expected_code = comparison_code(
            &transfer.context,
            &own_public_key,
            &transfer.encapsulated_key,
        );
        if !constant_time_code_eq(owner_entered_code, &expected_code) {
            return Err(TransferError::ComparisonMismatch);
        }

        let encapsulated_key =
            <Kem as KemTrait>::EncappedKey::from_bytes(&transfer.encapsulated_key)
                .map_err(|_| TransferError::EnvelopeInvalid)?;
        let plaintext = Zeroizing::new(
            single_shot_open::<HpkeAead, Kdf, Kem>(
                &OpModeR::Base,
                target_private_key,
                &encapsulated_key,
                TRANSFER_INFO,
                &transfer.ciphertext,
                &transfer.context.authenticated_bytes(),
            )
            .map_err(|_| TransferError::EnvelopeInvalid)?,
        );
        if plaintext.len() != WORKSPACE_KEY_BYTES {
            return Err(TransferError::WorkspaceKeyLength);
        }
        let mut workspace_key = Zeroizing::new([0_u8; WORKSPACE_KEY_BYTES]);
        workspace_key.copy_from_slice(plaintext.as_slice());
        Ok(workspace_key)
    }
}

struct RecoveryContext {
    owner_id: String,
    workspace_id: String,
    workspace_key_version: u32,
}

impl RecoveryContext {
    fn authenticated_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(128);
        push_field(&mut bytes, RECOVERY_LABEL);
        push_field(&mut bytes, self.owner_id.as_bytes());
        push_field(&mut bytes, self.workspace_id.as_bytes());
        bytes.extend_from_slice(&self.workspace_key_version.to_be_bytes());
        bytes
    }
}

struct RecoveryEnvelope {
    nonce: [u8; 24],
    ciphertext: Vec<u8>,
}

fn push_field(target: &mut Vec<u8>, value: &[u8]) {
    let length = u32::try_from(value.len()).expect("proof fields are bounded to u32");
    target.extend_from_slice(&length.to_be_bytes());
    target.extend_from_slice(value);
}

fn random_32() -> Zeroizing<[u8; 32]> {
    let mut bytes = Zeroizing::new([0_u8; 32]);
    getrandom::fill(bytes.as_mut()).expect("the operating-system CSPRNG must be available");
    bytes
}

fn array32(bytes: &[u8]) -> [u8; 32] {
    let mut value = [0_u8; 32];
    value.copy_from_slice(bytes);
    value
}

fn transfer_context() -> TransferContext {
    TransferContext {
        owner_id: "owner-fixture-01".to_owned(),
        workspace_id: "workspace-fixture-01".to_owned(),
        source_device_id: "device-source-fixture".to_owned(),
        target_device_id: "device-target-fixture".to_owned(),
        enrollment_id: Uuid::new_v4().to_string(),
        workspace_key_version: 4,
        expires_at_unix: 1_800_000_300,
    }
}

fn seal_transfer(
    context: TransferContext,
    workspace_key: &[u8; WORKSPACE_KEY_BYTES],
    target_public_key: &<Kem as KemTrait>::PublicKey,
) -> (ServerVisibleTransfer, String) {
    let aad = context.authenticated_bytes();
    let (encapsulated_key, ciphertext) = single_shot_seal::<HpkeAead, Kdf, Kem>(
        &OpModeS::Base,
        target_public_key,
        TRANSFER_INFO,
        workspace_key,
        &aad,
    )
    .expect("fictional workspace key seals to the approved target");
    let target_public_key = array32(target_public_key.to_bytes().as_slice());
    let encapsulated_key = array32(encapsulated_key.to_bytes().as_slice());
    let code = comparison_code(&context, &target_public_key, &encapsulated_key);
    (
        ServerVisibleTransfer {
            context,
            target_public_key,
            encapsulated_key,
            ciphertext,
        },
        code,
    )
}

fn comparison_code(
    context: &TransferContext,
    target_public_key: &[u8; 32],
    encapsulated_key: &[u8; 32],
) -> String {
    let mut hash = Sha256::new();
    hash.update(COMPARISON_LABEL);
    hash.update(context.authenticated_bytes());
    hash.update(target_public_key);
    hash.update(encapsulated_key);
    let digest = hash.finalize();
    let number = u64::from_be_bytes(digest[..8].try_into().unwrap()) % COMPARISON_MODULUS;
    format!("{number:06}")
}

fn constant_time_code_eq(left: &str, right: &str) -> bool {
    if left.len() != 6 || right.len() != 6 {
        return false;
    }
    left.as_bytes()
        .iter()
        .zip(right.as_bytes())
        .fold(0_u8, |difference, (left, right)| {
            difference | (left ^ right)
        })
        == 0
}

fn different_code(code: &str) -> &'static str {
    if code == "000000" {
        "000001"
    } else {
        "000000"
    }
}

fn derive_recovery_key(
    recovery_secret: &[u8; 32],
    context: &RecoveryContext,
) -> Zeroizing<[u8; 32]> {
    let binding = context.authenticated_bytes();
    let hkdf = Hkdf::<Sha256>::new(Some(&binding), recovery_secret);
    let mut key = Zeroizing::new([0_u8; 32]);
    hkdf.expand(RECOVERY_LABEL, key.as_mut())
        .expect("a 32-byte HKDF-SHA-256 output is valid");
    key
}

fn seal_recovery_envelope(
    recovery_secret: &[u8; 32],
    context: &RecoveryContext,
    workspace_key: &[u8; WORKSPACE_KEY_BYTES],
) -> RecoveryEnvelope {
    let mut nonce = [0_u8; 24];
    getrandom::fill(&mut nonce).expect("the operating-system CSPRNG must be available");
    let binding = context.authenticated_bytes();
    let recovery_key = derive_recovery_key(recovery_secret, context);
    let cipher = XChaCha20Poly1305::new(Key::from_slice(recovery_key.as_ref()));
    let ciphertext = cipher
        .encrypt(
            XNonce::from_slice(&nonce),
            Payload {
                msg: workspace_key,
                aad: &binding,
            },
        )
        .expect("fictional recovery envelope seals");
    RecoveryEnvelope { nonce, ciphertext }
}

fn open_recovery_envelope(
    recovery_secret: &[u8; 32],
    context: &RecoveryContext,
    envelope: &RecoveryEnvelope,
) -> Result<Zeroizing<[u8; WORKSPACE_KEY_BYTES]>, &'static str> {
    let binding = context.authenticated_bytes();
    let recovery_key = derive_recovery_key(recovery_secret, context);
    let cipher = XChaCha20Poly1305::new(Key::from_slice(recovery_key.as_ref()));
    let plaintext = Zeroizing::new(
        cipher
            .decrypt(
                XNonce::from_slice(&envelope.nonce),
                Payload {
                    msg: &envelope.ciphertext,
                    aad: &binding,
                },
            )
            .map_err(|_| "recovery envelope authentication failed")?,
    );
    if plaintext.len() != WORKSPACE_KEY_BYTES {
        return Err("recovery envelope length is invalid");
    }
    let mut workspace_key = Zeroizing::new([0_u8; WORKSPACE_KEY_BYTES]);
    workspace_key.copy_from_slice(plaintext.as_slice());
    Ok(workspace_key)
}

#[test]
fn surviving_device_transfer_recovers_the_same_workspace_key() {
    let workspace_key = random_32();
    let (target_private_key, target_public_key) = Kem::gen_keypair();
    let (transfer, comparison_code) =
        seal_transfer(transfer_context(), &workspace_key, &target_public_key);
    let mut state = EnrollmentState::default();

    let recovered = state
        .accept(
            &transfer,
            &target_private_key,
            &target_public_key,
            &comparison_code,
            1_800_000_000,
        )
        .unwrap();

    assert_eq!(recovered.as_ref(), workspace_key.as_ref());
}

#[test]
fn comparison_code_is_single_attempt_and_replay_is_refused() {
    let workspace_key = random_32();
    let (target_private_key, target_public_key) = Kem::gen_keypair();
    let (transfer, comparison_code) =
        seal_transfer(transfer_context(), &workspace_key, &target_public_key);
    let mut state = EnrollmentState::default();
    let wrong_code = different_code(&comparison_code);

    assert_eq!(
        state.accept(
            &transfer,
            &target_private_key,
            &target_public_key,
            wrong_code,
            1_800_000_000,
        ),
        Err(TransferError::ComparisonMismatch)
    );
    assert_eq!(
        state.accept(
            &transfer,
            &target_private_key,
            &target_public_key,
            &comparison_code,
            1_800_000_000,
        ),
        Err(TransferError::Replay)
    );
}

#[test]
fn substituted_target_key_is_refused_before_decryption() {
    let workspace_key = random_32();
    let (_target_private_key, target_public_key) = Kem::gen_keypair();
    let (attacker_private_key, attacker_public_key) = Kem::gen_keypair();
    let (transfer, comparison_code) =
        seal_transfer(transfer_context(), &workspace_key, &target_public_key);
    let mut state = EnrollmentState::default();

    assert_eq!(
        state.accept(
            &transfer,
            &attacker_private_key,
            &attacker_public_key,
            &comparison_code,
            1_800_000_000,
        ),
        Err(TransferError::ComparisonMismatch)
    );
}

#[test]
fn modified_context_is_rejected_by_hpke_authentication() {
    let workspace_key = random_32();
    let (target_private_key, target_public_key) = Kem::gen_keypair();
    let (mut transfer, _) = seal_transfer(transfer_context(), &workspace_key, &target_public_key);
    transfer.context.workspace_id = "substituted-workspace".to_owned();
    let recomputed_code = comparison_code(
        &transfer.context,
        &transfer.target_public_key,
        &transfer.encapsulated_key,
    );
    let mut state = EnrollmentState::default();

    assert_eq!(
        state.accept(
            &transfer,
            &target_private_key,
            &target_public_key,
            &recomputed_code,
            1_800_000_000,
        ),
        Err(TransferError::EnvelopeInvalid)
    );
}

#[test]
fn every_transfer_context_field_changes_the_authenticated_binding() {
    let original = transfer_context();
    let binding = original.authenticated_bytes();

    let mut mutations = Vec::new();
    let mut owner = original.clone();
    owner.owner_id.push_str("-changed");
    mutations.push(owner);
    let mut workspace = original.clone();
    workspace.workspace_id.push_str("-changed");
    mutations.push(workspace);
    let mut source = original.clone();
    source.source_device_id.push_str("-changed");
    mutations.push(source);
    let mut target = original.clone();
    target.target_device_id.push_str("-changed");
    mutations.push(target);
    let mut enrollment = original.clone();
    enrollment.enrollment_id.push_str("-changed");
    mutations.push(enrollment);
    let mut version = original.clone();
    version.workspace_key_version += 1;
    mutations.push(version);
    let mut expiry = original;
    expiry.expires_at_unix += 1;
    mutations.push(expiry);

    for mutation in mutations {
        assert_ne!(mutation.authenticated_bytes(), binding);
    }
}

#[test]
fn expired_and_revoked_enrollments_are_refused() {
    let workspace_key = random_32();
    let (target_private_key, target_public_key) = Kem::gen_keypair();

    let (boundary_transfer, boundary_code) =
        seal_transfer(transfer_context(), &workspace_key, &target_public_key);
    let mut boundary_state = EnrollmentState::default();
    assert_eq!(
        boundary_state.accept(
            &boundary_transfer,
            &target_private_key,
            &target_public_key,
            &boundary_code,
            boundary_transfer.context.expires_at_unix,
        ),
        Err(TransferError::Expired)
    );

    let (expired_transfer, expired_code) =
        seal_transfer(transfer_context(), &workspace_key, &target_public_key);
    let mut expired_state = EnrollmentState::default();
    assert_eq!(
        expired_state.accept(
            &expired_transfer,
            &target_private_key,
            &target_public_key,
            &expired_code,
            expired_transfer.context.expires_at_unix + 1,
        ),
        Err(TransferError::Expired)
    );

    let (revoked_transfer, revoked_code) =
        seal_transfer(transfer_context(), &workspace_key, &target_public_key);
    let mut revoked_state = EnrollmentState::default();
    revoked_state.revoke_device(&revoked_transfer.context.target_device_id);
    assert_eq!(
        revoked_state.accept(
            &revoked_transfer,
            &target_private_key,
            &target_public_key,
            &revoked_code,
            1_800_000_000,
        ),
        Err(TransferError::RevokedDevice)
    );
}

#[test]
fn customer_held_secret_recovers_after_total_device_loss() {
    let workspace_key = random_32();
    let recovery_secret = random_32();
    let context = RecoveryContext {
        owner_id: "owner-fixture-01".to_owned(),
        workspace_id: "workspace-fixture-01".to_owned(),
        workspace_key_version: 4,
    };
    let envelope = seal_recovery_envelope(&recovery_secret, &context, &workspace_key);

    // No device private key participates in this path. The encrypted envelope
    // and its routing context are sufficient only when the customer supplies
    // the separate recovery secret locally.
    let recovered = open_recovery_envelope(&recovery_secret, &context, &envelope).unwrap();
    assert_eq!(recovered.as_ref(), workspace_key.as_ref());

    let wrong_secret = random_32();
    assert_eq!(
        open_recovery_envelope(&wrong_secret, &context, &envelope).unwrap_err(),
        "recovery envelope authentication failed"
    );
}

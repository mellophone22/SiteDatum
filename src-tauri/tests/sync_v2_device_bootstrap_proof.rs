//! C10-03B initial-device proof-of-possession protocol proof.
//!
//! This test is isolated from the application, commands, workspace, and UI.
//! It establishes the exact Ed25519 message a future trusted enrollment bridge
//! must verify before consuming a server-side enrollment.

use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use sha2::{Digest, Sha256};
use uuid::Uuid;

const CONTEXT_LABEL: &[u8] = b"sitedatum.sync-v2.initial-device-possession.v1";

#[derive(Clone)]
struct EnrollmentContext {
    owner_id: Uuid,
    auth_session_id: Uuid,
    device_id: Uuid,
    enrollment_id: Uuid,
    public_key: [u8; 32],
    challenge: [u8; 32],
    expires_at_unix: u64,
}

impl EnrollmentContext {
    fn signed_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(180);
        push_field(&mut bytes, CONTEXT_LABEL);
        push_field(&mut bytes, self.owner_id.as_bytes());
        push_field(&mut bytes, self.auth_session_id.as_bytes());
        push_field(&mut bytes, self.device_id.as_bytes());
        push_field(&mut bytes, self.enrollment_id.as_bytes());
        push_field(&mut bytes, &self.public_key);
        push_field(&mut bytes, &self.challenge);
        bytes.extend_from_slice(&self.expires_at_unix.to_be_bytes());
        bytes
    }
}

fn push_field(bytes: &mut Vec<u8>, field: &[u8]) {
    bytes.extend_from_slice(&(field.len() as u32).to_be_bytes());
    bytes.extend_from_slice(field);
}

fn verify_proof(context: &EnrollmentContext, signature: &[u8]) -> bool {
    let Ok(public_key) = VerifyingKey::from_bytes(&context.public_key) else {
        return false;
    };
    let Ok(signature) = Signature::from_slice(signature) else {
        return false;
    };
    public_key
        .verify(&context.signed_bytes(), &signature)
        .is_ok()
}

fn fixture(signing_key: &SigningKey) -> EnrollmentContext {
    EnrollmentContext {
        owner_id: Uuid::parse_str("61000000-0000-4000-8000-000000000001").unwrap(),
        auth_session_id: Uuid::parse_str("62000000-0000-4000-8000-000000000001").unwrap(),
        device_id: Uuid::parse_str("63000000-0000-4000-8000-000000000001").unwrap(),
        enrollment_id: Uuid::parse_str("64000000-0000-4000-8000-000000000001").unwrap(),
        public_key: signing_key.verifying_key().to_bytes(),
        challenge: [0xA5; 32],
        expires_at_unix: 1_800_000_000,
    }
}

#[test]
fn valid_device_key_proves_possession_for_exact_server_context() {
    let signing_key = SigningKey::from_bytes(&[0x31; 32]);
    let context = fixture(&signing_key);
    let signature = signing_key.sign(&context.signed_bytes());

    assert!(verify_proof(&context, &signature.to_bytes()));
}

#[test]
fn wrong_key_and_malformed_signature_are_refused() {
    let signing_key = SigningKey::from_bytes(&[0x32; 32]);
    let attacker_key = SigningKey::from_bytes(&[0x33; 32]);
    let context = fixture(&signing_key);

    let attacker_signature = attacker_key.sign(&context.signed_bytes());
    assert!(!verify_proof(&context, &attacker_signature.to_bytes()));
    assert!(!verify_proof(&context, &[0x44; 63]));
}

#[test]
fn signature_cannot_be_replayed_into_changed_context() {
    let signing_key = SigningKey::from_bytes(&[0x34; 32]);
    let original = fixture(&signing_key);
    let signature = signing_key.sign(&original.signed_bytes());

    let mut mutations = Vec::new();
    let mut changed = original.clone();
    changed.owner_id = Uuid::new_v4();
    mutations.push(changed);
    let mut changed = original.clone();
    changed.auth_session_id = Uuid::new_v4();
    mutations.push(changed);
    let mut changed = original.clone();
    changed.device_id = Uuid::new_v4();
    mutations.push(changed);
    let mut changed = original.clone();
    changed.enrollment_id = Uuid::new_v4();
    mutations.push(changed);
    let mut changed = original.clone();
    changed.challenge[0] ^= 1;
    mutations.push(changed);
    let mut changed = original.clone();
    changed.expires_at_unix += 1;
    mutations.push(changed);

    for changed in mutations {
        assert!(!verify_proof(&changed, &signature.to_bytes()));
    }
}

#[test]
fn public_key_substitution_is_bound_into_the_signed_context() {
    let signing_key = SigningKey::from_bytes(&[0x35; 32]);
    let other_key = SigningKey::from_bytes(&[0x36; 32]);
    let original = fixture(&signing_key);
    let signature = signing_key.sign(&original.signed_bytes());
    let mut substituted = original.clone();
    substituted.public_key = other_key.verifying_key().to_bytes();

    assert!(!verify_proof(&substituted, &signature.to_bytes()));
}

#[test]
fn canonical_message_matches_the_trusted_bridge_vector() {
    let context = EnrollmentContext {
        owner_id: Uuid::parse_str("61000000-0000-4000-8000-000000000001").unwrap(),
        auth_session_id: Uuid::parse_str("62000000-0000-4000-8000-000000000001").unwrap(),
        device_id: Uuid::parse_str("63000000-0000-4000-8000-000000000001").unwrap(),
        enrollment_id: Uuid::parse_str("64000000-0000-4000-8000-000000000001").unwrap(),
        public_key: std::array::from_fn(|index| index as u8),
        challenge: [0xA5; 32],
        expires_at_unix: 1_800_000_000,
    };
    let digest = Sha256::digest(context.signed_bytes());

    let actual = digest
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    assert_eq!(
        actual,
        "1fb13514a81b5f847ed102855decd6c9d6a49761fbc48b16dcdfb8c5665897c4"
    );
}

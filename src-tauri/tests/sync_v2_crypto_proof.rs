//! Offline-only C10-02 cryptographic protocol proof.
//!
//! This integration test is deliberately not linked from the Tauri library,
//! exposed as a command, or reachable from the customer UI. It proves the
//! selected record-encryption and customer-held recovery boundary before any
//! local database, Windows credential, identity-provider, or hosted-service
//! integration is authorized.

use std::collections::HashSet;

use chacha20poly1305::{
    aead::{Aead, KeyInit, Payload},
    Key, XChaCha20Poly1305, XNonce,
};
use hkdf::Hkdf;
use sha2::Sha256;
use zeroize::Zeroizing;

const PROTOCOL_LABEL: &[u8] = b"sitedatum.sync-v2.record.v1";
const RECOVERY_LABEL: &[u8] = b"sitedatum.sync-v2.recovery-wrap.v1";

#[derive(Clone, Copy)]
struct RecordContext<'a> {
    owner_id: &'a str,
    workspace_id: &'a str,
    entity: &'a str,
    record_id: &'a str,
    revision: u64,
}

#[derive(Clone, Copy)]
struct RecoveryContext<'a> {
    owner_id: &'a str,
    workspace_id: &'a str,
    workspace_key_version: u32,
}

impl RecoveryContext<'_> {
    fn authenticated_bytes(self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(96);
        push_field(&mut bytes, RECOVERY_LABEL);
        push_field(&mut bytes, self.owner_id.as_bytes());
        push_field(&mut bytes, self.workspace_id.as_bytes());
        bytes.extend_from_slice(&self.workspace_key_version.to_be_bytes());
        bytes
    }
}

impl RecordContext<'_> {
    fn authenticated_bytes(self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(128);
        push_field(&mut bytes, PROTOCOL_LABEL);
        push_field(&mut bytes, self.owner_id.as_bytes());
        push_field(&mut bytes, self.workspace_id.as_bytes());
        push_field(&mut bytes, self.entity.as_bytes());
        push_field(&mut bytes, self.record_id.as_bytes());
        bytes.extend_from_slice(&self.revision.to_be_bytes());
        bytes
    }
}

fn push_field(target: &mut Vec<u8>, value: &[u8]) {
    let length = u32::try_from(value.len()).expect("protocol fields are bounded to u32");
    target.extend_from_slice(&length.to_be_bytes());
    target.extend_from_slice(value);
}

fn seal_record(
    workspace_key: &[u8; 32],
    nonce: &[u8; 24],
    context: RecordContext<'_>,
    plaintext: &[u8],
) -> Result<Vec<u8>, &'static str> {
    let cipher = XChaCha20Poly1305::new(Key::from_slice(workspace_key));
    cipher
        .encrypt(
            XNonce::from_slice(nonce),
            Payload {
                msg: plaintext,
                aad: &context.authenticated_bytes(),
            },
        )
        .map_err(|_| "record encryption failed")
}

fn open_record(
    workspace_key: &[u8; 32],
    nonce: &[u8; 24],
    context: RecordContext<'_>,
    ciphertext: &[u8],
) -> Result<Vec<u8>, &'static str> {
    let cipher = XChaCha20Poly1305::new(Key::from_slice(workspace_key));
    cipher
        .decrypt(
            XNonce::from_slice(nonce),
            Payload {
                msg: ciphertext,
                aad: &context.authenticated_bytes(),
            },
        )
        .map_err(|_| "record authentication failed")
}

fn derive_recovery_wrap_key(
    recovery_secret: &[u8; 32],
    context: RecoveryContext<'_>,
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
    context: RecoveryContext<'_>,
    nonce: &[u8; 24],
    workspace_key: &[u8; 32],
) -> Result<Vec<u8>, &'static str> {
    let binding = context.authenticated_bytes();
    let wrap_key = derive_recovery_wrap_key(recovery_secret, context);
    let cipher = XChaCha20Poly1305::new(Key::from_slice(wrap_key.as_ref()));
    cipher
        .encrypt(
            XNonce::from_slice(nonce),
            Payload {
                msg: workspace_key,
                aad: &binding,
            },
        )
        .map_err(|_| "recovery envelope encryption failed")
}

fn open_recovery_envelope(
    recovery_secret: &[u8; 32],
    context: RecoveryContext<'_>,
    nonce: &[u8; 24],
    envelope: &[u8],
) -> Result<Zeroizing<[u8; 32]>, &'static str> {
    let binding = context.authenticated_bytes();
    let wrap_key = derive_recovery_wrap_key(recovery_secret, context);
    let cipher = XChaCha20Poly1305::new(Key::from_slice(wrap_key.as_ref()));
    let plaintext = Zeroizing::new(
        cipher
            .decrypt(
                XNonce::from_slice(nonce),
                Payload {
                    msg: envelope,
                    aad: &binding,
                },
            )
            .map_err(|_| "recovery envelope authentication failed")?,
    );
    if plaintext.len() != 32 {
        return Err("recovery envelope length is invalid");
    }
    let mut key = [0_u8; 32];
    key.copy_from_slice(plaintext.as_slice());
    Ok(Zeroizing::new(key))
}

#[derive(Default)]
struct NonceLedger {
    claimed: HashSet<[u8; 24]>,
}

impl NonceLedger {
    fn claim(&mut self, nonce: [u8; 24]) -> Result<(), &'static str> {
        if self.claimed.insert(nonce) {
            Ok(())
        } else {
            Err("nonce reuse refused")
        }
    }
}

fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut encoded = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        encoded.push(DIGITS[(byte >> 4) as usize] as char);
        encoded.push(DIGITS[(byte & 0x0f) as usize] as char);
    }
    encoded
}

fn context() -> RecordContext<'static> {
    RecordContext {
        owner_id: "owner-fixture-01",
        workspace_id: "workspace-fixture-01",
        entity: "task",
        record_id: "task-fixture-01",
        revision: 7,
    }
}

fn recovery_context() -> RecoveryContext<'static> {
    RecoveryContext {
        owner_id: "owner-fixture-01",
        workspace_id: "workspace-fixture-01",
        workspace_key_version: 3,
    }
}

#[test]
fn hkdf_matches_rfc_5869_sha256_test_case_1() {
    let input_key_material = [0x0b_u8; 22];
    let salt = [
        0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c,
    ];
    let info = [0xf0, 0xf1, 0xf2, 0xf3, 0xf4, 0xf5, 0xf6, 0xf7, 0xf8, 0xf9];
    let hkdf = Hkdf::<Sha256>::new(Some(&salt), &input_key_material);
    let mut output = [0_u8; 42];
    hkdf.expand(&info, &mut output).unwrap();

    assert_eq!(
        hex(&output),
        "3cb25f25faacd57a90434f64d0362f2a2d2d0a90cf1a5a4c5db02d56ecc4c5bf34007208d5b887185865"
    );
}

#[test]
fn record_ciphertext_is_stable_and_bound_to_its_context() {
    let workspace_key = [0x11_u8; 32];
    let nonce = [0x22_u8; 24];
    let plaintext = br#"{"status":"waiting","title":"Fixture only"}"#;
    let ciphertext = seal_record(&workspace_key, &nonce, context(), plaintext).unwrap();

    assert_eq!(
        hex(&ciphertext),
        "f84594783dcd91e7c77736bcdc10d58b5cf0e1f837ea93900c2e54dd53a6305d2aa2c426a590b42e9e5d1e229d53c45d213f36b5fffc8b57f5890e"
    );
    assert_eq!(
        open_record(&workspace_key, &nonce, context(), &ciphertext).unwrap(),
        plaintext
    );

    let wrong_revision = RecordContext {
        revision: 8,
        ..context()
    };
    assert_eq!(
        open_record(&workspace_key, &nonce, wrong_revision, &ciphertext),
        Err("record authentication failed")
    );
}

#[test]
fn tampering_is_rejected_without_returning_plaintext() {
    let workspace_key = [0x31_u8; 32];
    let nonce = [0x41_u8; 24];
    let mut ciphertext =
        seal_record(&workspace_key, &nonce, context(), b"fictional metadata").unwrap();
    ciphertext[0] ^= 0x80;

    assert_eq!(
        open_record(&workspace_key, &nonce, context(), &ciphertext),
        Err("record authentication failed")
    );
}

#[test]
fn customer_held_secret_recovers_the_workspace_key_locally() {
    let recovery_secret = [0x51_u8; 32];
    let workspace_key = [0x61_u8; 32];
    let nonce = [0x71_u8; 24];
    let envelope =
        seal_recovery_envelope(&recovery_secret, recovery_context(), &nonce, &workspace_key)
            .unwrap();

    assert_eq!(
        hex(&envelope),
        "3a87401b953f95809ea256ac21ab38911920a756948de7ad556e597a195c8ee1006c833ec120fe7a1d520fc983ba85db"
    );
    let recovered =
        open_recovery_envelope(&recovery_secret, recovery_context(), &nonce, &envelope).unwrap();
    assert_eq!(recovered.as_ref(), &workspace_key);

    let wrong_secret = [0x52_u8; 32];
    assert!(open_recovery_envelope(&wrong_secret, recovery_context(), &nonce, &envelope).is_err());
    let wrong_owner = RecoveryContext {
        owner_id: "owner-fixture-02",
        ..recovery_context()
    };
    assert!(open_recovery_envelope(&recovery_secret, wrong_owner, &nonce, &envelope).is_err());
    let wrong_workspace = RecoveryContext {
        workspace_id: "another-workspace",
        ..recovery_context()
    };
    assert!(open_recovery_envelope(&recovery_secret, wrong_workspace, &nonce, &envelope).is_err());
    let wrong_key_version = RecoveryContext {
        workspace_key_version: 4,
        ..recovery_context()
    };
    assert!(
        open_recovery_envelope(&recovery_secret, wrong_key_version, &nonce, &envelope).is_err()
    );
}

#[test]
fn nonce_reuse_is_refused_before_encryption() {
    let mut ledger = NonceLedger::default();
    let nonce = [0x81_u8; 24];

    assert_eq!(ledger.claim(nonce), Ok(()));
    assert_eq!(ledger.claim(nonce), Err("nonce reuse refused"));
}

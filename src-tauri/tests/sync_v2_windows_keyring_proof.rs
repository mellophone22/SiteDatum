//! C10-02B Windows device-key lifecycle proof.
//!
//! This test target is not imported by the Tauri library and exposes no command
//! or UI surface. The live test is ignored by default because it briefly writes
//! one uniquely named fictional credential to Windows Credential Manager. A
//! cleanup guard removes that exact entry even if an assertion panics.

#![cfg(target_os = "windows")]

use base64::{engine::general_purpose::STANDARD_NO_PAD, Engine as _};
use keyring::Entry;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;
use zeroize::{Zeroize, Zeroizing};

const PROOF_SERVICE: &str = "com.sitedatum.sync-v2.c10-device-key-proof";
const RECORD_SCHEMA: &str = "sitedatum.sync-v2.device-key.v1";
const KEY_BYTES: usize = 32;

#[derive(Serialize, Deserialize)]
struct StoredDeviceKey {
    schema: String,
    key_version: u32,
    private_key_b64: String,
}

impl Drop for StoredDeviceKey {
    fn drop(&mut self) {
        self.schema.zeroize();
        self.key_version.zeroize();
        self.private_key_b64.zeroize();
    }
}

impl StoredDeviceKey {
    fn from_key(key_version: u32, private_key: &[u8; KEY_BYTES]) -> Self {
        Self {
            schema: RECORD_SCHEMA.to_owned(),
            key_version,
            private_key_b64: STANDARD_NO_PAD.encode(private_key),
        }
    }

    fn decode_key(&self) -> Result<Zeroizing<[u8; KEY_BYTES]>, &'static str> {
        if self.schema != RECORD_SCHEMA {
            return Err("device-key schema is unsupported");
        }
        let decoded = Zeroizing::new(
            STANDARD_NO_PAD
                .decode(&self.private_key_b64)
                .map_err(|_| "device-key encoding is invalid")?,
        );
        if decoded.len() != KEY_BYTES {
            return Err("device-key length is invalid");
        }
        let mut key = Zeroizing::new([0_u8; KEY_BYTES]);
        key.copy_from_slice(decoded.as_slice());
        Ok(key)
    }
}

struct CredentialCleanup {
    account: String,
}

impl CredentialCleanup {
    fn new(account: String) -> Self {
        Self { account }
    }
}

impl Drop for CredentialCleanup {
    fn drop(&mut self) {
        if let Ok(entry) = Entry::new(PROOF_SERVICE, &self.account) {
            let _ = entry.delete_credential();
        }
    }
}

fn random_device_key() -> Zeroizing<[u8; KEY_BYTES]> {
    let mut key = Zeroizing::new([0_u8; KEY_BYTES]);
    getrandom::fill(key.as_mut()).expect("Windows CSPRNG must be available for this proof");
    key
}

fn serialize_record(record: &StoredDeviceKey) -> Zeroizing<String> {
    Zeroizing::new(serde_json::to_string(record).expect("fictional device-key record serializes"))
}

fn read_record(entry: &Entry) -> StoredDeviceKey {
    let stored = Zeroizing::new(
        entry
            .get_password()
            .expect("fictional device-key credential is readable"),
    );
    serde_json::from_str(stored.as_str()).expect("fictional device-key record is valid")
}

fn fingerprint(key: &[u8; KEY_BYTES]) -> [u8; 32] {
    Sha256::digest(key).into()
}

#[test]
fn device_key_record_is_versioned_and_rejects_invalid_material() {
    let key = Zeroizing::new([0x4d_u8; KEY_BYTES]);
    let record = StoredDeviceKey::from_key(7, &key);
    let serialized = serialize_record(&record);
    let parsed: StoredDeviceKey = serde_json::from_str(serialized.as_str()).unwrap();

    assert_eq!(parsed.schema, RECORD_SCHEMA);
    assert_eq!(parsed.key_version, 7);
    assert_eq!(parsed.decode_key().unwrap().as_ref(), key.as_ref());

    let invalid = StoredDeviceKey {
        schema: RECORD_SCHEMA.to_owned(),
        key_version: 8,
        private_key_b64: STANDARD_NO_PAD.encode([0_u8; KEY_BYTES - 1]),
    };
    assert_eq!(
        invalid.decode_key().unwrap_err(),
        "device-key length is invalid"
    );
}

#[test]
#[ignore = "writes and deletes one uniquely named fictional Windows credential"]
fn windows_credential_manager_proves_create_read_replace_reopen_and_revoke() {
    let account = format!("fictional-device-{}", Uuid::new_v4());
    let _cleanup = CredentialCleanup::new(account.clone());
    let entry = Entry::new(PROOF_SERVICE, &account).expect("Windows credential entry opens");

    match entry.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => {}
        Err(error) => panic!("fictional proof credential could not be cleared: {error}"),
    }

    let first_key = random_device_key();
    let first_fingerprint = fingerprint(&first_key);
    let first_record = StoredDeviceKey::from_key(1, &first_key);
    let first_serialized = serialize_record(&first_record);
    entry
        .set_password(first_serialized.as_str())
        .expect("fictional version-one device key is stored");

    let first_read = read_record(&entry);
    assert_eq!(first_read.key_version, 1);
    assert_eq!(
        fingerprint(&first_read.decode_key().unwrap()),
        first_fingerprint
    );

    let replacement_key = random_device_key();
    let replacement_fingerprint = fingerprint(&replacement_key);
    assert_ne!(replacement_fingerprint, first_fingerprint);
    let replacement_record = StoredDeviceKey::from_key(2, &replacement_key);
    let replacement_serialized = serialize_record(&replacement_record);
    entry
        .set_password(replacement_serialized.as_str())
        .expect("fictional version-two device key replaces version one");

    drop(entry);
    let reopened = Entry::new(PROOF_SERVICE, &account)
        .expect("a fresh process-equivalent credential handle opens");
    let replacement_read = read_record(&reopened);
    assert_eq!(replacement_read.key_version, 2);
    assert_eq!(
        fingerprint(&replacement_read.decode_key().unwrap()),
        replacement_fingerprint
    );
    assert_ne!(
        fingerprint(&replacement_read.decode_key().unwrap()),
        first_fingerprint
    );

    reopened
        .delete_credential()
        .expect("fictional device key is revoked and deleted");
    assert!(matches!(
        reopened.get_password(),
        Err(keyring::Error::NoEntry)
    ));
}

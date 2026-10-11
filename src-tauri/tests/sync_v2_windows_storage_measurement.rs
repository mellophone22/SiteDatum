//! D3 Windows adapter measurements, not a vault or preservation format.
//! Only UUID-scoped fictional entries are addressed. No credential enumeration.
//! Live tests are ignored by default and require an explicit local invocation.
//! Payloads are public test strings/bytes, never customer or generated key material.

#![cfg(target_os = "windows")]

use keyring::{Entry, Error};
use std::time::Instant;
use uuid::Uuid;
use zeroize::Zeroizing;

const SERVICE: &str = "com.sitedatum.sync-v2.c10-d3-storage-measurement";
const MAX_BLOB_BYTES: usize = 2560;

struct OwnedEntries {
    scope: Uuid,
    accounts: Vec<String>,
}

impl OwnedEntries {
    fn new() -> Self {
        Self {
            scope: Uuid::new_v4(),
            accounts: Vec::new(),
        }
    }

    fn entry(&self, index: usize) -> Entry {
        Entry::new(SERVICE, &self.accounts[index])
            .unwrap_or_else(|_| panic!("fictional credential handle unavailable"))
    }

    fn allocate(&mut self) -> usize {
        let account = format!("fictional-{}-{}", self.scope, self.accounts.len());
        let entry = Entry::new(SERVICE, &account)
            .unwrap_or_else(|_| panic!("fictional credential handle unavailable"));
        // Refuse an unexpected collision rather than deleting/overwriting it.
        assert!(
            matches!(entry.get_secret(), Err(Error::NoEntry)),
            "fictional target was not absent"
        );
        self.accounts.push(account);
        self.accounts.len() - 1
    }

    fn cleanup_verified(&mut self) {
        for index in 0..self.accounts.len() {
            let entry = self.entry(index);
            assert!(
                matches!(entry.delete_credential(), Ok(()) | Err(Error::NoEntry)),
                "fictional credential cleanup failed"
            );
            assert!(
                matches!(self.entry(index).get_secret(), Err(Error::NoEntry)),
                "fictional credential remained after cleanup"
            );
        }
        self.accounts.clear();
    }
}

impl Drop for OwnedEntries {
    fn drop(&mut self) {
        // Cleanup never discovers targets: it uses only this run's exact list.
        let mut failures = 0;
        for account in &self.accounts {
            match Entry::new(SERVICE, account) {
                Ok(entry) => {
                    if !matches!(entry.delete_credential(), Ok(()) | Err(Error::NoEntry)) {
                        failures += 1;
                    } else if !matches!(entry.get_secret(), Err(Error::NoEntry)) {
                        failures += 1;
                    }
                }
                Err(_) => failures += 1,
            }
        }
        if failures != 0 {
            // Counts only. No targets, account names, values or raw OS errors.
            eprintln!("D3 fictional cleanup failures={failures}");
        } else if !self.accounts.is_empty() {
            eprintln!("D3 guard_cleanup_verified={}", self.accounts.len());
        }
    }
}

fn read_password(entry: &Entry) -> Zeroizing<String> {
    Zeroizing::new(
        entry
            .get_password()
            .unwrap_or_else(|_| panic!("fictional password read refused")),
    )
}

fn payload(index: usize) -> String {
    // Exactly 96 ASCII characters; 192 bytes through keyring's UTF-16 adapter.
    format!("fixture-entry-{index:04}-{}", "x".repeat(77))
}

fn fixture_inventory_matches(owned: &OwnedEntries, first: usize, count: usize) -> bool {
    // Known fixture equality is NOT an authenticated production catalog.
    (0..count).all(|index| {
        owned
            .entry(first + index)
            .get_password()
            .map(|stored| Zeroizing::new(stored).as_str() == payload(index))
            .unwrap_or(false)
    })
}

#[test]
fn fixture_sizes_use_utf16_bytes_not_utf8_or_character_count() {
    assert_eq!(payload(0).len(), 96);
    assert_eq!(payload(999).encode_utf16().count() * 2, 192);
    assert_eq!("x".repeat(1280).encode_utf16().count() * 2, MAX_BLOB_BYTES);
    assert_eq!("😀".repeat(640).encode_utf16().count() * 2, MAX_BLOB_BYTES);
    assert_eq!(
        "😀".repeat(641).encode_utf16().count() * 2,
        MAX_BLOB_BYTES + 4
    );
}

#[test]
#[ignore = "explicit UUID-scoped fictional Windows credential size measurements"]
fn windows_password_and_secret_boundaries_preserve_the_previous_value() {
    let mut owned = OwnedEntries::new();
    let index = owned.allocate();
    for characters in [1, 256, 444, 1024, 1280] {
        let value = Zeroizing::new("x".repeat(characters));
        assert!(
            owned.entry(index).set_password(&value).is_ok(),
            "bounded fictional write refused"
        );
        assert!(
            read_password(&owned.entry(index)).as_str() == value.as_str(),
            "readback mismatch"
        );
        println!(
            "D3 password_ascii_chars={characters} blob_bytes={} readback=pass",
            characters * 2
        );
    }
    let maximum = Zeroizing::new("x".repeat(1280));
    assert!(
        matches!(
            owned.entry(index).set_password(&"x".repeat(1281)),
            Err(Error::TooLong(_, _))
        ),
        "oversized ASCII password did not refuse at the adapter boundary"
    );
    assert!(
        read_password(&owned.entry(index)).as_str() == maximum.as_str(),
        "oversized replacement changed the prior value"
    );
    let unicode = Zeroizing::new("😀".repeat(640));
    assert!(
        owned.entry(index).set_password(&unicode).is_ok(),
        "bounded UTF-16 write refused"
    );
    assert!(
        matches!(
            owned.entry(index).set_password(&"😀".repeat(641)),
            Err(Error::TooLong(_, _))
        ),
        "oversized UTF-16 password did not refuse"
    );
    assert!(
        read_password(&owned.entry(index)).as_str() == unicode.as_str(),
        "UTF-16 readback mismatch"
    );
    let bytes = Zeroizing::new(vec![0x41; MAX_BLOB_BYTES]);
    assert!(
        owned.entry(index).set_secret(&bytes).is_ok(),
        "bounded binary write refused"
    );
    assert!(
        matches!(
            owned
                .entry(index)
                .set_secret(&vec![0x41; MAX_BLOB_BYTES + 1]),
            Err(Error::TooLong(_, _))
        ),
        "oversized binary secret did not refuse"
    );
    let read = Zeroizing::new(
        owned
            .entry(index)
            .get_secret()
            .unwrap_or_else(|_| panic!("fictional binary read refused")),
    );
    assert!(
        read.as_slice() == bytes.as_slice(),
        "binary prior value changed"
    );
    owned.cleanup_verified();
    println!("D3 boundary_checks=pass oversized_replacement_retained=pass cleanup_verified=1");
}

#[test]
#[ignore = "explicit 1/8/100/1000 UUID-scoped fictional Windows entry measurement"]
fn windows_known_entry_inventory_measurements() {
    for count in [1, 8, 100, 1000] {
        let mut owned = OwnedEntries::new();
        println!("D3 inventory_started entries={count}");
        let started = Instant::now();
        let mut written = 0;
        let mut refusal = "none";
        for index in 0..count {
            let slot = owned.allocate();
            if let Err(error) = owned.entry(slot).set_password(&payload(index)) {
                refusal = match error {
                    Error::PlatformFailure(_) => "platform_failure",
                    Error::NoStorageAccess(_) => "storage_access",
                    Error::TooLong(_, _) => "too_long",
                    _ => "other",
                };
                break;
            }
            written += 1;
        }
        let write_us = started.elapsed().as_micros();
        // Fresh handles, exact known targets: no listing real credentials.
        let started = Instant::now();
        assert!(
            fixture_inventory_matches(&owned, 0, written),
            "fictional inventory readback mismatch"
        );
        let read_us = started.elapsed().as_micros();
        let started = Instant::now();
        let tracked = owned.accounts.len();
        owned.cleanup_verified();
        let cleanup_us = started.elapsed().as_micros();
        println!("D3 requested_entries={count} written_entries={written} refusal={refusal} blob_bytes_per_entry=192 total_payload_bytes={} write_us={write_us} known_target_read_us={read_us} cleanup_us={cleanup_us} cleanup_verified={tracked}",
            written * 192);
        // Observational measurement: a refusal is NOT proof of requested capacity.
        // Functional readback and exact-target cleanup must still pass.
        if refusal != "none" {
            break;
        }
    }
}

#[test]
#[ignore = "explicit fictional missing/substituted entry test; no product keyring changed"]
fn windows_missing_or_substituted_fixture_entry_keeps_hot_control_unchanged() {
    let mut owned = OwnedEntries::new();
    let hot = owned.allocate();
    let control = Zeroizing::new("fictional-hot-control-not-a-key".to_string());
    assert!(
        owned.entry(hot).set_password(&control).is_ok(),
        "fictional control write refused"
    );
    let first = owned.accounts.len();
    for index in 0..8 {
        let slot = owned.allocate();
        assert!(
            owned.entry(slot).set_password(&payload(index)).is_ok(),
            "fictional entry write refused"
        );
    }
    assert!(fixture_inventory_matches(&owned, first, 8));
    assert!(
        owned.entry(first + 3).delete_credential().is_ok(),
        "fictional deletion refused"
    );
    assert!(
        !fixture_inventory_matches(&owned, first, 8),
        "missing fixture entry was accepted"
    );
    assert!(
        read_password(&owned.entry(hot)).as_str() == control.as_str(),
        "control changed"
    );
    assert!(
        owned.entry(first + 3).set_password(&payload(3)).is_ok(),
        "fixture restore refused"
    );
    assert!(
        owned.entry(first + 4).set_password(&payload(5)).is_ok(),
        "fixture substitution refused"
    );
    assert!(
        !fixture_inventory_matches(&owned, first, 8),
        "substituted fixture was accepted"
    );
    assert!(
        read_password(&owned.entry(hot)).as_str() == control.as_str(),
        "control changed"
    );
    owned.cleanup_verified();
    println!("D3 missing_fixture=refused substituted_fixture=refused hot_control=unchanged cleanup_verified=9");
}

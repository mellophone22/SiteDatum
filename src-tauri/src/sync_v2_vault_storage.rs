//! Dormant Windows vault persistence boundary. No application calls or activation.
//! Caller must supply a trusted destination parent and independently retained pin.
//! No active pointer, key deletion, pin discovery or multi-store journal is provided.
#![allow(dead_code)]

use super::sync_v2_vault_file_prototype::{Archive, Pin, Refusal, Secret};
use sha2::{Digest, Sha256};
use std::os::windows::fs::{MetadataExt, OpenOptionsExt};
use std::{
    fs::{File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    path::Path,
};
use uuid::Uuid;

const MAGIC: &[u8; 8] = b"SDPB0001"; // Experimental bundle, not an accepted shipping format.
const MAX_ARCHIVE: usize = 36_092;
const MAX_WRAP: usize = 8192;
const MAX_BUNDLE: usize = 12 + MAX_WRAP + MAX_ARCHIVE;
const REPARSE: u32 = 0x400;
const OPEN_REPARSE: u32 = 0x00200000;
type Result<T> = std::result::Result<T, Refusal>;

// Public metadata only. Cannot be deserialized from an untrusted verified flag.
// Holding this is NOT retirement authority or proof of a global current tip.
#[derive(Clone, Copy)]
pub(super) struct ExpectedBundle {
    owner: Uuid,
    workspace: Uuid,
    generation: u64,
    digest: [u8; 32],
}
impl ExpectedBundle {
    pub(super) fn from_independent_pin(
        owner: Uuid,
        workspace: Uuid,
        generation: u64,
        digest: [u8; 32],
    ) -> Result<Self> {
        if owner.is_nil()
            || workspace.is_nil()
            || generation == 0
            || generation > super::sync_v2_record_codec::MAX_SAFE_INTEGER
        {
            return Err(Refusal::Invalid);
        }
        Ok(Self {
            owner,
            workspace,
            generation,
            digest,
        })
    }
}

#[derive(Clone, Copy)]
enum Phase {
    Created,
    PrefixWritten,
    Written,
    Synced,
    ReadBack,
}

fn inspect_handle(file: &File, limit: usize) -> Result<()> {
    let metadata = file.metadata().map_err(|_| Refusal::Storage)?;
    if !metadata.is_file()
        || metadata.file_attributes() & REPARSE != 0
        || metadata.len() > limit as u64
    {
        return Err(Refusal::Invalid);
    }
    Ok(())
}
fn read_handle(file: &mut File, limit: usize) -> Result<Vec<u8>> {
    inspect_handle(file, limit)?;
    file.seek(SeekFrom::Start(0))
        .map_err(|_| Refusal::Storage)?;
    let mut bytes = Vec::with_capacity(limit + 1);
    file.take((limit + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|_| Refusal::Storage)?;
    if bytes.len() > limit {
        return Err(Refusal::Invalid);
    }
    Ok(bytes)
}
fn read_locked(path: &Path, limit: usize) -> Result<Vec<u8>> {
    // Final component is opened without following a reparse point, then inspected
    // on that handle. Parent directory custody is a caller prerequisite, not proved.
    let mut file = OpenOptions::new()
        .read(true)
        .share_mode(1)
        .custom_flags(OPEN_REPARSE)
        .open(path)
        .map_err(|_| Refusal::Storage)?;
    read_handle(&mut file, limit)
}
fn write_checked(
    path: &Path,
    bytes: &[u8],
    mut after: impl FnMut(Phase) -> Result<()>,
) -> Result<()> {
    if bytes.is_empty() || bytes.len() > MAX_BUNDLE {
        return Err(Refusal::Invalid);
    }
    // No overwrite, delete-on-close or rename. Exclusive handle through readback.
    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .create_new(true)
        .share_mode(0)
        .custom_flags(OPEN_REPARSE)
        .open(path)
        .map_err(|_| Refusal::Storage)?;
    let finish = (|| {
        inspect_handle(&file, MAX_BUNDLE)?;
        after(Phase::Created)?;
        let prefix = bytes.len().min(12);
        file.write_all(&bytes[..prefix])
            .map_err(|_| Refusal::Storage)?;
        after(Phase::PrefixWritten)?;
        file.write_all(&bytes[prefix..])
            .map_err(|_| Refusal::Storage)?;
        after(Phase::Written)?;
        file.sync_all().map_err(|_| Refusal::Storage)?;
        after(Phase::Synced)?;
        if read_handle(&mut file, MAX_BUNDLE)? != bytes {
            return Err(Refusal::Invalid);
        }
        after(Phase::ReadBack)
    })();
    // Any failure after creating a new file leaves it unconfirmed for inspection.
    finish.map_err(|_| Refusal::Incomplete)
}

pub(super) fn create_local(path: &Path, archive: &Archive) -> Result<ExpectedBundle> {
    create_local_inner(path, archive, |_| Ok(()))
}
fn create_local_inner(
    path: &Path,
    archive: &Archive,
    after: impl FnMut(Phase) -> Result<()>,
) -> Result<ExpectedBundle> {
    let (owner, workspace, generation) = archive.context();
    let secret = Secret::random()?;
    let wrapped = secret.wrap_local()?;
    if wrapped.is_empty() || wrapped.len() > MAX_WRAP {
        return Err(Refusal::Invalid);
    }
    let (ciphertext, _) = archive.seal(&secret, false)?;
    let mut bytes = Vec::with_capacity(12 + wrapped.len() + ciphertext.len());
    bytes.extend_from_slice(MAGIC);
    bytes.extend_from_slice(&(wrapped.len() as u32).to_be_bytes());
    bytes.extend_from_slice(&wrapped);
    bytes.extend_from_slice(&ciphertext);
    let expected = ExpectedBundle::from_independent_pin(
        owner,
        workspace,
        generation,
        Sha256::digest(&bytes).into(),
    )?;
    write_checked(path, &bytes, after)?;
    drop(secret); // Verify by reopening the persisted protection, not this secret.
    open_local(path, expected).map_err(|_| Refusal::Incomplete)?;
    Ok(expected)
}
pub(super) fn open_local(path: &Path, expected: ExpectedBundle) -> Result<Archive> {
    let bytes = read_locked(path, MAX_BUNDLE)?;
    if bytes.len() < 12
        || &bytes[..8] != MAGIC
        || <[u8; 32]>::from(Sha256::digest(&bytes)) != expected.digest
    {
        return Err(Refusal::Invalid);
    }
    let count = u32::from_be_bytes(bytes[8..12].try_into().map_err(|_| Refusal::Invalid)?) as usize;
    if count == 0
        || count > MAX_WRAP
        || bytes.len() < 12 + count + 128
        || bytes.len() > 12 + count + MAX_ARCHIVE
    {
        return Err(Refusal::Invalid);
    }
    let secret = Secret::unwrap_local(&bytes[12..12 + count])?;
    let archive = &bytes[12 + count..];
    Archive::open(
        archive,
        &secret,
        false,
        expected.owner,
        expected.workspace,
        Pin {
            generation: expected.generation,
            digest: Sha256::digest(archive).into(),
        },
    )
}

pub(super) fn create_recovery(path: &Path, archive: &Archive, secret: &Secret) -> Result<Pin> {
    let (bytes, pin) = archive.seal(secret, true)?;
    write_checked(path, &bytes, |_| Ok(()))?;
    let (o, w, _) = archive.context();
    open_recovery(path, secret, o, w, pin).map_err(|_| Refusal::Incomplete)?;
    Ok(pin)
}
pub(super) fn open_recovery(
    path: &Path,
    secret: &Secret,
    owner: Uuid,
    workspace: Uuid,
    pin: Pin,
) -> Result<Archive> {
    Archive::open(
        &read_locked(path, MAX_ARCHIVE)?,
        secret,
        true,
        owner,
        workspace,
        pin,
    )
}

#[cfg(test)]
mod tests {
    use super::super::sync_v2_key_recovery::WorkspaceKeys;
    use super::*;
    use zeroize::Zeroizing;

    struct Fixtures {
        root: std::path::PathBuf,
        files: Vec<std::path::PathBuf>,
    }
    impl Fixtures {
        fn new() -> Self {
            let root =
                std::env::temp_dir().join(format!("sitedatum-storage-fixtures-{}", Uuid::new_v4()));
            std::fs::create_dir(&root).expect("fictional directory creation refused");
            Self {
                root,
                files: Vec::new(),
            }
        }
        fn path(&mut self) -> std::path::PathBuf {
            let path = self
                .root
                .join(format!("sitedatum-storage-{}.fixture", Uuid::new_v4()));
            assert!(!path.exists());
            self.files.push(path.clone());
            path
        }
    }
    impl Drop for Fixtures {
        fn drop(&mut self) {
            for path in &self.files {
                // Exact UUID-scoped fictional paths only; no enumeration/recursion.
                if path.exists() {
                    std::fs::remove_file(path).expect("fictional cleanup refused");
                }
            }
            std::fs::remove_dir(&self.root).expect("fictional directory cleanup refused");
        }
    }
    fn fixture(count: u32, generation: u64, owner: Uuid, workspace: Uuid) -> Archive {
        Archive::new(
            owner,
            workspace,
            generation,
            (1..=count)
                .map(|v| (v, Zeroizing::new([((v % 251) + 1) as u8; 32])))
                .collect(),
        )
        .unwrap()
    }

    #[test]
    fn interrupted_generation_never_changes_original_or_returns_a_verified_pin() {
        let mut paths = Fixtures::new();
        let old = paths.path();
        let owner = Uuid::new_v4();
        let workspace = Uuid::new_v4();
        let original = fixture(8, 1, owner, workspace);
        let pin = create_local(&old, &original).unwrap();
        let before = read_locked(&old, MAX_BUNDLE).unwrap();
        for stop in 0..5 {
            let candidate = paths.path();
            let mut position = 0;
            let result = create_local_inner(&candidate, &fixture(9, 2, owner, workspace), |_| {
                let here = position;
                position += 1;
                if here == stop {
                    Err(Refusal::Storage)
                } else {
                    Ok(())
                }
            });
            assert!(matches!(result, Err(Refusal::Incomplete)));
            assert_eq!(read_locked(&old, MAX_BUNDLE).unwrap(), before);
            assert!(open_local(&old, pin).is_ok());
            assert!(candidate.exists()); // no silently deleted partial new files
        }
        assert!(matches!(
            create_local(&old, &original),
            Err(Refusal::Storage)
        ));
        assert_eq!(read_locked(&old, MAX_BUNDLE).unwrap(), before);
    }
    #[test]
    fn recovery_is_independent_and_old_or_wrong_context_cannot_be_substituted() {
        let owner = Uuid::new_v4();
        let workspace = Uuid::new_v4();
        for count in [1, 8, 100, 1000] {
            let mut paths = Fixtures::new();
            let local = paths.path();
            let recovery = paths.path();
            let archive = fixture(count, 2, owner, workspace);
            let secret = Secret::random().unwrap();
            let local_pin = create_local(&local, &archive).unwrap();
            let recovery_pin = create_recovery(&recovery, &archive, &secret).unwrap();
            drop(archive);
            // Remove only our fictional local bundle to model loss of local protection.
            std::fs::remove_file(&local).unwrap();
            let restored =
                open_recovery(&recovery, &secret, owner, workspace, recovery_pin).unwrap();
            assert_eq!(restored.write_version(), None);
            for v in 1..=count {
                assert!(restored.select(owner, workspace, v).is_ok());
            }
            assert!(open_recovery(
                &recovery,
                &Secret::random().unwrap(),
                owner,
                workspace,
                recovery_pin
            )
            .is_err());
            assert!(
                open_recovery(&recovery, &secret, Uuid::new_v4(), workspace, recovery_pin).is_err()
            );
            assert!(open_local(&local, local_pin).is_err());
            let new = paths.path();
            let current = fixture(count, 3, owner, workspace);
            let newer_pin = create_recovery(&new, &current, &secret).unwrap();
            assert!(open_recovery(&recovery, &secret, owner, workspace, newer_pin).is_err());
        }
    }
    #[test]
    fn bounded_reads_refuse_corrupt_oversized_and_directory_inputs() {
        let mut paths = Fixtures::new();
        let path = paths.path();
        let o = Uuid::new_v4();
        let w = Uuid::new_v4();
        let pin = create_local(&path, &fixture(1, 1, o, w)).unwrap();
        let mut bytes = read_locked(&path, MAX_BUNDLE).unwrap();
        bytes[12] ^= 1;
        let corrupt = paths.path();
        write_checked(&corrupt, &bytes, |_| Ok(())).unwrap();
        assert!(open_local(&corrupt, pin).is_err());
        let fake = ExpectedBundle {
            digest: Sha256::digest(&bytes).into(),
            ..pin
        };
        assert!(open_local(&corrupt, fake).is_err()); // DPAPI/AEAD, not just digest
        let huge = paths.path();
        let f = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&huge)
            .unwrap();
        f.set_len((MAX_BUNDLE + 1) as u64).unwrap();
        drop(f);
        assert!(open_local(&huge, pin).is_err());
        assert!(open_local(&std::env::temp_dir(), pin).is_err());
    }
    #[test]
    fn concurrent_creators_cannot_replace_the_same_generation_file() {
        let mut paths = Fixtures::new();
        let path = paths.path();
        let o = Uuid::new_v4();
        let w = Uuid::new_v4();
        let a = fixture(8, 1, o, w);
        let b = fixture(8, 1, o, w);
        let path2 = path.clone();
        let thread = std::thread::spawn(move || create_local(&path2, &a));
        let second = create_local(&path, &b);
        let first = thread.join().unwrap();
        assert_ne!(first.is_ok(), second.is_ok());
        let expected = first.or(second).unwrap();
        assert!(open_local(&path, expected).is_ok());
    }
    #[test]
    fn another_process_can_reopen_without_plaintext_secret_transfer() {
        let mut paths = Fixtures::new();
        let path = paths.path();
        let o = Uuid::new_v4();
        let w = Uuid::new_v4();
        let pin = create_local(&path, &fixture(1000, 1, o, w)).unwrap();
        let digest = pin
            .digest
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>();
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "sync_v2_vault_storage::tests::child_reopen_probe",
                "--ignored",
            ])
            .env("SITEDATUM_VAULT_CHILD_PATH", &path)
            .env("SITEDATUM_VAULT_CHILD_OWNER", o.to_string())
            .env("SITEDATUM_VAULT_CHILD_WORKSPACE", w.to_string())
            .env("SITEDATUM_VAULT_CHILD_DIGEST", digest)
            .output()
            .unwrap();
        assert!(output.status.success(), "fictional child reopen refused");
        assert!(
            String::from_utf8_lossy(&output.stdout).contains("1 passed; 0 failed"),
            "fictional child probe did not execute exactly one passing test"
        );
    }
    #[test]
    fn terminated_writer_leaves_previous_generation_readable() {
        let mut paths = Fixtures::new();
        let old = paths.path();
        let candidate = paths.path();
        let o = Uuid::new_v4();
        let w = Uuid::new_v4();
        let pin = create_local(&old, &fixture(8, 1, o, w)).unwrap();
        let previous = read_locked(&old, MAX_BUNDLE).unwrap();
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "sync_v2_vault_storage::tests::child_reopen_probe",
                "--ignored",
            ])
            .env("SITEDATUM_VAULT_CHILD_PATH", &candidate)
            .env("SITEDATUM_VAULT_CHILD_OWNER", o.to_string())
            .env("SITEDATUM_VAULT_CHILD_WORKSPACE", w.to_string())
            .env("SITEDATUM_VAULT_CHILD_MODE", "interrupt")
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(23));
        assert_eq!(read_locked(&candidate, MAX_BUNDLE).unwrap().len(), 12);
        assert!(open_local(&candidate, pin).is_err());
        assert_eq!(read_locked(&old, MAX_BUNDLE).unwrap(), previous);
        assert!(open_local(&old, pin).is_ok());
    }
    #[test]
    fn open_write_handle_denies_other_readers_and_writers() {
        let mut paths = Fixtures::new();
        let path = paths.path();
        let a = fixture(8, 1, Uuid::new_v4(), Uuid::new_v4());
        let expected = create_local_inner(&path, &a, |phase| {
            if matches!(phase, Phase::Written) {
                assert!(OpenOptions::new()
                    .write(true)
                    .share_mode(3)
                    .open(&path)
                    .is_err());
                assert!(read_locked(&path, MAX_BUNDLE).is_err());
            }
            Ok(())
        })
        .unwrap();
        assert!(open_local(&path, expected).is_ok());
    }
    #[test]
    #[ignore = "only invoked by the parent fictional subprocess fixture"]
    fn child_reopen_probe() {
        let path = std::path::PathBuf::from(
            std::env::var_os("SITEDATUM_VAULT_CHILD_PATH").expect("fixture path absent"),
        );
        let parent = path.parent().unwrap();
        assert_eq!(parent.parent(), Some(std::env::temp_dir().as_path()));
        let directory = parent.file_name().unwrap().to_str().unwrap();
        assert!(Uuid::parse_str(
            directory
                .strip_prefix("sitedatum-storage-fixtures-")
                .unwrap()
        )
        .is_ok());
        let name = path.file_name().unwrap().to_str().unwrap();
        let middle = name
            .strip_prefix("sitedatum-storage-")
            .unwrap()
            .strip_suffix(".fixture")
            .unwrap();
        assert!(Uuid::parse_str(middle).is_ok());
        let o = Uuid::parse_str(&std::env::var("SITEDATUM_VAULT_CHILD_OWNER").unwrap()).unwrap();
        let w =
            Uuid::parse_str(&std::env::var("SITEDATUM_VAULT_CHILD_WORKSPACE").unwrap()).unwrap();
        if std::env::var("SITEDATUM_VAULT_CHILD_MODE").as_deref() == Ok("interrupt") {
            let archive = fixture(9, 2, o, w);
            let _ = create_local_inner(&path, &archive, |phase| {
                if matches!(phase, Phase::PrefixWritten) {
                    std::process::exit(23);
                }
                Ok(())
            });
            panic!("fictional writer did not terminate at its boundary");
        }
        let hex = std::env::var("SITEDATUM_VAULT_CHILD_DIGEST").unwrap();
        assert_eq!(hex.len(), 64);
        let mut digest = [0; 32];
        for (i, b) in digest.iter_mut().enumerate() {
            *b = u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16).unwrap();
        }
        let archive = open_local(
            &path,
            ExpectedBundle::from_independent_pin(o, w, 1, digest).unwrap(),
        )
        .unwrap();
        assert_eq!(archive.write_version(), None);
        for v in 1..=1000 {
            assert!(archive.select(o, w, v).is_ok());
        }
    }
}

//! Dormant recovery ceremony backend. No UI/clipboard or durable trust authority.
#![allow(dead_code)]
use super::sync_v2_vault_file_prototype::{Archive, Pin, Refusal, RetainedManifest, Secret};
use super::sync_v2_vault_storage::{create_recovery, open_recovery};
use std::path::Path;
use uuid::Uuid;
use zeroize::Zeroizing;
type Result<T> = std::result::Result<T, Refusal>;
pub(super) struct RecoverySetup {
    archive: Archive,
    secret: Secret,
    revealed: bool,
    id: Uuid,
}
pub(super) struct PendingRecovery {
    owner: Uuid,
    workspace: Uuid,
    generation: u64,
    pin: Pin,
    manifest: RetainedManifest,
    comparison_key: Zeroizing<[u8; 32]>,
    id: Uuid,
    confirmed: bool,
}
/// In-memory readback only. NOT durable custody, an independent pin, a current
/// roster, an all-device barrier, or retirement/readiness authorization.
/// Never serialize, persist, log or send this receipt over IPC.
#[must_use]
pub(super) struct InProcessReadbackReceipt {
    owner: Uuid,
    workspace: Uuid,
    generation: u64,
    manifest: RetainedManifest,
    comparison_key: Zeroizing<[u8; 32]>,
    id: Uuid,
}
impl RecoverySetup {
    pub(super) fn ceremony_id(&self) -> Uuid {
        self.id
    }
    pub(super) fn begin(archive: Archive) -> Result<Self> {
        Ok(Self {
            archive,
            secret: Secret::random()?,
            revealed: false,
            id: Uuid::new_v4(),
        })
    }
    pub(super) fn reveal_once(&mut self) -> Result<Zeroizing<String>> {
        if self.revealed {
            return Err(Refusal::Invalid);
        }
        self.revealed = true;
        Ok(self.secret.recovery_text())
    }
    pub(super) fn save_copy(self, path: &Path) -> Result<PendingRecovery> {
        if !self.revealed {
            return Err(Refusal::Invalid);
        }
        let pin = create_recovery(path, &self.archive, &self.secret)?;
        let (owner, workspace, generation) = self.archive.context();
        let mut comparison_key = Zeroizing::new([0; 32]);
        getrandom::fill(comparison_key.as_mut()).map_err(|_| Refusal::Storage)?;
        let manifest = self.archive.retained_manifest(&comparison_key);
        // On return the original archive and setup secret are dropped/zeroized.
        Ok(PendingRecovery {
            owner,
            workspace,
            generation,
            pin,
            manifest,
            comparison_key,
            id: self.id,
            confirmed: false,
        })
    }
}
impl PendingRecovery {
    pub(super) fn confirm(
        &mut self,
        path: &Path,
        reentered: &str,
    ) -> Result<InProcessReadbackReceipt> {
        if self.confirmed {
            return Err(Refusal::Invalid);
        }
        let secret = Secret::from_recovery_text(reentered)?;
        let restored = open_recovery(path, &secret, self.owner, self.workspace, self.pin)?;
        if restored.retained_manifest(&self.comparison_key) != self.manifest {
            return Err(Refusal::Invalid);
        }
        self.confirmed = true;
        Ok(InProcessReadbackReceipt {
            owner: self.owner,
            workspace: self.workspace,
            generation: self.generation,
            manifest: self.manifest.clone(),
            comparison_key: Zeroizing::new(*self.comparison_key),
            id: self.id,
        })
    }
    pub(super) fn cancel(self) {} // Drops/zeroizes only the comparison key; never deletes the copy.
}
impl InProcessReadbackReceipt {
    pub(super) fn proves_readback_of(&self, archive: &Archive, ceremony_id: Uuid) -> bool {
        self.id == ceremony_id
            && archive.context() == (self.owner, self.workspace, self.generation)
            && archive.retained_manifest(&self.comparison_key) == self.manifest
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    struct CopyFixture {
        root: std::path::PathBuf,
        path: std::path::PathBuf,
    }
    impl CopyFixture {
        fn new() -> Self {
            let root =
                std::env::temp_dir().join(format!("sitedatum-confirm-fixture-{}", Uuid::new_v4()));
            std::fs::create_dir(&root).unwrap();
            Self {
                path: root.join("recovery.fixture"),
                root,
            }
        }
    }
    impl Drop for CopyFixture {
        fn drop(&mut self) {
            // Only this test-owned file and empty directory; no recursive deletion.
            if self.path.exists() {
                std::fs::remove_file(&self.path).expect("fictional copy cleanup refused");
            }
            std::fs::remove_dir(&self.root).expect("fictional directory cleanup refused");
        }
    }
    fn archive(n: u32, g: u64, o: Uuid, w: Uuid) -> Archive {
        Archive::new(
            o,
            w,
            g,
            (1..=n)
                .map(|v| (v, Zeroizing::new([((v % 251) + 1) as u8; 32])))
                .collect(),
        )
        .unwrap()
    }
    #[test]
    fn another_ceremony_local_bundle_and_wrong_manifest_never_confirm() {
        let o = Uuid::new_v4();
        let w = Uuid::new_v4();
        let first = CopyFixture::new();
        let second = CopyFixture::new();
        let local = CopyFixture::new();
        let mut setup = RecoverySetup::begin(archive(8, 1, o, w)).unwrap();
        let text = setup.reveal_once().unwrap();
        let mut pending = setup.save_copy(&first.path).unwrap();
        // Same archive AND same re-entered secret, but a distinct seal/ceremony.
        let secret = Secret::from_recovery_text(&text).unwrap();
        create_recovery(&second.path, &archive(8, 1, o, w), &secret).unwrap();
        assert!(pending.confirm(&second.path, &text).is_err());
        assert!(!pending.confirmed);
        super::super::sync_v2_vault_storage::create_local(&local.path, &archive(8, 1, o, w))
            .unwrap();
        assert!(pending.confirm(&local.path, &text).is_err());
        assert!(!pending.confirmed);
        let expected = pending.manifest.clone();
        pending.manifest = archive(7, 1, o, w).retained_manifest(&pending.comparison_key);
        assert!(pending.confirm(&first.path, &text).is_err());
        assert!(!pending.confirmed);
        pending.manifest = expected;
        assert!(pending.confirm(&first.path, &text).is_ok());
    }
    #[test]
    fn comparison_fingerprint_is_per_ceremony_and_not_a_stable_key_set_identifier() {
        let o = Uuid::new_v4();
        let w = Uuid::new_v4();
        let first = CopyFixture::new();
        let second = CopyFixture::new();
        let mut a = RecoverySetup::begin(archive(8, 1, o, w)).unwrap();
        let _ = a.reveal_once().unwrap();
        let mut b = RecoverySetup::begin(archive(8, 1, o, w)).unwrap();
        let _ = b.reveal_once().unwrap();
        let a = a.save_copy(&first.path).unwrap();
        let b = b.save_copy(&second.path).unwrap();
        assert!(a.manifest != b.manifest);
    }
    #[test]
    fn saved_flag_is_not_confirmation_and_only_reentered_secret_can_confirm_full_set() {
        for n in [1, 8, 100, 1000] {
            let o = Uuid::new_v4();
            let w = Uuid::new_v4();
            let mut setup = RecoverySetup::begin(archive(n, 1, o, w)).unwrap();
            let ceremony = setup.ceremony_id();
            let text = setup.reveal_once().unwrap();
            assert!(setup.reveal_once().is_err());
            let fixture = CopyFixture::new();
            let path = fixture.path.clone();
            let mut pending = setup.save_copy(&path).unwrap();
            assert!(!pending.confirmed);
            assert!(pending
                .confirm(&path, &Secret::random().unwrap().recovery_text())
                .is_err());
            assert!(!pending.confirmed);
            let confirmed = pending.confirm(&path, &text).unwrap();
            assert!(confirmed.proves_readback_of(&archive(n, 1, o, w), ceremony));
            assert!(!confirmed.proves_readback_of(&archive(n, 2, o, w), ceremony));
            assert!(!confirmed.proves_readback_of(&archive(n, 1, Uuid::new_v4(), w), ceremony));
            assert!(!confirmed.proves_readback_of(
                &archive(if n < 1000 { n + 1 } else { 999 }, 1, o, w),
                ceremony
            ));
            assert!(!confirmed.proves_readback_of(&archive(n, 1, o, w), Uuid::new_v4()));
            let altered = Archive::new(
                o,
                w,
                1,
                (1..=n)
                    .map(|v| {
                        (
                            v,
                            Zeroizing::new([if v == 1 { 0xaa } else { ((v % 251) + 1) as u8 }; 32]),
                        )
                    })
                    .collect(),
            )
            .unwrap();
            assert!(!confirmed.proves_readback_of(&altered, ceremony));
            assert!(pending.confirm(&path, &text).is_err());
            std::fs::remove_file(&path).unwrap();
        }
    }
    #[test]
    fn cancellation_preserves_copy_and_malformed_secret_never_confirms() {
        let o = Uuid::new_v4();
        let w = Uuid::new_v4();
        let mut setup = RecoverySetup::begin(archive(8, 1, o, w)).unwrap();
        let text = setup.reveal_once().unwrap();
        let fixture = CopyFixture::new();
        let path = fixture.path.clone();
        let mut pending = setup.save_copy(&path).unwrap();
        for bad in ["", "short", "===", &"x".repeat(1000)] {
            assert!(pending.confirm(&path, bad).is_err());
        }
        assert!(Secret::from_recovery_text(&(text.to_string() + "=")).is_err());
        pending.cancel();
        assert!(path.is_file());
        std::fs::remove_file(&path).unwrap();
    }
    #[test]
    fn corrupted_saved_copy_cannot_mark_ceremony_confirmed() {
        let o = Uuid::new_v4();
        let w = Uuid::new_v4();
        let mut setup = RecoverySetup::begin(archive(8, 1, o, w)).unwrap();
        let text = setup.reveal_once().unwrap();
        let fixture = CopyFixture::new();
        let mut pending = setup.save_copy(&fixture.path).unwrap();
        let mut bytes = std::fs::read(&fixture.path).unwrap();
        let last = bytes.len() - 1;
        bytes[last] ^= 1;
        // Explicit tampering of this exact fictional copy only.
        std::fs::write(&fixture.path, bytes).unwrap();
        assert!(pending.confirm(&fixture.path, &text).is_err());
        assert!(!pending.confirmed);
        pending.cancel();
        assert!(fixture.path.exists());
    }
}

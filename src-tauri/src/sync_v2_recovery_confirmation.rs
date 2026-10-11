//! Dormant recovery ceremony backend. No UI/clipboard or durable trust authority.
#![allow(dead_code)]
use super::sync_v2_vault_file_prototype::{Archive, Pin, Refusal, Secret};
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
    manifest: [u8; 32],
    id: Uuid,
    confirmed: bool,
}
pub(super) struct ConfirmedRecovery {
    owner: Uuid,
    workspace: Uuid,
    generation: u64,
    manifest: [u8; 32],
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
        let manifest = self.archive.retained_manifest();
        // On return the original archive and setup secret are dropped/zeroized.
        Ok(PendingRecovery {
            owner,
            workspace,
            generation,
            pin,
            manifest,
            id: self.id,
            confirmed: false,
        })
    }
}
impl PendingRecovery {
    pub(super) fn confirm(&mut self, path: &Path, reentered: &str) -> Result<ConfirmedRecovery> {
        if self.confirmed {
            return Err(Refusal::Invalid);
        }
        let secret = Secret::from_recovery_text(reentered)?;
        let restored = open_recovery(path, &secret, self.owner, self.workspace, self.pin)?;
        if restored.retained_manifest() != self.manifest {
            return Err(Refusal::Invalid);
        }
        self.confirmed = true;
        Ok(ConfirmedRecovery {
            owner: self.owner,
            workspace: self.workspace,
            generation: self.generation,
            manifest: self.manifest,
            id: self.id,
        })
    }
    pub(super) fn cancel(self) {} // Metadata only; never deletes the customer copy.
}
impl ConfirmedRecovery {
    pub(super) fn matches(&self, archive: &Archive, ceremony_id: Uuid) -> bool {
        self.id == ceremony_id
            && archive.context() == (self.owner, self.workspace, self.generation)
            && archive.retained_manifest() == self.manifest
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
            assert!(confirmed.matches(&archive(n, 1, o, w), ceremony));
            assert!(!confirmed.matches(&archive(n, 2, o, w), ceremony));
            assert!(!confirmed.matches(&archive(n, 1, Uuid::new_v4(), w), ceremony));
            assert!(!confirmed.matches(
                &archive(if n < 1000 { n + 1 } else { 999 }, 1, o, w),
                ceremony
            ));
            assert!(!confirmed.matches(&archive(n, 1, o, w), Uuid::new_v4()));
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
            assert!(!confirmed.matches(&altered, ceremony));
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

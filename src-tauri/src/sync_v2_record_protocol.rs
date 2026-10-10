//! C10-04A local checkpoint integrity and anti-rollback foundation.
//!
//! This module has no Tauri command and does not enable Sync. It authenticates
//! encrypted workspace checkpoints against a canonical record manifest, then
//! durably records only the highest accepted counter/digest in a local-only
//! SQLite table that is excluded from the replicated record set.

#![allow(dead_code)] // Deliberately dormant until the later C10 enablement gate.

use chacha20poly1305::{
    aead::{Aead, KeyInit, Payload},
    XChaCha20Poly1305, XNonce,
};
use rusqlite::{params, OptionalExtension};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::{
    error::{AppError, AppResult},
    persistence::Database,
};

const PROTOCOL_VERSION: u16 = 1;
const CHECKPOINT_LABEL: &[u8] = b"sitedatum.sync-v2.workspace-checkpoint.v1";
const MANIFEST_LABEL: &[u8] = b"sitedatum.sync-v2.record-manifest.v1";
const NONCE_BYTES: usize = 24;
const TAG_BYTES: usize = 16;
const CHECKPOINT_PLAINTEXT_BYTES: usize = 2 + 8 + 8 + 32;
const MAX_CHECKPOINT_CIPHERTEXT_BYTES: usize = NONCE_BYTES + CHECKPOINT_PLAINTEXT_BYTES + TAG_BYTES;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct RecordManifestEntry {
    pub record_id: Uuid,
    pub record_revision: u64,
    pub workspace_key_version: u32,
    pub tombstone: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SealedWorkspaceCheckpoint {
    pub protocol_version: u16,
    pub checkpoint_counter: u64,
    pub through_change_seq: u64,
    pub workspace_key_version: u32,
    pub ciphertext: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct VerifiedWorkspaceCheckpoint {
    pub checkpoint_counter: u64,
    pub through_change_seq: u64,
    pub manifest_digest: [u8; 32],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum CheckpointAcceptance {
    Advanced,
    AlreadyAccepted,
}

pub(crate) fn manifest_digest(entries: &[RecordManifestEntry]) -> AppResult<[u8; 32]> {
    let mut ordered = entries.to_vec();
    ordered.sort_unstable_by_key(|entry| *entry.record_id.as_bytes());

    let mut hasher = Sha256::new();
    hasher.update(MANIFEST_LABEL);
    hasher.update((ordered.len() as u64).to_be_bytes());
    let mut previous = None;
    for entry in ordered {
        if entry.record_revision == 0 || entry.workspace_key_version == 0 {
            return Err(checkpoint_invalid(
                "A record manifest entry used a zero revision or key version.",
            ));
        }
        if previous == Some(entry.record_id) {
            return Err(checkpoint_invalid(
                "The record manifest contained a duplicate record identifier.",
            ));
        }
        previous = Some(entry.record_id);
        hasher.update(entry.record_id.as_bytes());
        hasher.update(entry.record_revision.to_be_bytes());
        hasher.update(entry.workspace_key_version.to_be_bytes());
        hasher.update([u8::from(entry.tombstone)]);
    }
    Ok(hasher.finalize().into())
}

pub(crate) fn seal_workspace_checkpoint(
    owner_id: Uuid,
    workspace_id: Uuid,
    workspace_key_version: u32,
    workspace_key: &[u8; 32],
    checkpoint_counter: u64,
    through_change_seq: u64,
    manifest: &[RecordManifestEntry],
) -> AppResult<SealedWorkspaceCheckpoint> {
    validate_checkpoint_numbers(
        workspace_key_version,
        checkpoint_counter,
        through_change_seq,
    )?;
    let digest = manifest_digest(manifest)?;
    let mut plaintext = Vec::with_capacity(CHECKPOINT_PLAINTEXT_BYTES);
    plaintext.extend_from_slice(&PROTOCOL_VERSION.to_be_bytes());
    plaintext.extend_from_slice(&checkpoint_counter.to_be_bytes());
    plaintext.extend_from_slice(&through_change_seq.to_be_bytes());
    plaintext.extend_from_slice(&digest);

    let mut nonce = [0_u8; NONCE_BYTES];
    getrandom::fill(&mut nonce).map_err(|error| {
        checkpoint_crypto_error(format!(
            "Operating-system random generation failed: {error}"
        ))
    })?;
    let cipher = XChaCha20Poly1305::new(workspace_key.into());
    let encrypted = cipher
        .encrypt(
            XNonce::from_slice(&nonce),
            Payload {
                msg: &plaintext,
                aad: &checkpoint_aad(
                    owner_id,
                    workspace_id,
                    workspace_key_version,
                    checkpoint_counter,
                    through_change_seq,
                ),
            },
        )
        .map_err(|_| checkpoint_crypto_error("Checkpoint encryption failed."))?;
    let mut ciphertext = Vec::with_capacity(NONCE_BYTES + encrypted.len());
    ciphertext.extend_from_slice(&nonce);
    ciphertext.extend_from_slice(&encrypted);
    Ok(SealedWorkspaceCheckpoint {
        protocol_version: PROTOCOL_VERSION,
        checkpoint_counter,
        through_change_seq,
        workspace_key_version,
        ciphertext,
    })
}

pub(crate) fn open_workspace_checkpoint(
    owner_id: Uuid,
    workspace_id: Uuid,
    workspace_key: &[u8; 32],
    envelope: &SealedWorkspaceCheckpoint,
    manifest: &[RecordManifestEntry],
) -> AppResult<VerifiedWorkspaceCheckpoint> {
    if envelope.protocol_version != PROTOCOL_VERSION
        || envelope.ciphertext.len() != MAX_CHECKPOINT_CIPHERTEXT_BYTES
    {
        return Err(checkpoint_invalid(
            "The checkpoint protocol or ciphertext length is unsupported.",
        ));
    }
    validate_checkpoint_numbers(
        envelope.workspace_key_version,
        envelope.checkpoint_counter,
        envelope.through_change_seq,
    )?;
    let (nonce, encrypted) = envelope.ciphertext.split_at(NONCE_BYTES);
    let cipher = XChaCha20Poly1305::new(workspace_key.into());
    let plaintext = cipher
        .decrypt(
            XNonce::from_slice(nonce),
            Payload {
                msg: encrypted,
                aad: &checkpoint_aad(
                    owner_id,
                    workspace_id,
                    envelope.workspace_key_version,
                    envelope.checkpoint_counter,
                    envelope.through_change_seq,
                ),
            },
        )
        .map_err(|_| {
            checkpoint_invalid("Checkpoint authentication failed for this workspace and key.")
        })?;
    if plaintext.len() != CHECKPOINT_PLAINTEXT_BYTES {
        return Err(checkpoint_invalid(
            "The authenticated checkpoint plaintext length is invalid.",
        ));
    }

    let protocol = u16::from_be_bytes(plaintext[0..2].try_into().expect("fixed slice"));
    let counter = u64::from_be_bytes(plaintext[2..10].try_into().expect("fixed slice"));
    let through = u64::from_be_bytes(plaintext[10..18].try_into().expect("fixed slice"));
    let stored_digest: [u8; 32] = plaintext[18..50].try_into().expect("fixed slice");
    let actual_digest = manifest_digest(manifest)?;
    if protocol != envelope.protocol_version
        || counter != envelope.checkpoint_counter
        || through != envelope.through_change_seq
        || stored_digest != actual_digest
    {
        return Err(checkpoint_invalid(
            "The checkpoint does not describe the complete supplied record manifest.",
        ));
    }
    Ok(VerifiedWorkspaceCheckpoint {
        checkpoint_counter: counter,
        through_change_seq: through,
        manifest_digest: stored_digest,
    })
}

impl Database {
    pub(crate) fn accept_sync_v2_checkpoint(
        &mut self,
        workspace_id: Uuid,
        checkpoint: &VerifiedWorkspaceCheckpoint,
    ) -> AppResult<CheckpointAcceptance> {
        if checkpoint.checkpoint_counter == 0
            || checkpoint.through_change_seq == 0
            || checkpoint.checkpoint_counter > i64::MAX as u64
            || checkpoint.through_change_seq > i64::MAX as u64
        {
            return Err(checkpoint_invalid(
                "The verified checkpoint cannot be represented by local durable storage.",
            ));
        }
        let transaction = self.connection.transaction().map_err(database_error)?;
        let existing = transaction
            .query_row(
                "SELECT checkpoint_counter, checkpoint_digest, through_change_seq FROM sync_v2_checkpoint_anchors WHERE workspace_id = ?1",
                [workspace_id.to_string()],
                |row| Ok((row.get::<_, i64>(0)?, row.get::<_, Vec<u8>>(1)?, row.get::<_, i64>(2)?)),
            )
            .optional()
            .map_err(database_error)?;
        if let Some((counter, digest, through)) = existing {
            let new_counter = checkpoint.checkpoint_counter as i64;
            if new_counter < counter {
                return Err(checkpoint_rollback(
                    "The checkpoint counter is lower than this computer's durable anchor.",
                ));
            }
            if new_counter == counter {
                if digest.as_slice() != checkpoint.manifest_digest
                    || through != checkpoint.through_change_seq as i64
                {
                    return Err(checkpoint_rollback(
                        "The checkpoint reused an accepted counter with different content.",
                    ));
                }
                return Ok(CheckpointAcceptance::AlreadyAccepted);
            }
            if checkpoint.through_change_seq as i64 <= through {
                return Err(checkpoint_rollback(
                    "A newer checkpoint did not advance the authenticated change cursor.",
                ));
            }
        }

        transaction
            .execute(
                "INSERT INTO sync_v2_checkpoint_anchors (workspace_id, protocol_version, checkpoint_counter, checkpoint_digest, through_change_seq, updated_at_utc) VALUES (?1, 1, ?2, ?3, ?4, strftime('%Y-%m-%dT%H:%M:%fZ','now')) ON CONFLICT(workspace_id) DO UPDATE SET checkpoint_counter = excluded.checkpoint_counter, checkpoint_digest = excluded.checkpoint_digest, through_change_seq = excluded.through_change_seq, updated_at_utc = excluded.updated_at_utc",
                params![
                    workspace_id.to_string(),
                    checkpoint.checkpoint_counter as i64,
                    checkpoint.manifest_digest.as_slice(),
                    checkpoint.through_change_seq as i64,
                ],
            )
            .map_err(database_error)?;
        transaction.commit().map_err(database_error)?;
        Ok(CheckpointAcceptance::Advanced)
    }
}

fn checkpoint_aad(
    owner_id: Uuid,
    workspace_id: Uuid,
    workspace_key_version: u32,
    checkpoint_counter: u64,
    through_change_seq: u64,
) -> Vec<u8> {
    let mut aad = Vec::with_capacity(CHECKPOINT_LABEL.len() + 16 + 16 + 2 + 4 + 8 + 8);
    aad.extend_from_slice(CHECKPOINT_LABEL);
    aad.extend_from_slice(owner_id.as_bytes());
    aad.extend_from_slice(workspace_id.as_bytes());
    aad.extend_from_slice(&PROTOCOL_VERSION.to_be_bytes());
    aad.extend_from_slice(&workspace_key_version.to_be_bytes());
    aad.extend_from_slice(&checkpoint_counter.to_be_bytes());
    aad.extend_from_slice(&through_change_seq.to_be_bytes());
    aad
}

fn validate_checkpoint_numbers(
    workspace_key_version: u32,
    checkpoint_counter: u64,
    through_change_seq: u64,
) -> AppResult<()> {
    if workspace_key_version == 0 || checkpoint_counter == 0 || through_change_seq == 0 {
        return Err(checkpoint_invalid(
            "Checkpoint key version, counter, and change cursor must be positive.",
        ));
    }
    Ok(())
}

fn checkpoint_invalid(detail: impl Into<String>) -> AppError {
    AppError::from_technical(
        "SYNC_V2_CHECKPOINT_INVALID",
        "The encrypted Sync checkpoint could not be verified.",
        "Keep Sync disabled. Your local workspace is unchanged.",
        detail,
    )
}

fn checkpoint_rollback(detail: impl Into<String>) -> AppError {
    AppError::from_technical(
        "SYNC_V2_CHECKPOINT_ROLLBACK_DETECTED",
        "SiteDatum refused an older or inconsistent Sync checkpoint.",
        "Keep Sync disabled and retain this computer's local workspace for recovery.",
        detail,
    )
}

fn checkpoint_crypto_error(detail: impl Into<String>) -> AppError {
    AppError::from_technical(
        "SYNC_V2_CHECKPOINT_ENCRYPTION_FAILED",
        "SiteDatum could not protect the Sync checkpoint.",
        "Keep Sync disabled and try again. Your local workspace is unchanged.",
        detail,
    )
}

fn database_error(error: rusqlite::Error) -> AppError {
    AppError::from_technical(
        "SYNC_V2_CHECKPOINT_STORAGE_FAILED",
        "SiteDatum could not save this computer's Sync checkpoint anchor.",
        "Keep Sync disabled and check the local workspace database.",
        error.to_string(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn manifest() -> Vec<RecordManifestEntry> {
        vec![
            RecordManifestEntry {
                record_id: Uuid::parse_str("91000000-0000-4000-8000-000000000002").unwrap(),
                record_revision: 4,
                workspace_key_version: 2,
                tombstone: true,
            },
            RecordManifestEntry {
                record_id: Uuid::parse_str("91000000-0000-4000-8000-000000000001").unwrap(),
                record_revision: 7,
                workspace_key_version: 2,
                tombstone: false,
            },
        ]
    }

    #[test]
    fn manifest_digest_is_order_independent_and_rejects_duplicates() {
        let entries = manifest();
        let mut reversed = entries.clone();
        reversed.reverse();
        assert_eq!(
            manifest_digest(&entries).unwrap(),
            manifest_digest(&reversed).unwrap()
        );
        let duplicate = vec![entries[0].clone(), entries[0].clone()];
        assert_eq!(
            manifest_digest(&duplicate).unwrap_err().code,
            "SYNC_V2_CHECKPOINT_INVALID"
        );
    }

    #[test]
    fn checkpoint_authenticates_manifest_and_routing_headers() {
        let owner = Uuid::parse_str("92000000-0000-4000-8000-000000000001").unwrap();
        let workspace = Uuid::parse_str("93000000-0000-4000-8000-000000000001").unwrap();
        let key = [0x51; 32];
        let entries = manifest();
        let sealed = seal_workspace_checkpoint(owner, workspace, 2, &key, 3, 44, &entries).unwrap();
        let verified =
            open_workspace_checkpoint(owner, workspace, &key, &sealed, &entries).unwrap();
        assert_eq!(verified.checkpoint_counter, 3);
        assert_eq!(verified.through_change_seq, 44);

        let mut omitted = entries.clone();
        omitted.pop();
        assert_eq!(
            open_workspace_checkpoint(owner, workspace, &key, &sealed, &omitted)
                .unwrap_err()
                .code,
            "SYNC_V2_CHECKPOINT_INVALID"
        );
        assert!(open_workspace_checkpoint(owner, Uuid::new_v4(), &key, &sealed, &entries).is_err());
        let mut substituted = sealed.clone();
        substituted.ciphertext[NONCE_BYTES] ^= 1;
        assert!(open_workspace_checkpoint(owner, workspace, &key, &substituted, &entries).is_err());
    }

    #[test]
    fn durable_anchor_rejects_rollback_and_equal_counter_substitution() {
        let workspace = Uuid::new_v4();
        let path = std::env::temp_dir().join(format!("sitedatum-c10-04a-{}.db", Uuid::new_v4()));
        {
            let mut database = Database::open(&path).unwrap();
            let first = VerifiedWorkspaceCheckpoint {
                checkpoint_counter: 4,
                through_change_seq: 12,
                manifest_digest: [0x11; 32],
            };
            assert_eq!(
                database
                    .accept_sync_v2_checkpoint(workspace, &first)
                    .unwrap(),
                CheckpointAcceptance::Advanced
            );
            assert_eq!(
                database
                    .accept_sync_v2_checkpoint(workspace, &first)
                    .unwrap(),
                CheckpointAcceptance::AlreadyAccepted
            );
        }
        {
            let mut reopened = Database::open(&path).unwrap();
            let older = VerifiedWorkspaceCheckpoint {
                checkpoint_counter: 3,
                through_change_seq: 11,
                manifest_digest: [0x10; 32],
            };
            assert_eq!(
                reopened
                    .accept_sync_v2_checkpoint(workspace, &older)
                    .unwrap_err()
                    .code,
                "SYNC_V2_CHECKPOINT_ROLLBACK_DETECTED"
            );
            let substitution = VerifiedWorkspaceCheckpoint {
                checkpoint_counter: 4,
                through_change_seq: 12,
                manifest_digest: [0x22; 32],
            };
            assert_eq!(
                reopened
                    .accept_sync_v2_checkpoint(workspace, &substitution)
                    .unwrap_err()
                    .code,
                "SYNC_V2_CHECKPOINT_ROLLBACK_DETECTED"
            );
            let advanced = VerifiedWorkspaceCheckpoint {
                checkpoint_counter: 5,
                through_change_seq: 13,
                manifest_digest: [0x33; 32],
            };
            assert_eq!(
                reopened
                    .accept_sync_v2_checkpoint(workspace, &advanced)
                    .unwrap(),
                CheckpointAcceptance::Advanced
            );
        }
        let _ = fs::remove_file(path);
    }
}

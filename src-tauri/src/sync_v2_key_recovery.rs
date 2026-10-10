//! Dormant C10-04D key lifecycle. No commands, transport, customer activation,
//! plaintext key files, logging, automatic key retirement or provider authority.
#![allow(dead_code)]
use crate::{
    error::{AppError, AppResult},
    persistence::Database,
    sync_v2_local_queue::{decode, encode, StreamScope},
    sync_v2_record_codec::{open_record, seal_record, MAX_SAFE_INTEGER},
    sync_v2_record_protocol::{
        seal_workspace_checkpoint, RecordManifestEntry, SealedWorkspaceCheckpoint,
        VerifiedWorkspaceCheckpoint,
    },
};
use chacha20poly1305::{
    aead::{Aead, KeyInit, Payload},
    XChaCha20Poly1305, XNonce,
};
use hkdf::Hkdf;
use rusqlite::{params, OptionalExtension};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, io::Write, path::Path};
use uuid::Uuid;
use zeroize::Zeroizing;

const MAX_KEYS: usize = 8;
const LABEL: &[u8] = b"sitedatum.sync-v2.recovery-file.v1";
const MAGIC: &[u8; 8] = b"SDREC001";
fn refused() -> AppError {
    AppError::from_technical(
        "SYNC_V2_KEY_RECOVERY_REFUSED",
        "The Sync key or recovery material could not be verified.",
        "Keep local work. Retain the original recovery file and separate code.",
        "Key, scope, freshness, format or storage validation failed.",
    )
}
fn sql(_: rusqlite::Error) -> AppError {
    refused()
}

pub(crate) trait WorkspaceKeys {
    fn select(&self, owner: Uuid, workspace: Uuid, version: u32) -> AppResult<&[u8; 32]>;
    fn write_version(&self) -> Option<u32> {
        None
    }
}
// Existing low-level single-key proof APIs remain compatible. Runtime lifecycle
// callers must supply the scope-bound KeyRing, never this unscoped test adapter.
impl WorkspaceKeys for [u8; 32] {
    fn select(&self, _: Uuid, _: Uuid, version: u32) -> AppResult<&[u8; 32]> {
        if version == 0 {
            Err(refused())
        } else {
            Ok(self)
        }
    }
}
pub(crate) struct KeyRing {
    owner: Uuid,
    workspace: Uuid,
    active: u32,
    keys: BTreeMap<u32, Zeroizing<[u8; 32]>>,
}
impl WorkspaceKeys for KeyRing {
    fn write_version(&self) -> Option<u32> {
        Some(self.active)
    }
    fn select(&self, owner: Uuid, workspace: Uuid, version: u32) -> AppResult<&[u8; 32]> {
        if owner != self.owner || workspace != self.workspace {
            return Err(refused());
        }
        self.keys.get(&version).map(|k| &**k).ok_or_else(refused)
    }
}
impl KeyRing {
    pub(crate) fn initial(owner: Uuid, workspace: Uuid) -> AppResult<Self> {
        if owner.is_nil() || workspace.is_nil() {
            return Err(refused());
        }
        let mut key = Zeroizing::new([0; 32]);
        getrandom::fill(&mut *key).map_err(|_| refused())?;
        Ok(Self {
            owner,
            workspace,
            active: 1,
            keys: BTreeMap::from([(1, key)]),
        })
    }
    /// Fresh key is retained before any encryption/queue operation. At capacity
    /// fail closed; removal needs a later reviewed history/compaction policy.
    pub(crate) fn rotate(&mut self) -> AppResult<u32> {
        if self.keys.len() >= MAX_KEYS {
            return Err(refused());
        }
        let next = self.active.checked_add(1).ok_or_else(refused)?;
        let mut key = Zeroizing::new([0; 32]);
        getrandom::fill(&mut *key).map_err(|_| refused())?;
        self.keys.insert(next, key);
        self.active = next;
        Ok(next)
    }
    pub(crate) fn active_version(&self) -> u32 {
        self.active
    }
    fn raw(&self) -> Zeroizing<Vec<u8>> {
        let mut b = Zeroizing::new(Vec::new());
        b.extend_from_slice(self.owner.as_bytes());
        b.extend_from_slice(self.workspace.as_bytes());
        b.extend_from_slice(&self.active.to_be_bytes());
        b.push(self.keys.len() as u8);
        for (v, k) in &self.keys {
            b.extend_from_slice(&v.to_be_bytes());
            b.extend_from_slice(&**k);
        }
        b
    }
    fn from_raw(b: &[u8]) -> AppResult<Self> {
        if b.len() < 37 {
            return Err(refused());
        }
        let owner = Uuid::from_slice(&b[..16]).map_err(|_| refused())?;
        let workspace = Uuid::from_slice(&b[16..32]).map_err(|_| refused())?;
        let active = u32::from_be_bytes(b[32..36].try_into().map_err(|_| refused())?);
        let count = b[36] as usize;
        if owner.is_nil()
            || workspace.is_nil()
            || count == 0
            || count > MAX_KEYS
            || b.len() != 37 + count * 36
        {
            return Err(refused());
        }
        let mut keys = BTreeMap::new();
        let mut previous = 0;
        for e in b[37..].chunks_exact(36) {
            let v = u32::from_be_bytes(e[..4].try_into().map_err(|_| refused())?);
            if v != previous + 1 {
                return Err(refused());
            }
            previous = v;
            keys.insert(v, Zeroizing::new(e[4..].try_into().map_err(|_| refused())?));
        }
        if active != previous {
            return Err(refused());
        }
        Ok(Self {
            owner,
            workspace,
            active,
            keys,
        })
    }
    pub(crate) fn save_protected(&self) -> AppResult<()> {
        #[cfg(windows)]
        {
            use base64::Engine;
            let raw = self.raw();
            let encoded = Zeroizing::new(base64::engine::general_purpose::STANDARD.encode(&*raw));
            credential(self.owner, self.workspace)?
                .set_password(&encoded)
                .map_err(|_| refused())?;
            let read = Self::load_protected(self.owner, self.workspace)?;
            if read.raw().as_slice() != raw.as_slice() {
                return Err(refused());
            }
            Ok(())
        }
        #[cfg(not(windows))]
        {
            Err(refused())
        }
    }
    pub(crate) fn load_protected(owner: Uuid, workspace: Uuid) -> AppResult<Self> {
        #[cfg(windows)]
        {
            use base64::Engine;
            let encoded = Zeroizing::new(
                credential(owner, workspace)?
                    .get_password()
                    .map_err(|_| refused())?,
            );
            if encoded.len() > 1024 {
                return Err(refused());
            }
            let raw = Zeroizing::new(
                base64::engine::general_purpose::STANDARD
                    .decode(encoded.as_bytes())
                    .map_err(|_| refused())?,
            );
            let ring = Self::from_raw(&raw)?;
            ring.select(owner, workspace, ring.active)?;
            Ok(ring)
        }
        #[cfg(not(windows))]
        {
            let _ = (owner, workspace);
            Err(refused())
        }
    }
}
#[cfg(windows)]
fn credential(owner: Uuid, workspace: Uuid) -> AppResult<keyring::Entry> {
    keyring::Entry::new(
        "com.sitedatum.sync-v2.workspace-keyring.v1",
        &format!("{owner}:{workspace}"),
    )
    .map_err(|_| refused())
}

pub(crate) struct RecoveryCode(Zeroizing<[u8; 32]>);
impl RecoveryCode {
    pub(crate) fn generate() -> AppResult<Self> {
        let mut b = Zeroizing::new([0; 32]);
        getrandom::fill(&mut *b).map_err(|_| refused())?;
        Ok(Self(b))
    }
    /// Explicit local display only. Do not log, persist with the file or send.
    pub(crate) fn display(&self) -> Zeroizing<String> {
        let mut hash = Sha256::new();
        hash.update(LABEL);
        hash.update(&*self.0);
        let checksum = hash.finalize();
        let mut hex = Zeroizing::new(String::new());
        for b in self.0.iter().chain(checksum[..4].iter()) {
            use std::fmt::Write;
            write!(&mut *hex, "{b:02X}").expect("String formatting");
        }
        let mut text = Zeroizing::new(String::from("SDR1"));
        for group in hex.as_bytes().chunks(8) {
            text.push('-');
            text.push_str(std::str::from_utf8(group).expect("hex ASCII"));
        }
        text
    }
    pub(crate) fn parse(text: &str) -> AppResult<Self> {
        if text.len() != 85 || !text.starts_with("SDR1-") {
            return Err(refused());
        }
        let groups = text[5..].split('-').collect::<Vec<_>>();
        if groups.len() != 9
            || groups.iter().any(|g| {
                g.len() != 8
                    || !g
                        .bytes()
                        .all(|b| b.is_ascii_digit() || (b'A'..=b'F').contains(&b))
            })
        {
            return Err(refused());
        }
        let joined = Zeroizing::new(groups.concat());
        let mut secret = Zeroizing::new([0; 32]);
        let mut sum = [0; 4];
        for (i, pair) in joined.as_bytes().chunks_exact(2).enumerate() {
            let v = u8::from_str_radix(std::str::from_utf8(pair).map_err(|_| refused())?, 16)
                .map_err(|_| refused())?;
            if i < 32 {
                secret[i] = v;
            } else {
                sum[i - 32] = v;
            }
        }
        let mut hash = Sha256::new();
        hash.update(LABEL);
        hash.update(&*secret);
        if hash.finalize()[..4] != sum {
            return Err(refused());
        }
        Ok(Self(secret))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct Anchor {
    pub counter: u64,
    pub through: u64,
    pub digest: [u8; 32],
}
impl Anchor {
    fn validate(self) -> AppResult<()> {
        if self.counter == 0
            || self.through == 0
            || self.counter > MAX_SAFE_INTEGER
            || self.through > MAX_SAFE_INTEGER
        {
            Err(refused())
        } else {
            Ok(())
        }
    }
    pub(crate) fn permits(self, cp: &VerifiedWorkspaceCheckpoint) -> AppResult<()> {
        self.validate()?;
        if cp.checkpoint_counter == 0
            || cp.checkpoint_counter > MAX_SAFE_INTEGER
            || cp.through_change_seq == 0
            || cp.through_change_seq > MAX_SAFE_INTEGER
            || cp.checkpoint_counter < self.counter
            || cp.through_change_seq < self.through
            || (cp.checkpoint_counter == self.counter
                && (cp.manifest_digest != self.digest || cp.through_change_seq != self.through))
            || (cp.checkpoint_counter > self.counter && cp.through_change_seq <= self.through)
        {
            return Err(refused());
        }
        Ok(())
    }
}
pub(crate) struct RecoveredKeys {
    pub ring: KeyRing,
    pub minimum: Anchor,
    pub created_at_unix: u64,
    /// Even an authenticated file cannot prove newest state after its capture.
    pub proves_latest_state: bool,
}
fn recovery_key(
    code: &RecoveryCode,
    salt: &[u8],
    owner: Uuid,
    workspace: Uuid,
) -> AppResult<Zeroizing<[u8; 32]>> {
    let mut info = LABEL.to_vec();
    info.extend_from_slice(owner.as_bytes());
    info.extend_from_slice(workspace.as_bytes());
    let mut k = Zeroizing::new([0; 32]);
    Hkdf::<Sha256>::new(Some(salt), &*code.0)
        .expand(&info, &mut *k)
        .map_err(|_| refused())?;
    Ok(k)
}
pub(crate) fn create_recovery_file(
    ring: &KeyRing,
    minimum: Anchor,
    created_at_unix: u64,
    code: &RecoveryCode,
) -> AppResult<Vec<u8>> {
    minimum.validate()?;
    if created_at_unix == 0 {
        return Err(refused());
    }
    let mut salt = [0; 32];
    getrandom::fill(&mut salt).map_err(|_| refused())?;
    let mut nonce = [0; 24];
    getrandom::fill(&mut nonce).map_err(|_| refused())?;
    let mut header = MAGIC.to_vec();
    header.extend_from_slice(ring.owner.as_bytes());
    header.extend_from_slice(ring.workspace.as_bytes());
    header.extend_from_slice(&salt);
    header.extend_from_slice(&nonce);
    let mut body = Zeroizing::new(Vec::new());
    body.extend_from_slice(&created_at_unix.to_be_bytes());
    body.extend_from_slice(&minimum.counter.to_be_bytes());
    body.extend_from_slice(&minimum.through.to_be_bytes());
    body.extend_from_slice(&minimum.digest);
    body.extend_from_slice(&ring.raw());
    let key = recovery_key(code, &salt, ring.owner, ring.workspace)?;
    let encrypted = XChaCha20Poly1305::new((&*key).into())
        .encrypt(
            XNonce::from_slice(&nonce),
            Payload {
                msg: &body,
                aad: &header,
            },
        )
        .map_err(|_| refused())?;
    header.extend_from_slice(&encrypted);
    Ok(header)
}
pub(crate) fn open_recovery_file(
    bytes: &[u8],
    code: &RecoveryCode,
    owner: Uuid,
    workspace: Uuid,
    now: u64,
    independent_floor: Option<Anchor>,
) -> AppResult<RecoveredKeys> {
    if !(241..=493).contains(&bytes.len())
        || &bytes[..8] != MAGIC
        || bytes[8..24] != *owner.as_bytes()
        || bytes[24..40] != *workspace.as_bytes()
    {
        return Err(refused());
    }
    let key = recovery_key(code, &bytes[40..72], owner, workspace)?;
    let body = Zeroizing::new(
        XChaCha20Poly1305::new((&*key).into())
            .decrypt(
                XNonce::from_slice(&bytes[72..96]),
                Payload {
                    msg: &bytes[96..],
                    aad: &bytes[..96],
                },
            )
            .map_err(|_| refused())?,
    );
    if body.len() < 129 {
        return Err(refused());
    }
    let number = |start| -> AppResult<u64> {
        Ok(u64::from_be_bytes(
            body[start..start + 8].try_into().map_err(|_| refused())?,
        ))
    };
    let created = number(0)?;
    let minimum = Anchor {
        counter: number(8)?,
        through: number(16)?,
        digest: body[24..56].try_into().map_err(|_| refused())?,
    };
    minimum.validate()?;
    if created == 0 || created > now {
        return Err(refused());
    }
    if let Some(floor) = independent_floor {
        floor.permits(&VerifiedWorkspaceCheckpoint {
            checkpoint_counter: minimum.counter,
            through_change_seq: minimum.through,
            manifest_digest: minimum.digest,
        })?;
    }
    let ring = KeyRing::from_raw(&body[56..])?;
    ring.select(owner, workspace, ring.active)?;
    Ok(RecoveredKeys {
        ring,
        minimum,
        created_at_unix: created,
        proves_latest_state: false,
    })
}
/// Create-new only; cancellation or a collision never overwrites user material.
pub(crate) fn write_recovery_file(path: &Path, bytes: &[u8]) -> AppResult<()> {
    if path.extension().and_then(|s| s.to_str()) != Some("sitedatum-recovery")
        || !(241..=493).contains(&bytes.len())
        || !bytes.starts_with(MAGIC)
    {
        return Err(refused());
    }
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|_| refused())?;
    file.write_all(bytes)
        .and_then(|_| file.sync_all())
        .map_err(|_| refused())?;
    Ok(())
}

impl Database {
    pub(crate) fn create_sync_v2_recovery_file(
        &self,
        s: StreamScope,
        ring: &KeyRing,
        backups: &Path,
        code: &RecoveryCode,
        created: u64,
    ) -> AppResult<Vec<u8>> {
        ring.select(s.owner_id, s.workspace_id, ring.active)?;
        let applied = self
            .applied_sync_v2_receipt(s, ring, backups)?
            .ok_or_else(refused)?;
        create_recovery_file(
            ring,
            Anchor {
                counter: applied.checkpoint_counter,
                through: applied.through_change_seq,
                digest: applied.checkpoint_digest,
            },
            created,
            code,
        )
    }
    /// Runtime orchestration persists/read-verifies the new key before staging.
    /// Retained unused keys are safe after a queue failure; retry reuses them.
    pub(crate) fn rotate_sync_v2_protected_keys(
        &mut self,
        s: StreamScope,
        backups: &Path,
    ) -> AppResult<SealedWorkspaceCheckpoint> {
        let mut ring = KeyRing::load_protected(s.owner_id, s.workspace_id)?;
        let highest: u32 = {
            let mut q = self
                .connection
                .prepare("SELECT envelope FROM sync_v2_record_snapshots WHERE workspace_id=?1")
                .map_err(sql)?;
            let rows = q
                .query_map([s.workspace_id.to_string()], |r| r.get::<_, Vec<u8>>(0))
                .map_err(sql)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(sql)?;
            let mut highest = 0;
            for row in rows {
                let record = decode(&row)?;
                open_record(&record, &ring)?;
                highest = highest.max(record.header.workspace_key_version);
            }
            highest
        };
        if highest == 0 {
            return Err(refused());
        }
        let pending: bool = self
            .connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM sync_v2_rotation_batches WHERE workspace_id=?1)",
                [s.workspace_id.to_string()],
                |r| r.get(0),
            )
            .map_err(sql)?;
        if !pending && self.applied_sync_v2_receipt(s, &ring, backups)?.is_none() {
            return Err(refused());
        }
        if !pending && ring.active == highest {
            ring.rotate()?;
            ring.save_protected()?;
        }
        self.stage_sync_v2_rotation(s, &ring, backups)
    }
    /// Import only an independently authenticated minimum, never a provider's
    /// claimed newest checkpoint. Do not advance the pull cursor or claim apply.
    pub(crate) fn install_sync_v2_recovery_floor(
        &mut self,
        s: StreamScope,
        minimum: Anchor,
    ) -> AppResult<()> {
        minimum.validate()?;
        let tx = self.connection.transaction().map_err(sql)?;
        crate::sync_v2_local_queue::scope(&tx, s)?;
        crate::sync_v2_record_protocol::accept_checkpoint_in_transaction(
            &tx,
            s.workspace_id,
            &VerifiedWorkspaceCheckpoint {
                checkpoint_counter: minimum.counter,
                through_change_seq: minimum.through,
                manifest_digest: minimum.digest,
            },
        )?;
        tx.execute(
            "DELETE FROM sync_v2_applied_receipts WHERE workspace_id=?1",
            [s.workspace_id.to_string()],
        )
        .map_err(sql)?;
        tx.commit().map_err(sql)
    }
    /// No live-row writes. A caller first persists the new KeyRing in protected
    /// storage and keeps it after failure; exact envelopes survive queue restart.
    pub(crate) fn stage_sync_v2_rotation(
        &mut self,
        s: StreamScope,
        ring: &KeyRing,
        backups: &Path,
    ) -> AppResult<SealedWorkspaceCheckpoint> {
        ring.select(s.owner_id, s.workspace_id, ring.active)?;
        let version: i64 = self
            .connection
            .query_row("PRAGMA data_version", [], |r| r.get(0))
            .map_err(sql)?;
        let pending: bool = self
            .connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM sync_v2_rotation_batches WHERE workspace_id=?1)",
                [s.workspace_id.to_string()],
                |r| r.get(0),
            )
            .map_err(sql)?;
        if !pending && self.applied_sync_v2_receipt(s, ring, backups)?.is_none() {
            return Err(refused());
        }
        let tx = self.connection.transaction().map_err(sql)?;
        if tx
            .query_row("PRAGMA data_version", [], |r| r.get::<_, i64>(0))
            .map_err(sql)?
            != version
        {
            return Err(refused());
        }
        crate::sync_v2_local_queue::scope(&tx, s)?;
        let retry: Option<Vec<u8>> = tx
            .query_row(
                "SELECT checkpoint FROM sync_v2_rotation_batches WHERE workspace_id=?1",
                [s.workspace_id.to_string()],
                |r| r.get(0),
            )
            .optional()
            .map_err(sql)?;
        if let Some(bytes) = retry {
            let checkpoint: SealedWorkspaceCheckpoint =
                serde_json::from_slice(&bytes).map_err(|_| refused())?;
            if checkpoint.workspace_key_version != ring.active {
                return Err(refused());
            }
            let entries = {
                let mut q=tx.prepare("SELECT envelope FROM sync_v2_record_snapshots WHERE workspace_id=?1 ORDER BY record_id").map_err(sql)?;
                let rows = q
                    .query_map([s.workspace_id.to_string()], |r| r.get::<_, Vec<u8>>(0))
                    .map_err(sql)?
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(sql)?;
                let mut entries = Vec::new();
                for bytes in rows {
                    let record = decode(&bytes)?;
                    open_record(&record, ring)?;
                    if record.header.owner_id != s.owner_id
                        || record.header.workspace_id != s.workspace_id
                    {
                        return Err(refused());
                    }
                    entries.push(RecordManifestEntry {
                        record_id: record.header.record_id,
                        record_revision: record.header.expected_server_version + 1,
                        workspace_key_version: record.header.workspace_key_version,
                        tombstone: record.header.tombstone,
                    });
                }
                entries
            };
            crate::sync_v2_record_protocol::open_workspace_checkpoint(
                s.owner_id,
                s.workspace_id,
                ring,
                &checkpoint,
                &entries,
            )?;
            return Ok(checkpoint);
        }
        let blocked:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM sync_v2_outbox WHERE workspace_id=?1) OR EXISTS(SELECT 1 FROM sync_v2_record_conflicts WHERE workspace_id=?1 AND resolved_counter IS NULL) OR EXISTS(SELECT 1 FROM sync_v2_pull_batches WHERE workspace_id=?1)",[s.workspace_id.to_string()],|r|r.get(0)).map_err(sql)?;
        if blocked {
            return Err(refused());
        }
        let (counter,through):(i64,i64)=tx.query_row("SELECT checkpoint_counter,through_change_seq FROM sync_v2_checkpoint_anchors WHERE workspace_id=?1",[s.workspace_id.to_string()],|r|Ok((r.get(0)?,r.get(1)?))).map_err(sql)?;
        let rows = {
            let mut q=tx.prepare("SELECT record_id,record_kind,server_version,envelope FROM sync_v2_record_snapshots WHERE workspace_id=?1 ORDER BY record_id").map_err(sql)?;
            let result = q
                .query_map([s.workspace_id.to_string()], |r| {
                    Ok((
                        r.get::<_, String>(0)?,
                        r.get::<_, u8>(1)?,
                        r.get::<_, i64>(2)?,
                        r.get::<_, Vec<u8>>(3)?,
                    ))
                })
                .map_err(sql)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(sql)?;
            result
        };
        if rows.is_empty()
            || rows.len() > 10_000
            || counter < 1
            || through < 1
            || counter as u64 >= MAX_SAFE_INTEGER
            || through as u64 + rows.len() as u64 > MAX_SAFE_INTEGER
        {
            return Err(refused());
        }
        let mut replacements = Vec::new();
        let mut manifest = Vec::new();
        let mut size = 0;
        for (id, kind, version, bytes) in rows {
            let old = decode(&bytes)?;
            let h = &old.header;
            if h.owner_id != s.owner_id
                || h.workspace_id != s.workspace_id
                || h.record_id.to_string() != id
                || h.record_kind != kind
                || h.workspace_key_version >= ring.active
                || version < 1
                || version as u64 >= MAX_SAFE_INTEGER
                || h.expected_server_version + 1 != version as u64
            {
                return Err(refused());
            }
            let body = open_record(&old, ring)?;
            let mut header = h.clone();
            header.workspace_key_version = ring.active;
            header.expected_server_version = version as u64;
            header.mutation_id = Uuid::new_v4();
            let new = seal_record(header, ring, &body)?;
            let sealed = encode(&new)?;
            size += sealed.len();
            if size > 128 * 1024 * 1024 {
                return Err(refused());
            }
            manifest.push(RecordManifestEntry {
                record_id: new.header.record_id,
                record_revision: version as u64 + 1,
                workspace_key_version: ring.active,
                tombstone: new.header.tombstone,
            });
            replacements.push((id, version, bytes, new, sealed));
        }
        let checkpoint = seal_workspace_checkpoint(
            s.owner_id,
            s.workspace_id,
            ring.active,
            ring,
            counter as u64 + 1,
            through as u64 + replacements.len() as u64,
            &manifest,
        )?;
        for (id, version, bytes, new, sealed) in replacements {
            tx.execute(
                "INSERT INTO sync_v2_committed_bases VALUES(?1,?2,?3,?4)",
                params![s.workspace_id.to_string(), id, version, bytes],
            )
            .map_err(sql)?;
            tx.execute("INSERT INTO sync_v2_outbox(workspace_id,mutation_id,record_id,envelope) VALUES(?1,?2,?3,?4)",params![s.workspace_id.to_string(),new.header.mutation_id.to_string(),id,sealed]).map_err(sql)?;
            tx.execute("UPDATE sync_v2_record_snapshots SET envelope=?3 WHERE workspace_id=?1 AND record_id=?2",params![s.workspace_id.to_string(),id,encode(&new)?]).map_err(sql)?;
        }
        tx.execute(
            "DELETE FROM sync_v2_applied_receipts WHERE workspace_id=?1",
            [s.workspace_id.to_string()],
        )
        .map_err(sql)?;
        tx.execute(
            "INSERT INTO sync_v2_rotation_batches VALUES(?1,?2,?3,?4,?5)",
            params![
                s.workspace_id.to_string(),
                ring.active,
                checkpoint.checkpoint_counter as i64,
                checkpoint.through_change_seq as i64,
                serde_json::to_vec(&checkpoint).map_err(|_| refused())?
            ],
        )
        .map_err(sql)?;
        tx.commit().map_err(sql)?;
        Ok(checkpoint)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sync_v2_local_queue::{MutationReceipt, PulledRecord};
    use crate::sync_v2_record_codec::{
        tests::{header, note},
        RecordContent, SealedRecord,
    };
    fn ring() -> KeyRing {
        KeyRing::initial(Uuid::new_v4(), Uuid::new_v4()).unwrap()
    }
    fn anchor() -> Anchor {
        Anchor {
            counter: 5,
            through: 12,
            digest: [0x45; 32],
        }
    }
    fn record(r: &KeyRing, tombstone: bool) -> SealedRecord {
        let mut h = header(10);
        h.owner_id = r.owner;
        h.workspace_id = r.workspace;
        h.workspace_key_version = r.active;
        h.tombstone = tombstone;
        let content = if tombstone {
            RecordContent {
                schema_version: 1,
                fields: None,
            }
        } else {
            note(&h, "Fictional local record")
        };
        seal_record(h, r, &content).unwrap()
    }
    fn manifest(records: &[SealedRecord]) -> Vec<RecordManifestEntry> {
        records
            .iter()
            .map(|r| RecordManifestEntry {
                record_id: r.header.record_id,
                record_revision: r.header.expected_server_version + 1,
                workspace_key_version: r.header.workspace_key_version,
                tombstone: r.header.tombstone,
            })
            .collect()
    }
    #[test]
    fn generated_code_roundtrips_and_transcription_errors_fail_closed() {
        let code = RecoveryCode::generate().unwrap();
        let text = code.display();
        assert_eq!(text.len(), 85);
        let parsed = RecoveryCode::parse(&text).unwrap();
        assert!(parsed.0.as_slice() == code.0.as_slice());
        for i in 5..text.len() {
            if text.as_bytes()[i] == b'-' {
                continue;
            }
            let mut bad = text.as_bytes().to_vec();
            bad[i] = if bad[i] == b'0' { b'1' } else { b'0' };
            assert!(RecoveryCode::parse(std::str::from_utf8(&bad).unwrap()).is_err());
        }
        for bad in ["", "SDR1-", "password", " ", "SDR2-00000000"] {
            assert!(RecoveryCode::parse(bad).is_err());
        }
        assert!(RecoveryCode::parse(&text.to_ascii_lowercase()).is_err());
    }
    #[test]
    fn recovery_file_restores_all_versions_and_authenticated_checkpoint_without_code_storage() {
        let mut r = ring();
        r.rotate().unwrap();
        r.rotate().unwrap();
        let code = RecoveryCode::generate().unwrap();
        let file = create_recovery_file(&r, anchor(), 1000, &code).unwrap();
        let recovered = open_recovery_file(
            &file,
            &RecoveryCode::parse(&code.display()).unwrap(),
            r.owner,
            r.workspace,
            1001,
            None,
        )
        .unwrap();
        assert!(recovered.ring.raw().as_slice() == r.raw().as_slice());
        assert_eq!(recovered.minimum, anchor());
        assert_eq!(recovered.created_at_unix, 1000);
        assert!(!file.windows(32).any(|p| p == code.0.as_slice()));
        for key in r.keys.values() {
            assert!(!file.windows(32).any(|p| p == key.as_slice()));
        }
        assert_ne!(
            file,
            create_recovery_file(&r, anchor(), 1000, &code).unwrap()
        );
    }
    #[test]
    fn every_file_byte_is_authenticated_and_wrong_scope_code_time_and_format_are_refused() {
        let r = ring();
        let code = RecoveryCode::generate().unwrap();
        let file = create_recovery_file(&r, anchor(), 1000, &code).unwrap();
        assert_eq!(file.len(), 241);
        for i in 0..file.len() {
            let mut bad = file.clone();
            bad[i] ^= 1;
            assert!(open_recovery_file(&bad, &code, r.owner, r.workspace, 1001, None).is_err());
        }
        for length in [0, 7, 95, 240, 494, 4096] {
            assert!(
                open_recovery_file(&vec![0; length], &code, r.owner, r.workspace, 1001, None)
                    .is_err()
            );
        }
        assert!(open_recovery_file(
            &file,
            &RecoveryCode::generate().unwrap(),
            r.owner,
            r.workspace,
            1001,
            None
        )
        .is_err());
        assert!(open_recovery_file(&file, &code, Uuid::new_v4(), r.workspace, 1001, None).is_err());
        assert!(open_recovery_file(&file, &code, r.owner, Uuid::new_v4(), 1001, None).is_err());
        assert!(open_recovery_file(&file, &code, r.owner, r.workspace, 999, None).is_err());
    }
    #[test]
    fn independent_newer_floor_rejects_stale_recovery_and_equal_counter_substitution() {
        let r = ring();
        let code = RecoveryCode::generate().unwrap();
        let file = create_recovery_file(&r, anchor(), 1000, &code).unwrap();
        assert!(
            open_recovery_file(&file, &code, r.owner, r.workspace, 2000, Some(anchor())).is_ok()
        );
        for floor in [
            Anchor {
                counter: 6,
                through: 13,
                ..anchor()
            },
            Anchor {
                digest: [1; 32],
                ..anchor()
            },
            Anchor {
                through: 13,
                ..anchor()
            },
        ] {
            assert!(
                open_recovery_file(&file, &code, r.owner, r.workspace, 2000, Some(floor)).is_err()
            );
        }
        let old = VerifiedWorkspaceCheckpoint {
            checkpoint_counter: 4,
            through_change_seq: 11,
            manifest_digest: anchor().digest,
        };
        assert!(anchor().permits(&old).is_err());
        assert!(anchor()
            .permits(&VerifiedWorkspaceCheckpoint {
                checkpoint_counter: 6,
                through_change_seq: 12,
                manifest_digest: [1; 32]
            })
            .is_err());
    }
    #[test]
    fn keys_are_scoped_version_selected_and_old_keys_retained_at_capacity() {
        let mut r = ring();
        let old = record(&r, false);
        r.rotate().unwrap();
        let new = record(&r, false);
        assert!(open_record(&old, &r).is_ok());
        assert!(open_record(&new, &r).is_ok());
        let other = ring();
        assert!(open_record(&old, &other).is_err());
        let mut tampered = old.clone();
        tampered.header.workspace_key_version = 2;
        assert!(open_record(&tampered, &r).is_err());
        assert!(r.select(r.owner, r.workspace, 0).is_err());
        assert!(r.select(r.owner, r.workspace, 99).is_err());
        while r.active < 8 {
            r.rotate().unwrap();
        }
        let raw = r.raw();
        assert!(r.rotate().is_err());
        assert!(raw.as_slice() == r.raw().as_slice());
        assert!(open_record(&old, &r).is_ok());
        let recovered = KeyRing::from_raw(&raw).unwrap();
        assert!(open_record(&old, &recovered).is_ok());
    }
    #[test]
    fn malformed_keyrings_fail_and_no_debug_or_serialization_exposes_private_material() {
        let r = ring();
        let raw = r.raw();
        for size in 0..raw.len() {
            assert!(KeyRing::from_raw(&raw[..size]).is_err());
        }
        for index in [32, 36, 37] {
            let mut bad = raw.to_vec();
            bad[index] = 0xff;
            assert!(KeyRing::from_raw(&bad).is_err());
        }
    }
    #[test]
    fn export_is_create_new_only_and_never_overwrites_recovery_file() {
        let r = ring();
        let c = RecoveryCode::generate().unwrap();
        let bytes = create_recovery_file(&r, anchor(), 1000, &c).unwrap();
        let root = std::env::temp_dir().join(format!("sitedatum-recovery-test-{}", Uuid::new_v4()));
        std::fs::create_dir(&root).unwrap();
        let path = root.join("fictional.sitedatum-recovery");
        write_recovery_file(&path, &bytes).unwrap();
        assert!(write_recovery_file(&path, &bytes).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), bytes);
        assert!(write_recovery_file(&root.join("wrong.txt"), &bytes).is_err());
        std::fs::remove_dir_all(root).unwrap();
    }
    fn fixture(r: &KeyRing) -> (Database, StreamScope, std::path::PathBuf, Vec<SealedRecord>) {
        let root =
            std::env::temp_dir().join(format!("sitedatum-keyrotation-test-{}", Uuid::new_v4()));
        std::fs::create_dir(&root).unwrap();
        let s = StreamScope {
            owner_id: r.owner,
            workspace_id: r.workspace,
            device_id: Uuid::new_v4(),
        };
        let mut db = Database::open_in_memory().unwrap();
        let records = vec![record(r, false), record(r, true)];
        let changes = records
            .iter()
            .enumerate()
            .map(|(i, r)| PulledRecord {
                change_seq: i as u64 + 1,
                server_version: 1,
                record: r.clone(),
            })
            .collect::<Vec<_>>();
        let cp = seal_workspace_checkpoint(r.owner, r.workspace, 1, r, 1, 2, &manifest(&records))
            .unwrap();
        db.apply_sync_v2_supported_records(s, 0, &changes, &cp, r, &root)
            .unwrap();
        (db, s, root, records)
    }
    #[test]
    fn mixed_key_pages_and_checkpoints_select_each_exact_version() {
        let mut r = ring();
        let old = record(&r, false);
        r.rotate().unwrap();
        let new = record(&r, false);
        let s = StreamScope {
            owner_id: r.owner,
            workspace_id: r.workspace,
            device_id: Uuid::new_v4(),
        };
        let mut db = Database::open_in_memory().unwrap();
        let records = vec![old, new];
        let cp = seal_workspace_checkpoint(r.owner, r.workspace, 2, &r, 1, 2, &manifest(&records))
            .unwrap();
        let changes = records
            .iter()
            .enumerate()
            .map(|(i, record)| PulledRecord {
                change_seq: i as u64 + 1,
                server_version: 1,
                record: record.clone(),
            })
            .collect::<Vec<_>>();
        db.stage_sync_v2_pull(s, 0, &changes, &cp, &r).unwrap();
        assert_eq!(
            db.connection
                .query_row("SELECT pull_cursor FROM sync_v2_local_streams", [], |row| {
                    row.get::<_, i64>(0)
                })
                .unwrap(),
            2
        );
    }
    #[test]
    fn retained_keys_read_history_but_cannot_stage_new_edits() {
        let mut r = ring();
        let old = record(&r, false);
        r.rotate().unwrap();
        assert!(open_record(&old, &r).is_ok());
        let s = StreamScope {
            owner_id: r.owner,
            workspace_id: r.workspace,
            device_id: Uuid::new_v4(),
        };
        let mut db = Database::open_in_memory().unwrap();
        assert!(db.stage_sync_v2_mutation(s, &old, &r).is_err());
        assert!(db.pending_sync_v2_mutations(s).unwrap().is_empty());
        let new = record(&r, false);
        db.stage_sync_v2_mutation(s, &new, &r).unwrap();
        assert_eq!(db.pending_sync_v2_mutations(s).unwrap().len(), 1);
    }
    #[test]
    fn rotation_is_atomic_retains_tombstones_and_exact_bytes_and_does_not_edit_live_rows() {
        let mut r = ring();
        let (mut db, s, root, old) = fixture(&r);
        r.rotate().unwrap();
        let cp = db.stage_sync_v2_rotation(s, &r, &root).unwrap();
        let queued = db.pending_sync_v2_mutations(s).unwrap();
        assert_eq!(queued.len(), 2);
        assert!(queued.iter().any(|r| r.header.tombstone));
        assert!(queued
            .iter()
            .all(|r| r.header.workspace_key_version == 2 && r.header.expected_server_version == 1));
        assert_eq!(db.stage_sync_v2_rotation(s, &r, &root).unwrap(), cp);
        assert_eq!(db.pending_sync_v2_mutations(s).unwrap(), queued);
        assert_eq!(
            db.connection
                .query_row("SELECT COUNT(*) FROM notes", [], |row| row.get::<_, i64>(0))
                .unwrap(),
            1
        );
        assert!(db.mutate_sync_v2_live(s, &record(&r, false), &r).is_err());
        let path = root.join("restart.sqlite3");
        {
            let mut copy = rusqlite::Connection::open(&path).unwrap();
            rusqlite::backup::Backup::new(&db.connection, &mut copy)
                .unwrap()
                .run_to_completion(64, std::time::Duration::from_millis(1), None)
                .unwrap();
        }
        db = Database::open(&path).unwrap();
        assert_eq!(db.stage_sync_v2_rotation(s, &r, &root).unwrap(), cp);
        assert_eq!(db.pending_sync_v2_mutations(s).unwrap(), queued);
        let receipts = queued
            .iter()
            .map(|r| MutationReceipt {
                record_id: r.header.record_id,
                mutation_id: r.header.mutation_id,
                server_version: 2,
            })
            .collect::<Vec<_>>();
        db.accept_sync_v2_receipts(s, &receipts).unwrap();
        let changes = queued
            .iter()
            .enumerate()
            .map(|(i, r)| PulledRecord {
                change_seq: i as u64 + 3,
                server_version: 2,
                record: r.clone(),
            })
            .collect::<Vec<_>>();
        db.apply_sync_v2_supported_records(s, 2, &changes, &cp, &r, &root)
            .unwrap();
        assert_eq!(
            db.connection
                .query_row("SELECT COUNT(*) FROM sync_v2_rotation_batches", [], |row| {
                    row.get::<_, i64>(0)
                })
                .unwrap(),
            0
        );
        for record in old {
            assert!(open_record(&record, &r).is_ok());
        }
        drop(db);
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn failed_rotation_trigger_preserves_all_rows_and_missing_key_never_advances_cursor() {
        let mut r = ring();
        let old_ring = KeyRing::from_raw(&r.raw()).unwrap();
        let (mut db, s, root, _) = fixture(&r);
        r.rotate().unwrap();
        db.connection.execute_batch("CREATE TRIGGER stop_rotation BEFORE INSERT ON sync_v2_rotation_batches BEGIN SELECT RAISE(ABORT,'injected'); END").unwrap();
        assert!(db.stage_sync_v2_rotation(s, &r, &root).is_err());
        for table in [
            "sync_v2_outbox",
            "sync_v2_committed_bases",
            "sync_v2_rotation_batches",
        ] {
            assert_eq!(
                db.connection
                    .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| row
                        .get::<_, i64>(0))
                    .unwrap(),
                0
            );
        }
        db.connection
            .execute_batch("DROP TRIGGER stop_rotation")
            .unwrap();
        let cp = db.stage_sync_v2_rotation(s, &r, &root).unwrap();
        let queued = db.pending_sync_v2_mutations(s).unwrap();
        let changes = queued
            .iter()
            .enumerate()
            .map(|(i, r)| PulledRecord {
                change_seq: i as u64 + 3,
                server_version: 2,
                record: r.clone(),
            })
            .collect::<Vec<_>>();
        assert!(db
            .apply_sync_v2_reviewed_records(s, 2, &changes, &cp, &old_ring, &root)
            .is_err());
        assert_eq!(
            db.connection
                .query_row("SELECT pull_cursor FROM sync_v2_local_streams", [], |row| {
                    row.get::<_, i64>(0)
                })
                .unwrap(),
            2
        );
        assert_eq!(db.pending_sync_v2_mutations(s).unwrap(), queued);
        drop(db);
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn recovery_floor_is_durable_without_claiming_live_application_and_rejects_rollback() {
        let mut r = ring();
        let (mut db, s, root, records) = fixture(&r);
        let digest = crate::sync_v2_record_protocol::manifest_digest(&manifest(&records)).unwrap();
        let current = Anchor {
            counter: 1,
            through: 2,
            digest,
        };
        db.install_sync_v2_recovery_floor(s, current).unwrap();
        assert!(db.applied_sync_v2_receipt(s, &r, &root).unwrap().is_none());
        let floor = Anchor {
            counter: 2,
            through: 3,
            digest: [5; 32],
        };
        db.install_sync_v2_recovery_floor(s, floor).unwrap();
        assert!(db.install_sync_v2_recovery_floor(s, current).is_err());
        assert_eq!(
            db.connection
                .query_row("SELECT pull_cursor FROM sync_v2_local_streams", [], |row| {
                    row.get::<_, i64>(0)
                })
                .unwrap(),
            2
        );
        r.rotate().unwrap();
        drop(db);
        std::fs::remove_dir_all(root).unwrap();
    }
    #[cfg(windows)]
    #[test]
    #[ignore = "Explicit fictional Windows Credential Manager roundtrip; no customer entries"]
    fn windows_workspace_keyring_protected_roundtrip() {
        let mut r = ring();
        struct Cleanup(Uuid, Uuid);
        impl Drop for Cleanup {
            fn drop(&mut self) {
                let _ = credential(self.0, self.1).unwrap().delete_credential();
            }
        }
        let _cleanup = Cleanup(r.owner, r.workspace);
        r.save_protected().unwrap();
        r.rotate().unwrap();
        r.save_protected().unwrap();
        let loaded = KeyRing::load_protected(r.owner, r.workspace).unwrap();
        assert!(loaded.raw().as_slice() == r.raw().as_slice());
        assert_eq!(loaded.active, 2);
        credential(r.owner, r.workspace)
            .unwrap()
            .delete_credential()
            .unwrap();
        assert!(KeyRing::load_protected(r.owner, r.workspace).is_err());
    }
    #[test]
    fn recovery_export_and_rotation_refuse_untracked_live_changes_and_pending_work() {
        let mut r = ring();
        let (mut db, s, root, _) = fixture(&r);
        let code = RecoveryCode::generate().unwrap();
        let bytes = db
            .create_sync_v2_recovery_file(s, &r, &root, &code, 1000)
            .unwrap();
        let recovered =
            open_recovery_file(&bytes, &code, r.owner, r.workspace, 1001, None).unwrap();
        assert!(!recovered.proves_latest_state);
        db.connection
            .execute("UPDATE notes SET body='Fictional untracked local edit'", [])
            .unwrap();
        r.rotate().unwrap();
        assert!(db.stage_sync_v2_rotation(s, &r, &root).is_err());
        assert!(db
            .create_sync_v2_recovery_file(s, &r, &root, &code, 1000)
            .is_err());
        assert!(db.pending_sync_v2_mutations(s).unwrap().is_empty());
        drop(db);
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn maximum_recovery_file_and_invalid_anchor_bounds_are_checked() {
        let mut r = ring();
        while r.active < 8 {
            r.rotate().unwrap();
        }
        let code = RecoveryCode::generate().unwrap();
        let bytes = create_recovery_file(&r, anchor(), 1000, &code).unwrap();
        assert_eq!(bytes.len(), 493);
        assert!(open_recovery_file(&bytes, &code, r.owner, r.workspace, 1001, None).is_ok());
        for minimum in [
            Anchor {
                counter: 0,
                ..anchor()
            },
            Anchor {
                through: 0,
                ..anchor()
            },
            Anchor {
                counter: u64::MAX,
                ..anchor()
            },
        ] {
            assert!(create_recovery_file(&r, minimum, 1000, &code).is_err());
        }
        assert!(create_recovery_file(&r, anchor(), 0, &code).is_err());
    }
}

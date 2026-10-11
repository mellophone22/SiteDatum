//! D3 experimental file vault; tests or explicit dormant review feature only.
//! No commands, transport, existing credentials, hot-ring mutation or pruning.
//! Pins are independently supplied fixture expectations, NOT freshness discovery.
#![allow(dead_code)] // Deliberately unreachable from the application.

use chacha20poly1305::{
    aead::{Aead, KeyInit, Payload},
    XChaCha20Poly1305, XNonce,
};
use hkdf::Hkdf;
use sha2::{Digest, Sha256};
use std::{
    fs::{File, OpenOptions},
    io::{Read, Write},
    path::Path,
};
use uuid::Uuid;
use zeroize::Zeroizing;

const LIMIT: usize = 1000; // Experimental budget, not a product promise.
const HEADER: usize = 76;
const MAX_FILE: usize = HEADER + LIMIT * 36 + 16;
const LOCAL: &[u8; 8] = b"SDPV0001";
const RECOVERY: &[u8; 8] = b"SDPR0001";

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Refusal {
    Invalid,
    Storage,
    Incomplete,
}
type Result<T> = std::result::Result<T, Refusal>;

impl crate::sync_v2_key_recovery::WorkspaceKeys for Archive {
    fn select(&self, o: Uuid, w: Uuid, v: u32) -> crate::error::AppResult<&[u8; 32]> {
        Archive::select(self, o, w, v).map_err(|_| {
            crate::error::AppError::from_technical(
                "SYNC_V2_VAULT_PROTOTYPE_REFUSED",
                "Fictional archive verification refused.",
                "Keep all original material. This prototype cannot authorize retirement.",
                "Scope or exact-version mismatch.",
            )
        })
    }
    fn write_version(&self) -> Option<u32> {
        None
    }
}

// Neither secret nor archive implements Debug, Serialize, Clone or write authority.
pub(super) struct Secret(Zeroizing<[u8; 32]>);
impl Secret {
    pub(super) fn recovery_text(&self) -> Zeroizing<String> {
        use base64::Engine;
        Zeroizing::new(base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(self.0.as_ref()))
    }
    pub(super) fn from_recovery_text(text: &str) -> Result<Self> {
        use base64::Engine;
        if text.len() != 43 || !text.is_ascii() {
            return Err(Refusal::Invalid);
        }
        let raw = Zeroizing::new(
            base64::engine::general_purpose::URL_SAFE_NO_PAD
                .decode(text)
                .map_err(|_| Refusal::Invalid)?,
        );
        if raw.len() != 32 {
            return Err(Refusal::Invalid);
        }
        let mut value = Zeroizing::new([0; 32]);
        value.copy_from_slice(&raw);
        let secret = Self(value);
        if secret.recovery_text().as_str() != text {
            return Err(Refusal::Invalid);
        }
        Ok(secret)
    }
    pub(super) fn random() -> Result<Self> {
        let mut bytes = Zeroizing::new([0; 32]);
        getrandom::fill(bytes.as_mut()).map_err(|_| Refusal::Storage)?;
        Ok(Self(bytes))
    }
}
pub(super) struct Archive {
    owner: Uuid,
    workspace: Uuid,
    generation: u64,
    keys: Vec<(u32, Zeroizing<[u8; 32]>)>,
}
#[derive(Clone, Copy)]
pub(super) struct Pin {
    pub(super) generation: u64,
    pub(super) digest: [u8; 32],
}

impl Archive {
    pub(super) fn retained_manifest(&self) -> [u8; 32] {
        // In-process full-set comparison only; never logged/published as authority.
        let mut h = Sha256::new();
        h.update(b"sitedatum.prototype.d3.retained-manifest.v1");
        h.update(self.owner.as_bytes());
        h.update(self.workspace.as_bytes());
        h.update(self.generation.to_be_bytes());
        h.update((self.keys.len() as u32).to_be_bytes());
        for (v, key) in &self.keys {
            h.update(v.to_be_bytes());
            h.update(key.as_ref());
        }
        h.finalize().into()
    }
    pub(super) fn context(&self) -> (Uuid, Uuid, u64) {
        (self.owner, self.workspace, self.generation)
    }
    pub(super) fn new(
        owner: Uuid,
        workspace: Uuid,
        generation: u64,
        keys: Vec<(u32, Zeroizing<[u8; 32]>)>,
    ) -> Result<Self> {
        if owner.is_nil()
            || workspace.is_nil()
            || generation == 0
            || generation > crate::sync_v2_record_codec::MAX_SAFE_INTEGER
            || keys.is_empty()
            || keys.len() > LIMIT
        {
            return Err(Refusal::Invalid);
        }
        let mut previous = 0;
        for (version, _) in &keys {
            if *version <= previous || *version > i32::MAX as u32 {
                return Err(Refusal::Invalid);
            }
            previous = *version;
        }
        Ok(Self {
            owner,
            workspace,
            generation,
            keys,
        })
    }
    pub(super) fn select(&self, owner: Uuid, workspace: Uuid, version: u32) -> Result<&[u8; 32]> {
        if owner != self.owner || workspace != self.workspace {
            return Err(Refusal::Invalid);
        }
        self.keys
            .binary_search_by_key(&version, |(v, _)| *v)
            .map(|i| &*self.keys[i].1)
            .map_err(|_| Refusal::Invalid)
    }
    pub(super) fn seal(&self, secret: &Secret, recovery: bool) -> Result<(Vec<u8>, Pin)> {
        let mut header = Vec::with_capacity(HEADER);
        header.extend_from_slice(if recovery { RECOVERY } else { LOCAL });
        header.extend_from_slice(self.owner.as_bytes());
        header.extend_from_slice(self.workspace.as_bytes());
        header.extend_from_slice(&self.generation.to_be_bytes());
        header.extend_from_slice(&(self.keys.len() as u32).to_be_bytes());
        let mut nonce = [0; 24];
        getrandom::fill(&mut nonce).map_err(|_| Refusal::Storage)?;
        header.extend_from_slice(&nonce);
        let mut raw = Zeroizing::new(Vec::with_capacity(self.keys.len() * 36));
        for (v, key) in &self.keys {
            raw.extend_from_slice(&v.to_be_bytes());
            raw.extend_from_slice(key.as_ref());
        }
        let key = derive(secret, &header, recovery)?;
        let encrypted = XChaCha20Poly1305::new_from_slice(key.as_ref())
            .map_err(|_| Refusal::Invalid)?
            .encrypt(
                XNonce::from_slice(&nonce),
                Payload {
                    msg: &raw,
                    aad: &header,
                },
            )
            .map_err(|_| Refusal::Invalid)?;
        let mut file = Vec::with_capacity(HEADER + encrypted.len());
        file.extend_from_slice(&header);
        file.extend_from_slice(&encrypted);
        let pin = Pin {
            generation: self.generation,
            digest: Sha256::digest(&file).into(),
        };
        Ok((file, pin))
    }
    pub(super) fn open(
        bytes: &[u8],
        secret: &Secret,
        recovery: bool,
        owner: Uuid,
        workspace: Uuid,
        expected: Pin,
    ) -> Result<Self> {
        if !(HEADER + 36 + 16..=MAX_FILE).contains(&bytes.len()) {
            return Err(Refusal::Invalid);
        }
        let h = &bytes[..HEADER];
        let generation = u64::from_be_bytes(h[40..48].try_into().map_err(|_| Refusal::Invalid)?);
        let count =
            u32::from_be_bytes(h[48..52].try_into().map_err(|_| Refusal::Invalid)?) as usize;
        if &h[..8] != if recovery { RECOVERY } else { LOCAL }
            || &h[8..24] != owner.as_bytes()
            || &h[24..40] != workspace.as_bytes()
            || count == 0
            || count > LIMIT
            || bytes.len() != HEADER + count * 36 + 16
            || generation != expected.generation
            || <[u8; 32]>::from(Sha256::digest(bytes)) != expected.digest
        {
            return Err(Refusal::Invalid);
        }
        let key = derive(secret, h, recovery)?;
        let raw = Zeroizing::new(
            XChaCha20Poly1305::new_from_slice(key.as_ref())
                .map_err(|_| Refusal::Invalid)?
                .decrypt(
                    XNonce::from_slice(&h[52..76]),
                    Payload {
                        msg: &bytes[HEADER..],
                        aad: h,
                    },
                )
                .map_err(|_| Refusal::Invalid)?,
        );
        let mut keys = Vec::with_capacity(count);
        for entry in raw.chunks_exact(36) {
            keys.push((
                u32::from_be_bytes(entry[..4].try_into().map_err(|_| Refusal::Invalid)?),
                Zeroizing::new(entry[4..].try_into().map_err(|_| Refusal::Invalid)?),
            ));
        }
        Self::new(owner, workspace, generation, keys)
    }
}
fn derive(secret: &Secret, header: &[u8], recovery: bool) -> Result<Zeroizing<[u8; 32]>> {
    let mut derived = Zeroizing::new([0; 32]);
    Hkdf::<Sha256>::new(Some(header), secret.0.as_ref())
        .expand(
            if recovery {
                b"sitedatum.prototype.d3.recovery.v1"
            } else {
                b"sitedatum.prototype.d3.local.v1"
            },
            derived.as_mut(),
        )
        .map_err(|_| Refusal::Invalid)?;
    Ok(derived)
}
pub(super) fn read_bounded(path: &Path) -> Result<Vec<u8>> {
    let mut f = File::open(path).map_err(|_| Refusal::Storage)?;
    if f.metadata().map_err(|_| Refusal::Storage)?.len() > MAX_FILE as u64 {
        return Err(Refusal::Invalid);
    }
    let mut bytes = Vec::with_capacity(MAX_FILE + 1);
    (&mut f)
        .take((MAX_FILE + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|_| Refusal::Storage)?;
    if bytes.len() > MAX_FILE {
        return Err(Refusal::Invalid);
    }
    Ok(bytes)
}
pub(super) fn write_new(path: &Path, bytes: &[u8]) -> Result<()> {
    if !(HEADER + 52..=MAX_FILE).contains(&bytes.len()) {
        return Err(Refusal::Invalid);
    }
    let mut f = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|_| Refusal::Storage)?;
    f.write_all(bytes)
        .and_then(|_| f.sync_all())
        .map_err(|_| Refusal::Incomplete)?;
    if read_bounded(path).map_err(|_| Refusal::Incomplete)? != bytes {
        return Err(Refusal::Incomplete);
    }
    // Failed new files remain for inspection; no automatic deletion or replacement.
    Ok(())
}

#[cfg(windows)]
fn dpapi(input: &[u8], protect: bool) -> Result<Zeroizing<Vec<u8>>> {
    use windows_sys::Win32::{
        Foundation::LocalFree,
        Security::Cryptography::{
            CryptProtectData, CryptUnprotectData, CRYPTPROTECT_UI_FORBIDDEN, CRYPT_INTEGER_BLOB,
        },
    };
    use zeroize::Zeroize;
    if input.is_empty() || input.len() > 8192 {
        return Err(Refusal::Invalid);
    }
    let source = CRYPT_INTEGER_BLOB {
        cbData: input.len() as u32,
        pbData: input.as_ptr() as *mut u8,
    };
    let mut output = CRYPT_INTEGER_BLOB {
        cbData: 0,
        pbData: std::ptr::null_mut(),
    };
    // SAFETY: all buffers live across the synchronous call; optional pointers null.
    // User scope, no LOCAL_MACHINE flag, no prompt, no description or customer text.
    let success = unsafe {
        if protect {
            CryptProtectData(
                &source,
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null(),
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut output,
            )
        } else {
            CryptUnprotectData(
                &source,
                std::ptr::null_mut(),
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null(),
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut output,
            )
        }
    };
    if success == 0 {
        return Err(Refusal::Storage);
    }
    // SAFETY: successful DPAPI returns a LocalAlloc-owned buffer of cbData bytes.
    // Copy only within the bounded OS result; wipe plaintext before freeing it.
    unsafe {
        if output.pbData.is_null() || output.cbData == 0 || output.cbData > 8192 {
            if !output.pbData.is_null() {
                LocalFree(output.pbData as _);
            }
            return Err(Refusal::Invalid);
        }
        let slice = std::slice::from_raw_parts_mut(output.pbData, output.cbData as usize);
        let copied = Zeroizing::new(slice.to_vec());
        slice.zeroize();
        LocalFree(output.pbData as _);
        Ok(copied)
    }
}

#[cfg(windows)]
impl Secret {
    pub(super) fn wrap_local(&self) -> Result<Zeroizing<Vec<u8>>> {
        dpapi(self.0.as_ref(), true)
    }
    pub(super) fn unwrap_local(wrapped: &[u8]) -> Result<Self> {
        let plain = dpapi(wrapped, false)?;
        if plain.len() != 32 {
            return Err(Refusal::Invalid);
        }
        let mut secret = Zeroizing::new([0; 32]);
        secret.copy_from_slice(&plain);
        Ok(Self(secret))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn archive(count: usize, generation: u64, o: Uuid, w: Uuid) -> Archive {
        Archive::new(
            o,
            w,
            generation,
            (1..=count)
                .map(|v| (v as u32, Secret::random().unwrap().0))
                .collect(),
        )
        .unwrap()
    }
    #[test]
    fn corruption_wrong_secret_scope_and_kind_refuse_even_with_recomputed_hash() {
        let o = Uuid::new_v4();
        let w = Uuid::new_v4();
        let a = archive(1, 1, o, w);
        let secret = Secret::random().unwrap();
        let (bytes, pin) = a.seal(&secret, false).unwrap();
        for i in 0..bytes.len() {
            let mut changed = bytes.clone();
            changed[i] ^= 1;
            let altered_pin = Pin {
                digest: Sha256::digest(&changed).into(),
                ..pin
            };
            assert!(Archive::open(&changed, &secret, false, o, w, altered_pin).is_err());
        }
        assert!(Archive::open(&bytes, &Secret::random().unwrap(), false, o, w, pin).is_err());
        assert!(Archive::open(&bytes, &secret, true, o, w, pin).is_err());
        assert!(Archive::open(&bytes, &secret, false, Uuid::new_v4(), w, pin).is_err());
        assert!(Archive::open(&bytes, &secret, false, o, Uuid::new_v4(), pin).is_err());
    }
    #[test]
    fn limits_sparse_versions_duplicates_and_missing_version() {
        let o = Uuid::new_v4();
        let w = Uuid::new_v4();
        for versions in [vec![0], vec![1, 1], vec![2, 1], vec![i32::MAX as u32 + 1]] {
            assert!(Archive::new(
                o,
                w,
                1,
                versions
                    .into_iter()
                    .map(|v| (v, Zeroizing::new([7; 32])))
                    .collect()
            )
            .is_err());
        }
        assert!(Archive::new(
            o,
            w,
            1,
            (1..=1001).map(|v| (v, Zeroizing::new([7; 32]))).collect()
        )
        .is_err());
        assert!(Archive::new(o, w, 0, vec![(1, Zeroizing::new([7; 32]))]).is_err());
        let a = Archive::new(
            o,
            w,
            1,
            vec![
                (1, Zeroizing::new([7; 32])),
                (i32::MAX as u32, Zeroizing::new([8; 32])),
            ],
        )
        .unwrap();
        let s = Secret::random().unwrap();
        let (b, p) = a.seal(&s, false).unwrap();
        let restored = Archive::open(&b, &s, false, o, w, p).unwrap();
        assert!(restored.select(o, w, 2).is_err());
        assert!(restored.select(o, w, i32::MAX as u32).is_ok());
        assert!(Archive::open(&vec![0; MAX_FILE + 1], &s, false, o, w, p).is_err());
        for n in 0..b.len() {
            assert!(Archive::open(&b[..n], &s, false, o, w, p).is_err());
        }
    }
    #[test]
    fn recovery_survives_loss_of_local_secret_and_all_archive_memory() {
        let o = Uuid::new_v4();
        let w = Uuid::new_v4();
        for count in [1, 8, 100, 1000] {
            let a = archive(count, 2, o, w);
            let recovery = Secret::random().unwrap();
            let local = Secret::random().unwrap();
            let (local_bytes, _) = a.seal(&local, false).unwrap();
            let (copy, pin) = a.seal(&recovery, true).unwrap();
            // Independent fictional known-key digests, not copied key material.
            let expected: Vec<[u8; 32]> = a
                .keys
                .iter()
                .map(|(_, k)| Sha256::digest(k.as_ref()).into())
                .collect();
            drop(a);
            drop(local);
            drop(local_bytes);
            let restored = Archive::open(&copy, &recovery, true, o, w, pin).unwrap();
            for (i, digest) in expected.iter().enumerate() {
                assert_eq!(
                    &<[u8; 32]>::from(Sha256::digest(restored.select(o, w, i as u32 + 1).unwrap())),
                    digest
                );
            }
            let newer = archive(count, 3, o, w);
            let (_, newpin) = newer.seal(&recovery, true).unwrap();
            assert!(Archive::open(&copy, &recovery, true, o, w, newpin).is_err());
        }
    }
    #[test]
    fn existing_generation_file_is_never_overwritten_and_truncation_refuses() {
        let path = std::env::temp_dir().join(format!("sitedatum-d3-{}.fixture", Uuid::new_v4()));
        let o = Uuid::new_v4();
        let w = Uuid::new_v4();
        let secret = Secret::random().unwrap();
        let (bytes, pin) = archive(8, 1, o, w).seal(&secret, false).unwrap();
        write_new(&path, &bytes).unwrap();
        assert_eq!(write_new(&path, &bytes), Err(Refusal::Storage));
        assert_eq!(read_bounded(&path).unwrap(), bytes);
        assert!(Archive::open(&read_bounded(&path).unwrap(), &secret, false, o, w, pin).is_ok());
        let f = OpenOptions::new().write(true).open(&path).unwrap();
        f.set_len(HEADER as u64).unwrap();
        f.sync_all().unwrap();
        assert!(Archive::open(&read_bounded(&path).unwrap(), &secret, false, o, w, pin).is_err());
        // Delete only this UUID-scoped disposable fixture, never customer paths.
        std::fs::remove_file(&path).unwrap();
    }
    #[test]
    fn fresh_nonces_and_domain_separation_prevent_relabeling() {
        let o = Uuid::new_v4();
        let w = Uuid::new_v4();
        let a = archive(8, 1, o, w);
        let secret = Secret::random().unwrap();
        let (mut bytes, pin) = a.seal(&secret, false).unwrap();
        let (again, _) = a.seal(&secret, false).unwrap();
        assert_ne!(&bytes[52..76], &again[52..76]);
        bytes[..8].copy_from_slice(RECOVERY);
        let relabeled = Pin {
            digest: Sha256::digest(&bytes).into(),
            ..pin
        };
        assert!(Archive::open(&bytes, &secret, true, o, w, relabeled).is_err());
    }
    #[cfg(windows)]
    #[test]
    fn windows_dpapi_roundtrip_and_corrupt_wrapper_refusal() {
        let secret = Secret::random().unwrap();
        let wrapped = dpapi(secret.0.as_ref(), true).unwrap();
        let unwrapped = dpapi(&wrapped, false).unwrap();
        assert!(unwrapped.as_slice() == secret.0.as_ref());
        let mut corrupt = wrapped.to_vec();
        corrupt[0] ^= 1;
        assert!(dpapi(&corrupt, false).is_err());
        assert!(dpapi(&[], false).is_err());
        assert!(dpapi(&vec![0; 8193], false).is_err());
    }

    #[cfg(windows)]
    #[test]
    fn windows_file_vault_reopens_using_only_protected_local_secret() {
        let owner = Uuid::new_v4();
        let workspace = Uuid::new_v4();
        let a = archive(1000, 1, owner, workspace);
        let secret = Secret::random().unwrap();
        let wrapped = dpapi(secret.0.as_ref(), true).unwrap();
        let (file, pin) = a.seal(&secret, false).unwrap();
        let path =
            std::env::temp_dir().join(format!("sitedatum-d3-local-{}.fixture", Uuid::new_v4()));
        write_new(&path, &file).unwrap();
        let protected_path =
            std::env::temp_dir().join(format!("sitedatum-d3-protected-{}.fixture", Uuid::new_v4()));
        // DPAPI ciphertext only. This OS blob is not a published wrapper format.
        write_new(&protected_path, &wrapped).unwrap();
        let expected: Vec<[u8; 32]> = a
            .keys
            .iter()
            .map(|(_, k)| Sha256::digest(k.as_ref()).into())
            .collect();
        drop(a);
        drop(secret);
        drop(file);
        drop(wrapped);
        let wrapped = read_bounded(&protected_path).unwrap();
        let unwrapped = dpapi(&wrapped, false).unwrap();
        let mut secret = Zeroizing::new([0; 32]);
        assert_eq!(unwrapped.len(), 32);
        secret.copy_from_slice(&unwrapped);
        drop(unwrapped);
        let restored = Archive::open(
            &read_bounded(&path).unwrap(),
            &Secret(secret),
            false,
            owner,
            workspace,
            pin,
        )
        .unwrap();
        for (i, digest) in expected.iter().enumerate() {
            assert_eq!(
                &<[u8; 32]>::from(Sha256::digest(
                    restored.select(owner, workspace, i as u32 + 1).unwrap()
                )),
                digest
            );
        }
        std::fs::remove_file(&path).unwrap();
        std::fs::remove_file(&protected_path).unwrap();
    }
}

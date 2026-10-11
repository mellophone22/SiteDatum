# C10-04D D3 — bounded encrypted-file vault prototype

Date: 2026-10-11  
Source commit: 72855ef57375d097d7e9169be1858868bb09d8e8  
Base: 1bf424a1ad6d498fc51e63a09dd60bca23449135  
Branch: codex/c10-02e-claude-review

## Decision and scope

The founder explicitly approved the local encrypted-file storage direction, with a separate customer-held recovery copy. We implemented that choice as a bounded **test-only prototype**, not another specification-only correction. No approval to ship a new custody/cryptographic protocol, activate Sync, remove a key or migrate customer material is inferred.

We retain the no-removal operational baseline. The eight-key production ring is unchanged. The new module is included only under cfg(test); there is no Tauri command, UI, hosted endpoint, transport or production import path. The proof module and its experimental markers are not a published file-format contract.

Sources:

- src-tauri/src/sync_v2_vault_file_prototype.rs — encrypted archive, file IO, scoped Windows DPAPI adapter and adversarial tests.
- src-tauri/src/sync_v2_retirement_census_proof.rs — actual SQLite twelve-column recovery drill and refusal of historical archive write authority.
- src-tauri/src/lib.rs — test-only module gate.
- src-tauri/Cargo.toml and Cargo.lock — Windows-only dev dependency on windows-sys 0.61.2, a version already present in the lockfile; no new production dependency.

The preceding [Windows measurements](C10-04D-D3-WINDOWS-MEASUREMENTS.md) observed a refusal during a large Credential Manager inventory. They justify testing a file-shaped archive; they do not establish that every file-shaped construction is safe.

## Implemented construction

We use existing XChaCha20Poly1305 and HKDF-SHA256 dependencies, not a new cipher. The prototype uses separate randomly generated 32-byte local and recovery secrets. The recovery secret is not a password, an email reset token, or an owner-signing root. It is never printed or written in plaintext by the fixtures.

The local secret is wrapped with user-scoped Windows DPAPI without the LOCAL_MACHINE flag or a prompt. The Windows test writes only the DPAPI ciphertext to a unique disposable auxiliary file, discards the plaintext secret and archive, reopens both files, unwraps the secret and verifies all 1,000 restored keys. This is same-user/same-host, same-process reopen evidence, **not** a reboot, roaming, account-reset or cross-machine custody test.

The recovery copy independently encrypts the complete retained key set using the separate recovery secret and a separate HKDF domain. It does not require the local DPAPI wrapper. Both archive kinds authenticate the full header as associated data. The header contains an experimental eight-byte marker, owner/workspace UUIDs, generation, entry count and a fresh 24-byte random nonce. These metadata are visible, not secret.

| Experimental envelope field | Size |
| --- | --- |
| Marker | 8 bytes |
| Owner UUID + workspace UUID | 32 bytes |
| Generation, big-endian | 8 bytes |
| Entry count, big-endian | 4 bytes |
| Random nonce | 24 bytes |
| Sorted encrypted entries | 36 bytes per version |
| Authentication tag | 16 bytes |

Experimental markers are SDPV0001 for local and SDPR0001 for recovery. They are **not shipping magic values**. HKDF uses the complete header as salt and distinct local/recovery prototype labels as info. An entry contains its original positive u32 version and 32-byte key. Versions are never renumbered; sparse versions are allowed, duplicate/descending/zero/out-of-i32-range versions refuse. Generations are 1..=MAX_SAFE_INTEGER.

The prototype ceiling is 1,000 entries and 36,092 file bytes. Reads check metadata and take at most that bound plus one byte. Exact envelope length must agree with count. Temporary plaintext/key buffers use Zeroizing with preallocated bounded capacity; secrets/archive types do not implement Debug, Serialize or Clone. This is not a guarantee against compiler copies, memory dumps or same-user compromise.

## File and authority behavior

Writes use create_new, write_all, sync_all and byte-for-byte bounded readback. An existing file is never replaced. A failed new-file write remains for inspection and returns Incomplete; it is not silently deleted or called a confirmed recovery copy. This is a primitive for immutable generation files, not a committed generation-pointer or crash-safe multi-store publication protocol.

Opening requires the expected owner/workspace and an independently supplied generation/full-file SHA-256 pin. A pin is a fixture expectation, not an implemented durable trust anchor: a hash can detect a mismatch but cannot discover freshness. No provider-supplied pin is trusted by a production path, because no production path exists. An old copy refuses against a newer expected pin; losing or rolling back every independent pin is still an unresolved freshness limit.

An opened Archive resolves exact-version historical reads and unconditionally returns None from write_version. The existing queue rejects it without database changes even when it can decrypt the requested record. Existing D1 compile-fail role proofs still pass; this prototype compatibility adapter relies on the existing runtime write-version refusal, not a new claim of compile-time separation at every existing WorkspaceKeys caller.

## Executed tests

The final source adds nine tests:

- Every byte of a small envelope mutated individually is refused, even if the public expected digest is recomputed. Wrong secret, archive kind, owner and workspace also refuse.
- Version/order/count/generation limits, sparse high versions, missing exact versions, every truncated prefix and oversized in-memory input are checked.
- Independent recovery restores all known keys at 1, 8, 100 and 1,000 entries after the archive and local secret are dropped.
- An authentic older recovery copy refuses against a newer generation/digest expectation.
- Existing generation-file replacement refuses and leaves the original bytes intact; an explicitly truncated disposable fixture refuses.
- Fresh nonce sampling and local-to-recovery relabelling refusal are checked. Random nonce uniqueness is an algorithm assumption, not proved by one sampling test.
- Windows DPAPI roundtrip, corrupt-wrapper refusal and input bounds pass.
- Reopening the local vault and its persisted protected-secret fixture restores 1,000 keys after plaintext source objects are dropped.
- The actual migrated SQLite schema is exercised at every retained version for counts 1, 8, 100 and 1,000, followed by a restored-archive queue-write refusal regression.

The last two SQLite checks are separate tests. The census drill covers all twelve ciphertext columns, including nullable columns populated with ciphertext and both resolved/unresolved conflicts. It checks seventeen cells per retained version, using a fixed fictional database fixture sequentially repopulated at each version. We authenticate 18,853 cells across the four sizes using the restored archive, with read-only census transactions and no census-side writes.

This is **not** a database containing 12,000 historical rows simultaneously, historical-manifest discovery, customer backup migration, device transfer, a real DPAPI-loss experiment, or a production recovery ceremony. The checkpoint fixture uses a known empty manifest. Recovery works after we discard the local secret; we do not destroy the user's DPAPI master keys or alter real Windows credentials.

Disposable files created by successful drills were removed by their exact UUID-scoped paths. No real credential was enumerated, replaced or removed. Failed prototype new files are deliberately not automatically deleted. No customer data or secret was logged.

## Verification

Executed at the final source:

- cargo test --locked: **236 passed, 0 failed, 8 ignored** across Rust targets.
- cargo fmt --check: pass.
- cargo check --locked --lib: pass.
- npm test: **114 passed in 35 files**.
- npm run lint: pass.
- npm run build: pass.
- git diff --check: pass.

The default ignored live credential drills were not rerun. A non-fatal incremental-cache hard-link fallback warning occurred. cargo clippy could not run because cargo-clippy is not installed for this host's Rust toolchain; no Rust Clippy pass is claimed. No UI changed, so visual review is not applicable.

## Rollout and remaining gates

We can remove the test-only module without migrating any customer material. Nothing in the 1.4.4 installer gains this capability. Ordinary local editing, backup and essential export are unchanged.

This completes the bounded implementation/proof slice, **not D3's production acceptance**. Before promotion we still need an accepted final format and custody contract, independently reviewed cryptographic composition, authenticated/durable external pins, recovery-secret delivery and confirmation UX, production resource budgets, Windows restart/account/roaming/ACL/reparse-point behavior and failure injection. DPAPI is not a portable recovery format and does not protect against arbitrary code running as the same Windows user.

D2 owner-root encoding/verifier, D4 authenticated admission/floors/consistent hosted locks and D5 durable multi-store journal remain separate prerequisites. D6 pruning and D7 activation remain blocked. No main merge, release, deployment, customer file mutation or Sync activation follows.

## Primary references

The [pinned RustCrypto documentation](https://docs.rs/chacha20poly1305/0.10.1/chacha20poly1305/) describes the existing AEAD and 24-byte XChaCha nonce; a library audit is not an audit of this archive protocol. [Microsoft CryptProtectData](https://learn.microsoft.com/en-us/windows/win32/api/dpapi/nf-dpapi-cryptprotectdata) and [CryptUnprotectData](https://learn.microsoft.com/en-us/windows/win32/api/dpapi/nf-dpapi-cryptunprotectdata) describe user-scoped protection, integrity checks and documented roaming exceptions. They do not establish independent freshness or portable customer recovery.

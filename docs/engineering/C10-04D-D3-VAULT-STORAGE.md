# C10-04D D3 — gated Windows vault bundle persistence

Date: 2026-10-11  
Source commit: 951057a573a8365bd0c18e9b7ec9ddc2d12d9d32  
Base: 030db34f0de7cfc41ed6eb9ea4d5cb76d54360ba  
Branch: codex/c10-02e-claude-review

## Implemented scope

Following the founder's approval of encrypted-file storage and instruction to continue with vault storage/recovery, we implemented a reusable Windows persistence boundary. This is actual code and executed testing, **not completed production acceptance**.

The existing experimental archive codec is reused rather than copied. It and the new persistence module compile under tests or the explicit sync-v2-vault-review feature, which is absent from default features. There is no application caller, Tauri command, automatic migration, active-generation pointer, network operation, key removal or Sync activation. A review build is not a customer-use authorization.

Files changed:

- src-tauri/src/sync_v2_vault_storage.rs — local bundle creation/opening, recovery-file persistence, bounded handle IO and tests.
- src-tauri/src/sync_v2_vault_file_prototype.rs — scope accessor, internal pin metadata access and narrowly scoped secret wrap/unwrap functions; archive encoding and labels unchanged.
- src-tauri/src/lib.rs — explicit review/test compilation gates.
- src-tauri/Cargo.toml — disabled-by-default review feature and optional Windows-only dependency on already pinned windows-sys 0.61.2. The existing dev dependency remains; Cargo.lock is unchanged.

We preserve the existing eight-key no-removal baseline. Historical reads remain exact-version only, and the archive still has no write_version authority. The earlier [file-vault prototype](C10-04D-D3-FILE-VAULT-PROTOTYPE.md) and its twelve-column SQLite recovery drill remain in the regression suite.

## Storage behavior

We package the Windows-protected local secret with its encrypted archive in **one immutable generation file**. This avoids independently publishing two local files, but it does not solve the larger SQLite/hosted/pin journal problem.

The experimental bundle begins with SDPB0001, then a big-endian u32 wrapper length, followed by the DPAPI ciphertext and the existing local encrypted archive. Wrapper length is bounded at 8,192 bytes; archive length at 36,092; total bundle at 44,296. These remain review/prototype budgets, not final product limits or a published format promise. Metadata, ciphertext and hashes are not treated as project records; nothing is sent to billing or providers.

A local create generates a fresh 32-byte local secret, wraps it with user-scoped DPAPI, encrypts the full archive and assembles the bundle in memory. It uses CREATE_NEW semantics with an exclusive handle, writes the complete bytes, synchronizes the file and verifies readback on the same handle. After closing that handle and discarding the plaintext local secret, it reopens the persisted bundle, unwraps the persisted secret and verifies the archive before returning the expected-context token.

The complete bundle digest, owner, workspace and generation are supplied again on reopening from an independently retained pin. The archive is also authenticated by AEAD. A digest is not an independently implemented freshness source: ExpectedBundle::from_independent_pin is a trust-boundary constructor, not a verifier of where a pin came from. D2/D5 must supply accepted durable pin evidence before customer use; an untrusted file, provider response or filename cannot supply that evidence.

The recovery file is independently encrypted using the separate recovery secret. Creation performs durable write/readback and authenticated reopen before returning its pin. Reading it requires the secret, exact scope and independently retained pin. The local DPAPI bundle is not required for this recovery path.

## File safeguards and limitations

We open the final path component with FILE_FLAG_OPEN_REPARSE_POINT and inspect the opened handle, refusing reparse points, directories/non-regular files and oversized inputs. Bounded reads allocate at most the configured bound plus one byte. Read handles allow other readers but deny conflicting writers/deleters; creation handles are exclusive through same-handle readback.

The destination **parent directory must already be trusted and protected by the caller**. This slice does not create a private application root, set its ACL, or prove resistance to ancestor junction substitution. Final-component inspection is not proof that all ancestors were safely resolved.

An existing generation is never overwritten, renamed or deleted. Any failure after new-file creation returns Incomplete, leaves that new file for inspection, and returns no confirmed-generation token. The previous generation is never modified. A fully written but unacknowledged orphan can still be cryptographically authentic; callers must not discover/adopt it as current merely because it opens successfully. No active pointer or orphan-adoption API is supplied here.

The phase hooks are private and used for deterministic fault injection by tests. They model interruptions, not disk-full hardware failures or power loss. The production create calls do not expose those hooks.

## Executed tests

Seven new parent tests passed:

1. Injected refusal at five boundaries (created, prefix written, full bytes written, synchronized, readback) returns Incomplete, leaves each candidate for inspection, and keeps the old generation byte-identical and readable.
2. Recovery at 1, 8, 100 and 1,000 keys succeeds after the fictional local bundle is deleted; wrong secret/scope and an older recovery file against a newer expected pin refuse. Restored archives have no write authority.
3. Corrupt wrappers refuse, including after a fake recomputed public digest; oversized and directory inputs refuse.
4. Two concurrent creators targeting the same filename yield exactly one successful creation, which opens successfully.
5. A separate process reopens 1,000 historical keys from the local bundle using DPAPI, without plaintext secret transfer. Only fictional file path, scope and public digest metadata are passed. The parent verifies the exact child test ran once and passed, so a zero-test child success is not counted.
6. A child writer exits with code 23 immediately after writing a 12-byte prefix, before synchronization. The partial candidate refuses, while the old generation remains unchanged and readable. This is process-exit interruption, not a power-loss simulation.
7. While the exclusive creation handle is open, attempted competing readers and writers refuse; the bundle opens after the handle is closed.

The child probe is ignored in the ordinary suite and explicitly invoked by its two parent fixtures. Its default ignored count must not be interpreted as an unexecuted subprocess drill.

Tests own a newly created UUID-scoped temporary directory. They remove only their explicitly tracked fictional files and then the empty directory, without recursive deletion or credential enumeration. Interrupted fixture files are inspected before that explicit fixture cleanup. No customer keys, files, credentials, profiles or account configuration were changed.

## Verification

Executed at the final source:

- cargo test --locked: **243 passed, 0 failed, 9 ignored** across Rust targets.
- cargo fmt --check: pass.
- cargo check --locked --lib: pass.
- cargo check --locked --lib --features sync-v2-vault-review: pass.
- npm test: **114 passed in 35 files**.
- npm run lint: pass.
- npm run build: pass.
- git diff --check: pass.

Earlier live Credential Manager measurements were not rerun. Clippy remains unavailable on this toolchain, as recorded in the preceding slice; no Clippy pass is claimed. No UI changed, so visual review is not applicable.

## Still required before customer use

We have completed this storage/interruption implementation slice. We have **not** completed D3 production acceptance or all Sync work.

Remaining vault requirements include the accepted final format and independent cryptographic composition review, protected application-root/ancestor/ACL behavior, recovery-secret delivery/confirmation UI, durable independent pin custody, final resource budgets, Windows reboot/account-reset/roaming tests, permission and disk/full-power-loss failure handling, and an agreed recovery ceremony. DPAPI is not portable recovery and cannot defend against arbitrary code running as the same Windows user.

D2 authority encoding/verifier, D4 authenticated admission/consistent hosted locks/floors, and D5 actual multi-store journal and orphan reconciliation remain open. The single-file bundle reduces one local publication split but does not join it atomically with SQLite, hosted acknowledgements or customer recovery generations. D6 key retirement and D7 activation remain blocked.

Rollback is removing the dormant review feature/modules without customer migration. No main merge, release, deployed service, customer migration or Sync activation follows.

## Primary source basis

[Microsoft CreateFileW](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-createfilew) documents CREATE_NEW, exclusive sharing and final reparse-point handling. [Microsoft FlushFileBuffers](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-flushfilebuffers) documents flushing file buffers; a successful sync call is not proof of all crash/power-loss behavior. DPAPI/AEAD references and their limits are preserved in the preceding prototype record.

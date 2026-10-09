# C10-02B device-key lifecycle proof

**Status:** Complete on the supported Windows workstation (2026-10-08)

## Purpose

Prove the narrow local lifecycle required for a future Sync v2 device-private
key without enabling Sync or changing the production application. The proof
uses only fictional material and one uniquely named temporary entry in Windows
Credential Manager.

## Isolation boundary

- The implementation lives only in
  `src-tauri/tests/sync_v2_windows_keyring_proof.rs`.
- No production Rust module imports it and no Tauri command or UI control can
  invoke it.
- The live test is ignored by default. It runs only when explicitly selected.
- The credential service is reserved for the C10 proof and is different from
  the production licensing and legacy cloud-session services.
- Every run uses a random account suffix and a cleanup guard deletes that exact
  entry on success or panic.
- No project name, record, document, file path, customer identity, or production
  credential enters the proof.

## Lifecycle exercised

1. Generate 32 bytes with the operating-system CSPRNG.
2. Store a schema- and key-version-tagged fictional private key.
3. Read it through Windows Credential Manager and compare only an in-memory
   SHA-256 fingerprint.
4. Replace version 1 with independently generated version 2 material.
5. Reopen a fresh credential handle and confirm version 2 persists while
   version 1 is no longer returned.
6. Revoke by deleting the credential and confirm the store returns `NoEntry`.

The test never prints key material or serialized credentials. Secret buffers
and serialized strings are wrapped for zeroization where the selected crates
support it. Rust, Windows, allocator, crash-dump, and OS-internal copies cannot
be proven fully zeroized; production design must therefore minimize secret
lifetimes, disable secret-bearing diagnostics, and treat OS credential storage
as the supported at-rest boundary rather than claiming perfect memory erasure.

## Commands

The in-memory format test runs with the ordinary suite:

```powershell
cargo test --test sync_v2_windows_keyring_proof
```

The explicitly authorized live proof is:

```powershell
cargo test --test sync_v2_windows_keyring_proof -- --ignored --exact windows_credential_manager_proves_create_read_replace_reopen_and_revoke
```

Result: passed. The test confirmed create, read, version replacement,
fresh-handle reopen, revoke/delete, and post-delete `NoEntry`. The temporary
credential was removed, and the test output contained no key or serialized
credential value.

## Non-goals

- no real reinstall or Windows-profile migration;
- no production credential-name migration;
- no SQLite or project-data access;
- no hosted identity, Supabase, RLS, retention, or deletion change;
- no device enrollment, HPKE transfer, or recovery flow; and
- no customer-visible Sync feature.

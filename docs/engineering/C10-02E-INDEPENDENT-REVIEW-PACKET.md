# C10-02E independent cryptographic and protocol review packet

**Status:** Independent AI review returned NOT APPROVED; remediation implemented and re-review pending — 2026-10-08

## Purpose

This packet defines the exact C10-02 snapshot and evidence an independent reviewer must assess before C10-03 may begin. The internal preflight described below is preparation only. It is not the independent approval required by ADR-019.

Production Sync remains disabled. `CommercialFeature::MetadataSync` is denied for Free and Pro, the `SYNC_DEFERRED` boundary remains in force, and this change set adds no customer-visible Sync control, production Sync command, or hosted production resource.

## Pre-hardening scan reference

- Base repository revision: `86ebfd561ac11b2b1a62f048e0ccbc9839aa76dd`
- Pre-hardening working-tree snapshot: `codex-security-snapshot/v1:sha256:2319dbe4e47953bd4119235f87209aad6f107e285d285ce3dd2668af17a16d89`
- Internal preflight scan: `939f28b0-cabf-4986-90b9-91159f9b8cd3`
- Review scope: C10-01 through C10-02D architecture and test-only proofs

The three hardening corrections described below were made after this scan. A
new immutable digest must be recorded when the final review package is
committed; the pre-hardening digest must not be presented as the reviewed final
source.

## Evidence set

The reviewer must examine:

1. `ADR-019-SYNC-V2-PRIVACY-AND-SECURITY-BOUNDARY.md`
2. `SYNC-V2-DATA-INVENTORY.md`
3. `C10-02-CRYPTO-IDENTITY-IMPLEMENTATION-PLAN.md`
4. `C10-02B-DEVICE-KEY-LIFECYCLE-PROOF.md`
5. `C10-02C-DEVICE-TRANSFER-RECOVERY-PROOF.md`
6. `C10-02D-DISPOSABLE-IDENTITY-PROOF.md`
7. `src-tauri/tests/sync_v2_crypto_proof.rs`
8. `src-tauri/tests/sync_v2_windows_keyring_proof.rs`
9. `src-tauri/tests/sync_v2_device_transfer_proof.rs`
10. `src-tauri/tests/sync_v2_identity_proof.rs`
11. `proofs/sync-v2-identity/supabase/migrations/20261008000000_sync_v2_identity_rls_proof.sql`
12. `proofs/sync-v2-identity/supabase/tests/identity_rls.sql`
13. the pinned development dependencies in `src-tauri/Cargo.toml` and `src-tauri/Cargo.lock`

The original internal evidence run produced:

- 82 passing Rust tests and one deliberately ignored live Windows Credential Manager test;
- a separately selected passing Windows Credential Manager lifecycle drill;
- 20 passing disposable local Supabase pgTAP assertions;
- a clean repository secret-pattern scan; and
- removal of the disposable Supabase containers and volumes after the proof.

The independent AI second-opinion report subsequently identified two blocking
High findings and returned **NOT APPROVED FOR C10-03**. The remediation and new
verification evidence are recorded in
`C10-02E-INDEPENDENT-REVIEW-REMEDIATION.md`. This packet's original result must
not be presented as the current gate decision.

## Required review questions

The independent review must address, at minimum:

- algorithm and construction suitability for record encryption, recovery envelopes, and approved-device transfer;
- authenticated-data completeness and canonical encoding ambiguity;
- nonce generation, uniqueness, persistence, and crash/retry behavior;
- workspace-key, device-key, and recovery-secret generation, storage, rotation, transfer, revocation, and destruction;
- replay, substitution, expiry, downgrade, key-version, and comparison-code handling;
- OAuth/OIDC authorization-code plus S256 PKCE binding, callback validation, token validation, JWKS rotation, logout, revocation, device loss, and account deletion;
- owner, session, and device authorization composition in RLS and privileged functions;
- ciphertext-only provider visibility and project-content leakage through errors, logs, identifiers, or support artifacts;
- production reachability of every test helper and development dependency; and
- whether the proofs provide a safe foundation for C10-03, without treating them as production implementations.

## Internal preflight result

The completed internal diff scan reported zero production-reachable security findings. It confirmed the following controls:

- Sync v2 remains denied and production-unreachable.
- New cryptographic implementations exist only in integration tests and use development-only dependencies.
- The Windows credential proof is ignored by default, uses fictional identifiers, and cleans up its exact entry.
- The disposable RLS proof denies anonymous and cross-owner access and requires matching active session and device state.
- No privileged provider credential or customer project content was added.

The preflight also identified three proof-hardening items. They were not production vulnerabilities because the affected models are test-only and unreachable. All three have now been resolved:

1. Device-enrollment expiry is now exclusive, rejects `now >= expires_at`, and tests the equality boundary.
2. Authorization-code expiry now uses the same exclusive boundary and tests equality.
3. Modeled account deletion now invalidates pending authorization codes, rejects their exchange with `ACCOUNT_DELETED`, and tests that behavior.

After these corrections, the focused device-transfer and identity suites passed
12 tests. The complete Rust suite passed 82 tests with the deliberately ignored
live Windows Credential Manager test remaining ignored. `cargo fmt --check`
also passed.

## Independent reviewer deliverable

The reviewer must return a dated report tied to the revision and snapshot digest above. Each issue must include severity, affected file and line, threat scenario, recommended correction, and retest criteria. The report must state one of:

- **Approved:** no unresolved high or critical issue and the design is suitable to proceed to C10-03;
- **Approved with conditions:** no unresolved high or critical issue, with listed conditions that must be completed before a specified later gate; or
- **Not approved:** C10-03 remains blocked.

All high and critical findings must be resolved and re-reviewed. Lower-severity findings must be resolved or recorded as an explicit founder-approved residual risk. The report must contain only fictional evidence and must not include credentials, private customer information, provider payloads, project records, file paths, documents, or database contents.

## Gate state

C10-02E remains open until:

1. a new immutable digest is recorded for the hardened source;
2. the independent reviewer examines the new immutable remediation snapshot;
3. every high or critical finding is closed; and
4. the approval and residual-risk record are committed with the C10 evidence.

C10-03 must not begin before this gate closes.

# C10 — Sync v2 reintroduction

**Status:** C10-01 complete; C10-02A/B/C/D proofs complete; customer access remains disabled

## Objective

Reintroduce optional two-computer metadata synchronization without changing SiteDatum's local-first, single-user model or exposing plaintext project content to the Sync provider. Normal files remain user-controlled Windows files. Free local use remains accountless.

## Non-goals

- collaboration, organizations, shared workspaces, RBAC, or seat billing;
- web/mobile access to project records;
- project-document upload or storage;
- telemetry or support upload of project content;
- automatic migration from the legacy Sync service; or
- using Sync as a substitute for local backup and complete Free export.

## Work packages

### C10-01 — Architecture, inventory, threat model, and policy

- Accept ADR-019.
- Freeze the plaintext and server-visible data inventory.
- Produce an independent, source-linked threat model.
- Define explicit consent, withdrawal, retention, deletion, and recovery requirements.
- Define evidence-based exit criteria.

**No runtime or hosted-service change is authorized by C10-01.**

### C10-02 — Cryptographic and identity proof

Implementation plan: `C10-02-CRYPTO-IDENTITY-IMPLEMENTATION-PLAN.md`.

- Select versioned authenticated-encryption and key-derivation algorithms from a mature audited library.
- Specify per-workspace/per-record key hierarchy, nonce rules, rotation, Windows-protected storage, device transfer, and recovery.
- Prototype a short-lived identity exchange between SiteDatum customer identity and the isolated Sync backend without a desktop secret.
- Commission an independent design review and address every high/critical finding.

Exit: test vectors, lost-device drill, key-transfer/recovery drill, token audience/issuer verification, replay resistance, and documented failure behavior pass. No customer beta yet.

### C10-03 — Reproducible hosted boundary

- Create a new isolated Supabase project for Sync v2.
- Commit schema, RLS, grants, RPCs, retention jobs, deletion workflow, rollback, and pgTAP tests.
- Add cross-user and unauthenticated negative tests for every operation.
- Prove service-role credentials are absent from client artifacts and repository history.

Exit: a disposable project can be built and verified from the repository; security advisors and automated tests pass.

### C10-04 — Local record protocol

- Add versioned record codecs, encrypted outbox, pull cursor, idempotent mutation IDs, tombstones, and record-scoped conflicts.
- Validate all pulled changes before a transaction and create a verified safety backup before application.
- Preserve local work through every network, schema, authentication, and decryption failure.

Exit: deterministic Rust tests pass for two-computer offline edits, duplicate/reordered delivery, replay, conflict, tombstone, schema skew, corrupt ciphertext, and rollback.

### C10-05 — Consent, device, deletion, and recovery UX

Primary job: let one person deliberately enable, understand, control, and completely remove their encrypted cloud copy without disrupting local work.

The settings surface must show concrete status, last successful sync, pending work, active devices, policy version, retention state, and clear actions for Sync now, Disconnect this computer, Disable Sync, and Delete cloud data. Destructive actions name the consequence and keep local records/files explicitly in scope. Every visible control must work, be keyboard accessible, and have error/retry states.

Exit: UI, backend, and accessibility tests pass for consent, policy renewal, device authorization/revocation, withdrawal, disable, recovery-window cancellation, delete-now, and account deletion.

### C10-06 — Recovery and operational verification

- Document provider outage, token compromise, key/device loss, deletion failure, stuck retention, data corruption, and rollback runbooks.
- Perform encrypted-row backup and restore in a disposable environment.
- Verify content-free monitoring and support diagnostics.
- Exercise the upload kill switch without disabling local work.

Exit: restore and incident drills produce retained, redacted evidence and meet the recovery targets approved before this package.

### C10-07 — Fictional two-profile beta and launch review

- Start behind a server and desktop feature flag for explicitly enrolled fictional/test profiles.
- Verify two Windows profiles/computers, long offline periods, upgrades, uninstall/reinstall, device loss, deletion, and provider outage.
- Perform final privacy, threat-model, RLS, consent-copy, retention, and recovery review.

Exit: founder approves the evidence package. General availability remains a separate release decision.

## Required validation matrix

| Area | Required proof |
| --- | --- |
| Local-first | App launches and all ordinary work continues with no account/network; disabling/deleting Sync preserves local records and files |
| Confidentiality | Provider rows and logs contain no plaintext project content; wrong-key and cross-user attempts fail |
| Authorization | Owner-only CRUD and RPC tests; unauthenticated, wrong-owner, mutable-metadata, stale-token, and revoked-device tests |
| Integrity | Authenticated decryption, expected-version checks, idempotency, cursor replay, schema validation, and atomic local apply |
| Conflicts | Both candidates retained; record named locally; no silent last-write-wins; repeated resolution is idempotent |
| Deletion | Delete-now removes active rows, revokes sessions, issues a content-free receipt, and respects documented backup aging |
| Recovery | Local backup/restore, encrypted hosted restore, key/device recovery, outage, and kill-switch drills |
| Privacy | Versioned consent matches actual data inventory; withdrawal stops uploads; no project content reaches licensing/billing/telemetry/support |
| Compatibility | Mixed supported client versions and schema upgrades fail safely or interoperate by documented rule |

## Current gate

C10-01 is complete and the founder approved the C10-02 key-recovery and identity-proof direction on 2026-10-08. The offline record/recovery proof, isolated Windows device-key lifecycle proof, approved-device transfer/total-device-loss recovery drills, and disposable identity/RLS proof are complete. The identity proof used a local disposable Supabase stack only; it created no hosted resource and left no container or volume running. The contained legacy code remains hidden from fresh installations. C10-02 remains limited to test-only proof work until the independent-review gate passes; no production Supabase project, secret, customer row, application command, or runtime feature flag is changed by this package.

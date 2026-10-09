# ADR-019 — Sync v2 privacy and security boundary

**Status:** Accepted for design; implementation remains disabled — 2026-10-08

## Context

SiteDatum is a single-user, local-first Windows product. SQLite remains the operational metadata source of truth and ordinary project documents remain normal Windows files. Free local use remains accountless.

The contained legacy Sync implementation serializes the listed SQLite tables into one `WorkspaceSnapshot`, stores that payload in a legacy Supabase project, and resolves concurrency at whole-workspace granularity. Applying a downloaded snapshot deletes and reinserts every synchronized table inside one local transaction. The code also embeds legacy project coordinates and relies on a hosted schema that is described in documentation but is not represented by a repository migration.

Those properties were acceptable for a limited prototype, but they do not satisfy the privacy, consent, deletion, recovery, and independently reproducible security controls required for a generally available paid feature.

Repository evidence for this decision:

- the legacy table and path allowlists are in `src-tauri/src/cloud_sync.rs:17-38`;
- whole-workspace export and replacement are in `src-tauri/src/cloud_sync.rs:210-341`;
- the legacy REST/RPC boundary is in `src-tauri/src/cloud_sync.rs:389-433`;
- the reconciliation baseline stores the serialized workspace in a column named `content_hash` (`src-tauri/src/cloud_sync.rs:344-350` and `src-tauri/migrations/0008_cloud_sync.sql:1-7`);
- disconnect deletes only the local credential (`src-tauri/src/cloud_auth.rs:232-243`); and
- fresh-install containment is enforced in Rust at `src-tauri/src/lib.rs:135-182` and `src-tauri/src/lib.rs:1278-1395`.

## Decision

### 1. Legacy Sync remains contained

The existing Rust `SYNC_DEFERRED` boundary remains in force. No C10 package may expose Sync v2 to customers until the exit criteria below pass. Existing grandfathered legacy access is a compatibility path, not the foundation for Sync v2, and there is no automatic migration of legacy cloud data.

### 2. Local-first authority does not change

- SQLite is the working database on every computer.
- Project-document bytes stay in the user's Windows filesystem and may be replicated by a user-selected filesystem provider such as OneDrive.
- Sync is optional, separately consented, and never required for Free use, launch, editing, backup, restore, or complete CSV export.
- Sync failure, cancellation, or deletion never deletes, hides, moves, or makes local records uneditable.
- The licensing service remains separate and never receives synchronized project content.

### 3. Sync v2 uses a separate reviewed service boundary

Sync v2 must use a dedicated Supabase project or an equivalently isolated backend, with infrastructure represented by committed migrations and tests. It must not reuse the legacy project coordinates embedded in the desktop client. The desktop may contain only documented public client identifiers. Service-role credentials, signing private keys, webhook secrets, and encryption recovery secrets remain server-side.

Customer authorization is based on the authenticated subject identifier and owner-scoped rows. It must not trust mutable user metadata. Every exposed table has RLS enabled, explicit grants, owner-scoped `SELECT`, `INSERT`, `UPDATE`, and `DELETE` policies, and negative cross-user tests. Any `SECURITY DEFINER` function lives outside exposed schemas, fixes `search_path`, revokes `PUBLIC`, and performs its own ownership check.

### 4. Cloud content is record-level and end-to-end encrypted

The replacement protocol stores one encrypted envelope per local record instead of one readable whole-workspace snapshot. The server receives only the minimum routing and concurrency metadata needed to store and retrieve ciphertext:

- authenticated owner ID;
- random workspace ID and record ID;
- constrained record-kind code;
- monotonically increasing server version;
- client mutation ID for idempotency;
- ciphertext length and encrypted payload;
- tombstone flag and server timestamps; and
- protocol/schema version.

Project names, task titles, RFI questions and responses, notes, contacts, file names, paths, document contents, customer names, and other record fields are encrypted on the Windows computer before upload. Encryption keys are generated and retained on customer computers in Windows-protected storage. The backend must not receive a plaintext workspace key.

The detailed plaintext inventory and the permitted server-visible envelope are defined in `SYNC-V2-DATA-INVENTORY.md`.

### 5. The protocol is incremental, idempotent, and conflict-preserving

The local implementation will use an outbox and a durable pull cursor. Each mutation has a unique idempotency key and an expected record version. A stale expected version creates a record-scoped conflict; it never silently overwrites the remote value. Deletions are explicit tombstones so an offline computer cannot resurrect a deleted record unknowingly.

Downloaded changes are validated completely before a local transaction begins. Foreign-key and schema compatibility failures leave the current SQLite workspace unchanged. A verified local safety backup is created before applying a pull batch or resolving a conflict. Conflict resolution names the affected record and preserves both encrypted candidates until the user chooses a result.

### 6. Identity and entitlement are narrow gates, not data authority

Sync v2 is a Pro capability, but an entitlement only authorizes access to the feature. It never grants access to another owner's cloud rows. The implementation may reuse the customer's SiteDatum account only after a short-lived token-exchange design is proven to bind the licensing identity to the dedicated Sync project's authenticated subject without shipping a privileged secret. Until that proof exists, identity federation is an unresolved implementation gate rather than an assumed control.

### 7. Consent is explicit and versioned

Before the first upload, SiteDatum presents a dedicated consent review that states:

- the exact metadata categories included;
- that document bytes are excluded;
- that protected ciphertext and limited routing metadata are stored by the Sync provider;
- the purpose, optional nature, and Pro dependency of Sync;
- how another computer is authorized and how encryption keys are transferred or recovered;
- the retention periods below;
- the difference between disconnecting a computer, disabling Sync, and deleting cloud data; and
- the consequence of losing every authorized device and any user-held recovery material.

Consent is affirmative, not bundled with licensing or account creation, and recorded locally and remotely as a policy version, timestamp, and authenticated subject. A changed material policy requires renewed consent before further uploads. Declining or withdrawing consent leaves local work intact.

### 8. Retention and deletion baseline

The approved implementation baseline is:

| Data | Active retention | After user action | Maximum operational retention |
| --- | --- | --- | --- |
| Encrypted active records | While Sync is enabled | Delete immediately from active tables on **Delete cloud data** | Provider backups age out on the documented backup schedule |
| Tombstones | Until all active devices acknowledge them | Purge after acknowledgement and at least 30 days | 45 days unless an incident hold is documented |
| Disabled workspace ciphertext | 30-day recovery window | Purge automatically at day 30; **Delete now** bypasses the window | Provider backups age out on schedule |
| Content-free security/audit events | Not used for product analytics | Not user-restorable | 90 days |
| Device sessions | Until revoked, expired, or disconnected | Revoke immediately | No retained refresh token after revocation |

Deleting cloud data is authenticated, idempotent, auditable without content, and returns a deletion receipt. It does not touch the local SQLite database or project files. Account deletion must either delete Sync data first or clearly combine both actions. Support staff cannot inspect or restore plaintext content.

### 9. Recovery treats Sync as replication, not backup

Verified local SQLite backup and complete Free CSV export remain the primary recovery controls. Sync v2 adds, but does not replace, these controls:

- provider database backups or scheduled logical dumps for encrypted server rows;
- a documented restore runbook and quarterly restore drill in a disposable project;
- a device-loss and session-revocation drill;
- an encryption-key recovery or device-transfer drill; and
- a rollback/kill switch that stops uploads while preserving local work and downloads needed for an orderly exit.

No recovery claim is accepted from a configuration screenshot alone. A test must restore encrypted rows, re-establish authorized device access, decrypt them on a test computer, and reconcile them into a disposable local workspace without changing unrelated records.

## Rejected alternatives

- Reactivating the legacy whole-workspace snapshot.
- Uploading the SQLite database file.
- Uploading project-document bytes.
- Server-readable project metadata.
- Last-write-wins conflict handling.
- Authorizing from email address or mutable user metadata.
- Treating an authenticated account or Pro entitlement as Sync consent.
- Silent legacy migration.
- Indefinite retention after disabling Sync.

## Exit criteria before implementation can be exposed

Sync v2 remains unavailable until every item is evidenced:

1. A committed hosted schema and rollback exist in a new isolated backend boundary.
2. RLS, grants, RPC ownership, cross-user isolation, and deletion are covered by automated positive and negative tests.
3. The desktop cryptographic design has named algorithms, key lifecycle, Windows storage behavior, recovery behavior, and an independent review.
4. The record-level protocol passes two-computer offline edit, idempotency, cursor replay, tombstone, schema-upgrade, and conflict tests.
5. Consent, withdrawal, disable, delete-now, account deletion, and policy-version renewal pass UI and backend tests.
6. Logs, errors, telemetry, support artifacts, and provider records pass a project-content leakage review.
7. Backup restore, device loss, key transfer/recovery, provider outage, and rollback drills pass with retained evidence.
8. A fictional two-profile beta passes without exposing or altering either profile's unrelated rows or files.
9. Privacy, terms, retention, and support documentation match the verified implementation.
10. The founder explicitly approves launch after reviewing the evidence package.

## Consequences

Sync v2 is a substantial feature, not a settings toggle. It requires a new protocol, local migrations, a reviewed cloud schema, cryptographic key management, consent and deletion surfaces, and operations evidence. The additional work is intentional: it prevents a cloud convenience feature from weakening SiteDatum's local-first privacy and recovery guarantees.

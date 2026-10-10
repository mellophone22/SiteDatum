# C10 — Sync v2 reintroduction

**Status:** C10-01, C10-02, and disposable local C10-03A through C10-03H workspace-lifecycle slice complete; remaining C10-03 work pending; customer access remains disabled

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
- Add an authenticated encrypted workspace checkpoint, durable per-device
  high-water counter/digest, and transfer/recovery anchors that reject whole-set
  rollback, omission, and equal-counter substitution.
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

C10-01 is complete and the founder approved the C10-02 key-recovery and
identity-proof direction on 2026-10-08. The independent 2026-10-09 re-review
closed both High findings and conditionally approved beginning C10-03. C10-03A
completed a reproducible local identity/authorization boundary: hardened
UUID claim parsing, server-derived device claims, bound and expiring sessions,
and negative RLS tests. It creates no hosted resource and changes no production
Supabase project, secret, customer row, application command, billing setting,
or runtime feature flag. The legacy Sync boundary and centralized
`SYNC_DEFERRED` denial remain in force. C10-03B now adds a verified, local-only
first-device enrollment boundary with Ed25519 proof of possession, one-time
server challenges, durable rate limiting, and atomic device/session binding.
It intentionally exposed no client-callable enrollment function. C10-03C now
adds and locally verifies an authenticated Edge Function bridge with exact
request parsing, live Auth-session revalidation, service-only RPCs, Ed25519
proof verification, generic errors, and durable request-rate windows. It still
creates no production resource and changes no desktop behavior. C10-03D now
adds a non-command native client proof with Windows Credential Manager key
protection, crash-safe pending state, exact proof signing, bounded HTTPS, and a
passing end-to-end disposable enrollment drill. The approved second-device
database state is now complete in C10-03E: an exact target proves possession,
an exact live source device approves it, attempts are single-use and expiring,
and atomic acceptance binds at most a second active device. The service-only
functions had no public bridge or desktop exposure and transferred no workspace
key. C10-03F now adds a separate authenticated bridge, private opaque relay,
and non-command Windows-native HPKE proof: both device signatures bind the
exact enrollment and envelope, only the accepted target fetches it, and the
service cannot decrypt the workspace key. C10-03G now adds the encrypted
hosted record boundary:
device-signed batches atomically commit opaque record envelopes, immutable
changes, an encrypted checkpoint, idempotency state, and the workspace cursor.
Expected-version conflicts reject the whole batch, exact retries are safe,
cross-owner access fails, and the service never interprets ciphertext. The
The C10-03H workspace-lifecycle slice now adds signed disable/recovery,
immediate hosted deletion, account-level Sync-data cleanup with device/session
revocation, content-free receipts, and a daily 30-day retention sweep. Safe
tombstone compaction remains blocked on C10-04's durable acknowledgement and
replacement-checkpoint proof. Other remaining C10-03 gates are clean
reconstruction and security-advisor evidence, and client/repository secret
proof.

C10-04A now implements the first local record-protocol foundation: a canonical
manifest over record IDs, revisions, workspace-key versions, and tombstone
state; an XChaCha20-Poly1305 authenticated encrypted checkpoint bound to its
owner/workspace routing metadata; and a durable local-only high-water anchor
that rejects rollback and equal-counter substitution after restart. This code
has no command or UI and does not enable Sync. C10-04B now adds explicit
versioned codecs for every inventoried record category, authenticated encrypted
records/tombstones, an atomic encrypted staging outbox, exact receipt retries,
and durable checkpoint-covered pull cursor/device staging acknowledgements.
Thirteen focused tests include restart, injected transaction failures, corrupt
and omitted pages, and competing fictional offline edits. These acknowledgements
prove staging only and cannot authorize tombstone compaction. Live project
adapters/application with safety backups, conflict resolution, partial paging,
key rotation, recovery anchors, and comprehensive two-computer scenarios remain
pending in C10-04C and later slices. See `C10-04B-ENCRYPTED-LOCAL-QUEUE.md`.

C10-04C1 now supplies the first backed-up live adapters, limited to notes and
contacts. It rehearses a complete verified page in a private SQLite copy,
requires a verified local safety backup, and commits live application with the
staging cursor/anchor transaction. Divergent local edits and unsupported kinds
fail closed. Full adapter coverage, a durable conflict inbox, partial paging,
live outbox integration and applied-device receipts remain pending. It has no
command/UI/transport and does not enable Sync. See
`C10-04C1-BACKED-UP-RECORD-APPLY.md`.

C10-04C2 extends this boundary to tasks with project/contact reference checks,
exact field preservation and safe local dependency ordering. Task deletion or
project reassignment cannot orphan existing RFI/submittal relationships. No
customer runtime or Sync transport is enabled. Full adapter coverage and other
C10-04C gates remain pending; see `C10-04C2-TASK-ADAPTER.md`.

C10-04C3 adds projects and conservative device-local portable-root mapping.
Existing local project paths cannot be redirected by remote edits; reserved
names, reparse points, path collisions and orphaning project deletions fail
closed. No document operation, applied-device receipt or runtime Sync activation
is added. See `C10-04C3-PROJECT-ADAPTER.md` for proof tests and remaining gates.

C10-04C4 adds exact RFI/submittal metadata application with lifecycle validation,
existing link/parent checks, parent-cycle refusal and guards against cascading
deletion of links or attachment references. New relationship/attachment envelopes
remain unsupported. No document operation or customer activation is added. See
`C10-04C4-RFI-SUBMITTAL-ADAPTERS.md` for evidence and remaining work.

C10-04C5 adds RFI-task/submittal-task relationship envelopes with schema-13
local-only immutable identity bindings. Restart/retry-safe explicit unlinking
permits dependent deletion without cascades; ambiguous, missing, cross-project
or locally edited links fail closed. Sync remains disabled. See
`C10-04C5-TASK-LINK-ADAPTERS.md`; attachment/file adapters are the next slice.

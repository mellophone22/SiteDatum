# C10-04D — Versioned keys and recovery proof

Status: format foundation independently approved with conditions on 2026-10-10;
source/test condition remediation locally verified on 2026-10-11; release/version
negotiation remains an activation gate. Retirement design NOT approved for
implementation; founder policies approved on 2026-10-11, protocol proofs pending.
Not accepted or activated.
Customer Sync remains disabled. No command, UI, transport, hosted deployment,
release, activation or cost is introduced.

## Founder decision and security boundary

On 2026-10-10 the founder approved an encrypted recovery file plus a separately
held OS-generated 256-bit code, a capture date and saved checkpoint minimum.
An authentic older file cannot prove the latest cloud state after total device
loss. The new printable encoding and wire format require independent review.

KeyRing binds owner/workspace UUIDs and selects the exact authenticated record
or checkpoint version. Fresh keys come from the OS CSPRNG. New rotations increase
the version; SDREC002 can represent gaps, with at most eight retained keys.
Capacity refuses rotation, never silently
retires history keys. Mixed-version queue, paging, conflicts and safe apply retain
old-key reads; new edits and fresh KeepLocal resolutions use the active version.
The single-array WorkspaceKeys adapter is legacy low-level proof compatibility,
not scope-bound runtime authority. Future lifecycle callers must use KeyRing.

Keyrings have no Debug/Serialize implementation and use Zeroizing secret buffers.
Windows Credential Manager stores a bounded canonical keyring in an opaque
owner/workspace account. Writes are verified using a fresh handle. Non-Windows
protected storage refuses. No key/code goes into SQLite, logs, repository,
diagnostics, provider rows, licensing, billing or telemetry. HKDF 0.12.4 moves
from dev-only to native dependencies without changing its locked version.

## Rotation and restart

Protected orchestration requires a current live, backup-bound applied receipt,
reads all snapshots and persists/read-verifies a fresh key before staging.
A previously retained key is reused after failed preparation.
The staging transaction refuses pending ordinary mutations, unresolved conflicts,
partial pull pages, empty workspaces, untracked live changes and unsafe bounds.
A data-version check guards a concurrent commit between the live proof and TX.

All current records, including tombstones, are decrypted under their exact
retained version and resealed unchanged under the active key. Envelopes, outbox,
committed baselines and replacement encrypted checkpoint journal commit all-or-
nothing. Live project rows are unchanged. Exact bytes survive reopen and partial
receipt retries. New ordinary edits stay blocked until a verified covering
checkpoint clears the journal in the same transaction.

Schema 15 adds a local-only journal: workspace UUID, key version, counter,
through cursor and encrypted checkpoint bytes. Verified migration backups remain
mandatory. Prior schema upgrade and newer-schema refusal fixtures were updated.
No hosted schema changes are made.

This is fail-closed staging, not a complete hosted rotation/revocation workflow.
Cross-device conflicts can need later explicit resolution; no pending work or
keys are discarded automatically. Rotation cannot erase secrets or plaintext
already retained by a previously authorized/compromised device. Full-keyring
and anchor device transfer, signed hosted compaction and remaining C10 UX/
operational integration are still required before activation.

## Recovery code and original SDREC001 wire format

This section records the independently reviewed original layout. The new writer
and compactable semantics are specified in the SDREC002 section below.

The code contains 256 OS-random bits, not a human password. Printable form:
SDR1- plus nine uppercase eight-hex-digit groups (85 characters total).
The first 64 hex characters encode the secret; the final eight encode the first
four bytes of SHA-256(protocol label || secret). The checksum detects
transcription errors, not authentication or password strengthening.
Keep the code separate from the file; never send it to SiteDatum.

The encrypted file is 241–493 bytes for 1–8 keys:

- Header, 96 bytes: SDREC001 (8), owner UUID (16), workspace UUID (16),
  random salt (32), random XChaCha nonce (24).
- HKDF-SHA-256(secret, salt) uses info
  sitedatum.sync-v2.recovery-file.v1 || owner UUID || workspace UUID.
- XChaCha20-Poly1305 authenticates the entire header and encrypts the body,
  with a 16-byte tag.
- Body: capture Unix date (8), checkpoint counter (8), through cursor (8),
  manifest digest (32), canonical keyring (37 + 36 bytes/key).
- Strict canonical integer/length/version ordering, scope, magic, date and
  safe-integer bounds are checked. This is a new application-specific protocol,
  not a claim of an independently audited standard.

The runtime export wrapper requires current live, verified applied evidence and
a safety backup. It does not substitute a provider-asserted floor for live proof.
Wrong scope/code, every-byte tampering, malformed lengths and future capture
dates fail. A trusted independently retained newer minimum rejects a stale file.
Installing the minimum changes only the anti-rollback floor and clears stale
applied evidence: it does not advance the pull cursor or claim application.
RecoveredKeys always reports proves_latest_state=false.

Opaque owner/workspace IDs are visible in the header; keys, capture date and
checkpoint remain encrypted. The customer-held file is excluded from replicated
records and CSV/support export. create_new and sync_all prohibit overwrite;
a failed write may leave a partial newly created file and never reports success.

Loss of all devices and both recovery artifacts makes cloud ciphertext
irrecoverable. Without a newer independent anchor, a saved file cannot establish
changes after capture. Future UX must show the date and explain both limitations.

## Verification and acceptance gate

Final local Rust suite: 165 unit + 27 integration tests passed (192 total).
The explicitly invoked Windows protected-storage test also passed: it created
only a unique fictional credential, verified replacement/fresh-handle reads,
deleted its entry and confirmed it could no longer be loaded.
Frontend regression: 113 tests passed; lint and production build passed.

Focused tests cover transcription errors, every-byte tampering, wrong code/
scope/date/length, stale and equal-counter floors, strict keyrings, retained
history/capacity, mixed-version reads, active-version writes, tombstones,
atomic rotation/reopen/retry, injected failure, missing keys, durable floors,
overwrite refusal and refusal after untracked local changes.
No customer recovery artifact or real credential was generated by the tests.

The 2026-10-10 supplemental independent review conditionally approved this
slice with no High/Critical finding. The following remediation is verified:

- M-1: the unscoped array adapter is cfg(test)-only; write_version is required,
  and missing/mismatched authority refuses ordinary staging, KeepLocal choice
  creation and replay into the resolution outbox. Read-only history still works.
- L-2: known-size plaintext keyring/body and code-display buffers are preallocated
  before secret bytes enter them, avoiding their former growth/reallocation.
- L-4: the malformed-keyring test is named for what it actually tests; absent
  Debug/Serialize remains a source/type property, not claimed test evidence.
- L-3: failed new-file writes explicitly warn that the file may be incomplete,
  is not confirmed recovery, and require another filename for retry. No automatic
  deletion, overwrite, path exposure or raw error logging is added.
- L-1 partial: SQL and file-storage failures now have a separate content-free
  storage category. Full validation/credential/freshness UX taxonomy remains an
  activation prerequisite rather than being claimed complete.
- I-3: receipt rehearsal documents its synthetic sequence sentinel assumption.
- I-4: GitHub Windows CI now explicitly invokes the ignored fictional protected
  keyring drill; retained CI-step evidence is pending the new run.
- Containment regression checks the current source boundary and behaviorally
  denies MetadataSync for all plans/freshness modes. Source inspection is not
  represented as a general transitive Rust call-graph analysis.

Remediation local evidence: production cargo check and formatting passed;
168 unit + 28 integration tests passed (196 total); the explicit fictional
Windows storage drill passed; 113 frontend tests, lint and build passed.
The patch review identified a persisted-resolution queue bypass; its sink guard
and negative tests were added and the entire Rust suite rerun successfully.

Historical M-2 state: eight retained keys permitted seven rotations, and SDREC001
requires versions contiguous from one. Do not issue customer recovery files
or activate Sync before a founder capacity/compaction decision is documented
and tested. Possible decisions are a larger explicit finite retention budget,
a reviewed compactable format, or acceptance of a disclosed seven-rotation
budget. None was selected implicitly by the remediation.
I-1 structural recovered-vs-current state and I-2 code-entry normalization remain
C10-05 consuming-UX requirements. Earlier hosted and operational gates remain.

Acceptance is still blocked on the compaction gates below; see
C10-04D-CLAUDE-REVIEW-INSTRUCTIONS.md. These proofs do not replace hosted restore,
revocation, consent/deletion, final two-computer or operational drills.

## Safe compaction decision and format foundation — 2026-10-10

The founder selected safe compaction rather than merely raising the lifetime
rotation budget. This authorizes compactable-format work, not automatic deletion
of historical dependencies. The format-only native slice is implemented;
the actual retirement protocol remains blocked on the gates below.

SDREC002 uses the same 96-byte header, encrypted capture date/minimum and bounded
keyring lengths (241–493 bytes), but a separate HKDF domain:
`sitedatum.sync-v2.recovery-file.v2 || owner UUID || workspace UUID`.
The keyring retains explicit version IDs in strictly increasing order;
each is positive and at most i32::MAX, and active must equal the highest retained
ID. Gaps and a first ID above one are representable, with at most eight keys
resident at once. Versions are never renumbered or reused. The first retained
version is the first encoded entry, not inferred from count. The numeric
version maximum still refuses safely; this is not an infinite counter claim.

The new writer emits only SDREC002. SDREC001 remains a strict read-compatible
format with its original encryption label and contiguous 1..active parser;
new sparse semantics are not retroactively applied to old files. SDR1 printable
code/checksum semantics and checksum label remain unchanged. Magic is bound by
both encryption-domain selection and whole-header authentication, so changing
001 to 002 or vice versa does not migrate a file.

Protected storage writes `SDKR0002 || canonical keyring`, with preallocated
Zeroizing buffers and fresh-handle verification. The existing opaque credential
account is preserved; untagged legacy rings are read strictly without automatic
rewrite. An explicit save writes the tagged format. Older proof binaries cannot
read tagged/compacted encodings; fail-closed client/version negotiation is required
before eventual activation. This is not backwards write compatibility.
Exact bounded frame lengths distinguish legacy and tagged protected payloads;
an owner UUID beginning with SDKR0002 remains a valid legacy owner. A malformed
tagged payload is never retried using legacy parsing.

### Format support and version-mismatch handling — 2026-10-11

The minimum format-capable application **build** is the reviewed
`579cfedc2c5f670db22ce440a15e03f8b44f4c03` implementation or a validated successor:
it reads SDREC001 and SDREC002, and untagged legacy or SDKR0002 protected rings.
This is not a claim that the published 1.4.4 installer contains that build.
No customer application release supporting these formats has been assigned.
Before activation, bind this minimum build and the diagnostic remediation to an
exact published application version and enforce that floor in client/transfer
negotiation; do not infer support from the current unchanged 1.4.4 source label.
That release/negotiation prerequisite remains open, not closed by this patch.

Exact bounded tagged frames with a decimal version above 0002, and matching-scope
recovery-file frames with a decimal version above 002, now refuse with
`SYNC_V2_RECOVERY_NEWER_FORMAT`. The message explicitly says the material only
**appears** newer and has not been verified: markers are unauthenticated hints,
never import, migration, freshness or key-removal authority. Guidance says to
use a supporting version, keep originals/local work and not recreate the ring.
Malformed known formats and supported-format authentication failures remain
generic refusals. Legacy UUID/tag collisions still dispatch by exact length.
An already-installed older binary cannot be retroactively given this diagnostic;
downgrades must be blocked by future release/format negotiation before activation.

Legacy file generation is now structurally test-only (`RecoveryFileFormat::Legacy`
is cfg(test)); the production export chooses Current. A regression exercises all
eight resident-key counts and verifies exports are SDREC002 and still readable.
The create-new byte-file persistence helper is not a legacy encryptor: it can
preserve existing legacy ciphertext without decrypting or changing its format.

Tests exercise legacy-file byte tampering and read compatibility, both magic
substitutions, high/sparse IDs, duplicates/descending/zero/missing-active IDs,
the protocol integer ceiling, and protected legacy/tagged/sparse roundtrips.
A wire-only fictional fixture cycles beyond seven rotations after manually
removing dependencies inside the test. That proves representation, NOT safe
retirement. Production still has no key-removal method. Eight resident keys
still refuse additional rotation until a reviewed retirement operation exists.

### Retirement protocol requirements (not implemented or approved as runtime policy)

The current snapshot applied receipt is insufficient retirement authority.
Before any production key removal, the implementation must prove:

1. A complete ciphertext census covers snapshots, outbox, committed baselines,
   all pull history, partial pages/checkpoints, every conflict candidate including
   resolved rows, rotation journals, hosted changes/checkpoints and recovery
   generations. No local live project record, activity or normal file is deleted.
   The local schema census must explicitly visit every non-null value in all
   twelve columns below, including resolved conflict rows. NULL is absence,
   not permission to skip a populated optional branch:

   | Local table | Ciphertext columns |
   | --- | --- |
   | `sync_v2_record_snapshots` | `envelope` |
   | `sync_v2_outbox` | `envelope` |
   | `sync_v2_committed_bases` | `envelope` (nullable) |
   | `sync_v2_pull_history` | `envelope` |
   | `sync_v2_pull_parts` | `envelope` |
   | `sync_v2_pull_batches` | `checkpoint` |
   | `sync_v2_rotation_batches` | `checkpoint` |
   | `sync_v2_record_conflicts` | `local_envelope`, `remote_envelope`, `baseline_envelope` (nullable), `pending_envelope` (nullable), `resolution_envelope` (nullable) |

   `checkpoint_digest` in anchors/receipts/acknowledgements is a hash, not
   ciphertext. SQLite safety backups, customer-held recovery files, hosted
   history/checkpoints and any future archives are additional dependencies,
   not accounted for merely by querying these twelve live columns.
2. A replacement authenticated checkpoint covers current records/tombstones
   under retained keys. Every active device signs a durable **applied** receipt
   for that exact owner/workspace/counter/cursor/digest and retained-key floor.
   Existing staging acknowledgements and provider assertions are insufficient.
3. Hosted compaction commits an authenticated history/bootstrap floor, rejects
   retired-version writes and cannot omit an active device acknowledgement.
   A device absent from the barrier cannot be silently treated as acknowledged.
4. Exact retry semantics are preserved. Current pull history compares original
   envelope bytes; re-encrypting it in place is not compatible. An authenticated
   floor and reviewed bootstrap/replay rules must precede history migration.
   Any local-only retained history archive needs its own versioned authenticated
   encoding and tested access path, not a silently altered record envelope.
5. Historical backups/recovery generations remain usable under a documented
   key-preservation or archive migration policy. Customer-held files cannot be
   discovered, overwritten or deleted automatically. A freshly verified recovery
   generation and explicit consequence review precede retirement.
6. SQLite, protected storage, device acknowledgements and recovery generation
   use a crash-safe ordered journal. Restart can never leave data requiring a
   key that was already removed; failures retain the old ring and local work.
7. A durable quiesce-and-fence epoch joins the census/barrier to retirement.
   Concurrent enrollment, device-roster changes/revocation, restore, Sync writes,
   new conflict candidates and recovery-generation changes must either be fenced
   or invalidate the barrier and restart preparation. The hosted and local
   compare-and-commit checks bind the same epoch, roster digest, checkpoint and
   key floor. A census completed before an unfenced mutation is not evidence.
   Restarts fail closed before any key removal, revalidate dependency versions
   and retain the old ring on uncertainty. Ordinary local work remains usable:
   this is not permission to lock project editing indefinitely; edits must
   safely abort/postpone retirement rather than silently be lost.

### Separate barrier state and trust model (plan only; no migration)

R-1 is confirmed: `sync_v2_device_acknowledgements` (migration 0012) and
`sync_v2_applied_receipts` (0014) each have PRIMARY KEY(workspace_id). They
represent this local stream/device only. `applied_sync_v2_receipt` checks the
receipt device against the bound stream; its backup/live proof is not an
all-device certificate. Writing another device into these tables would overwrite
or misbind local evidence. Neither is an all-active-device barrier today.

Preserve these existing semantics. The proposed next reviewed protocol slice
needs **separate** retirement-epoch, roster and signed per-device receipt tables,
plus the corresponding hosted boundary. A receipt identity must include at
least (owner, workspace, retirement_epoch, device), not only workspace; a future
SQLite migration must enforce this without treating migrated staging/applied
rows as signatures. Historical epochs cannot overwrite current evidence, and
signatures bind the exact checkpoint, roster epoch/digest, proposed retirement
set and canonical retained-key version set/digest, as well as the key floor.
A scalar floor alone cannot describe sparse retained IDs. Signed proof issuance
must preserve the exclusive local database authority already required by safe
apply and recheck census/checkpoint/epoch in the locked snapshot. Hosted
finalization must serialize with current record writers and membership changes,
not merely count acknowledgement rows observed earlier.
This is a source-backed alternative to changing the current tables' primary
keys; the independent reviewer must review that choice before implementation.

The active-device set must derive from verified owner-authorized enrollment and
revocation history and device signing identities, not an unverified provider
list. Provider omission, roster rollback, unknown/duplicate devices or a missing
signature fail closed. How total-loss recovery re-establishes roster trust is
also a required proof, not supplied by a recovery-file capture minimum.
The owner-only authority policy was approved on 2026-10-11 below. Its signed
history, completeness, rollback resistance and total-loss trust bootstrap
still require protocol design, tests and independent review.

Future removal must consume a private validated `RetirementBarrier` capability
(name provisional), constructible only after roster, signed durable applied
evidence, full census, hosted floor, preservation policy and epoch fence verify.
There must be no constructor from an ordinary applied receipt, staging row,
provider assertion or recovery file. No such type or removal API exists now.

### Remaining protocol proofs and founder decisions

Before implementation of removal, review a standalone protocol slice specifying:
the census and adversarial nullable/resolved-row tests; separate signed barrier
and roster storage/migrations; hosted bootstrap floor and retired-write denial;
exact-retry-compatible archive encoding; full-ring-plus-floor device transfer;
historical backup/recovery dependency preservation; and multi-store ordered
journal with crash and concurrent-fence tests. Existing local rotation staging
and single-device backup-bound receipts do not provide these mechanisms.
The hosted record/lifecycle proof exists under `proofs/sync-v2-identity`; the
missing boundary is retirement/compaction, not all hosted schema or endpoints.

The following were the five policy questions presented before protocol design:

1. Offline device: block retirement until every active device applies, or an
   explicitly reviewed rebootstrap/exclusion flow that preserves unsynced work.
2. Historical recovery generations: preserve old generations or explicitly
   supersede them under a disclosed, independently reviewed migration policy.
3. Backups: retain required keys or offer verified explicit migration/preservation
   without silently changing/deleting customer-held backups.
4. Cadence/consent: whether retirement is manual per event or may ever be automatic,
   and the exact consequence confirmation required.
5. Roster authority: who may enroll/revoke/exclude a device and how clients verify
   a complete current roster, including total-device-loss recovery.

### Founder-approved policies — 2026-10-11

The founder explicitly stated: "Yes, I approve all 5 policies", approving the
five conservative recommendations presented with commit 60adf45:

1. **All active devices must acknowledge.** Block retirement until every active
   device supplies the required verified durable applied proof. Never
   automatically exclude a device because it is offline or stale. Waiting can
   be indefinite; local work remains available and retirement is postponed.
2. **Historical recovery files remain usable.** Preserve their original
   recoverability for the material/state they captured. Do not automatically
   invalidate or supersede them. This does not make an old file a latest-state
   proof or give it keys created after capture.
3. **Historical backups stay untouched and recoverable.** No silent migration,
   overwrite or deletion. Preserve necessary recovery dependencies; if the
   protocol cannot prove this safely, postpone retirement. No archival key
   format, key relocation or backup rewrite mechanism is approved implicitly.
4. **Every retirement requires explicit per-event confirmation.** No automatic
   retirement. Policy approval is not consent to a particular future retirement;
   its consequences and preservation evidence must be reviewed separately.
5. **Only the owner authorizes enrollment/revocation.** Clients must verify signed
   device history and a complete trusted roster; a provider-supplied list alone
   cannot authorize destruction. No automatic offline-device exclusion is
   permitted under the guise of revocation. The future explicit revocation flow
   must preserve local/unsynced work and explain consequences; it is not supplied
   by this policy record.

This closes the founder-choice portion of R-3, not its protocol/implementation
proofs. Preserving old recovery/backups may prevent safe retirement indefinitely;
the design must acknowledge that outcome instead of promising unlimited rotation
or silent cryptographic erasure. Removing a key from a future active ring would
not erase historical copies retained for recovery or on revoked devices.
The eight-key capacity still refuses further rotation today. The approval is for
the next design phase only, not implementation of removal, a schema migration,
deployment, main merge, release, any cost or Sync activation.

Missing source boundaries: no hosted applied-device acknowledgement/compaction
action exists; current device transfer carries one key, not a full ring and
trusted floor; backups/conflicts/history keep old ciphertext. Completing them
is a separate protocol vertical slice, not supplied by sparse encoding.
The approved policies reject automatic offline-device rebootstrap/exclusion and
silent historical recovery invalidation. The corresponding preservation,
roster, fence and journal protocol still needs independent design review before
implementation can remove keys. The default remains no removal and fail-closed
capacity refusal. No runtime code changed when these policies were recorded.

Local format validation: production cargo check, focused native tests, complete
Rust regression, formatting and the unique-fictional Windows storage drill.
Final local results: 172 unit + 28 integration tests passed (200 total), the
explicit protected-storage test passed, and 113 frontend tests, lint and build
passed. The patch review's legacy UUID/tag collision was reproduced, fixed
with exact framing, and covered by its own regression test before final checks.
No customer artifact was issued, hosted service altered, credential migrated
or key retired. Independent review of SDREC002 and the retirement plan is the
next gate at that point; see the updated review instructions. The subsequent
2026-10-10 report conditionally approved the format only and rejected retirement
implementation. Its conditions and the 2026-10-11 follow-up are recorded above.

Follow-up local verification on 2026-10-11: production `cargo check --locked`,
`cargo fmt --check`, focused key-recovery tests (23 passed, one ignored), full
`cargo test --locked` (174 unit + 28 integration = 202 passed; four unit and
one integration ignored), and the separately invoked fictional Windows protected
keyring roundtrip (one passed). `npm test -- --run` passed 113 tests across 35
files; lint and TypeScript/Vite build passed. No UI surface changed, so visual
review is not applicable. Existing tamper, legacy reader, collision, scope and
rotation-capacity tests remain intact alongside the two added regressions.
Remote CI for the exact follow-up commit must be reported separately, not inferred
from these local results. The reviewed 579cfed GitHub run 38067599031 completed
successfully; its GitLab pipeline 2933840958 failed due to CI quota exhaustion.


## 2026-10-11 preservation-first review corrections and D1 proof

Claude's review of packet 8be1f5e endorsed a design direction, not implementation.
The corrected [packet](c10-04d-retirement-review-20261011/hardening.md) adopts
preservation completeness as the proposed safety property: all retained historical
versions remain readable, even when legitimate pulls add dependencies after
preparation. The census is diagnostic, not pruning authority. The barrier must
prove complete membership and readiness, not a stable dependency set.

[D1](c10-04d-retirement-review-20261011/d1-proof.md) is complete only as a fictional,
test-only twelve-column census and separate read/write authority contract proof.
Twelve focused tests passed; full Rust regression passed 214 tests with five ignored;
113 frontend tests, lint, build, production cargo check and formatting passed. No
existing production trait, hosted SQL or migration changed. The existing containment
regression remains intact. This is not historical manifest discovery, vault storage,
distributed barrier or retirement implementation.

D3 is a hard predecessor of D6: a reviewed, implemented preservation format covering
the full retained set and passed total-loss recovery drills at 1, 8, 100 and 1000
versions are required before any pruning. SDREC002's eight-key maximum and current
single-key transfer are not substitutes. D2 owner-root/rollback specification,
D4 existing hosted-lock re-keying and offline-floor semantics, and D5 ordered abort
cleanup remain unproved. See the [ledger](c10-04d-retirement-review-20261011/review-remediation.md).

Capacity disclosure: if an active computer never returns online and the owner does
not explicitly revoke it, the all-device barrier cannot complete and key capacity
cannot be reclaimed. Offline age never silently excludes that computer. Compaction
would retire keys from new writes, not erase captured decryption capability. Cloud
deletion concerns hosted ciphertext, not local keys, backups or recovery files.
These are draft future consent requirements, not a shipped retirement surface.

No keys removed, protected credentials migrated, new recovery format assigned,
service altered, main merge, release, deployment or Sync activation. D6 and D7 stay
blocked. The no-removal, fail-closed capacity baseline remains operational.

## D2 owner-root and roster trust specification draft — 2026-10-11

The [D2 packet](c10-04d-d2-owner-roster-review-20261011/hardening.md) specifies
separate owner/device authority, complete membership history, independent pins,
ordinary-restore reconciliation and FreshnessUnestablished behavior after total
anchor loss. This is a specification draft, not accepted D2 implementation.

The recommended candidate is a separately held offline owner-signing artifact;
its custody is a new mechanism decision requiring explicit founder selection
and independent review. The five preservation policies remain unchanged.
No root is issued, format assigned, schema or production/test source changed,
service deployed or key removed. Old root/private recovery material alone does
not establish latest roster freshness. Local work remains available when trust
cannot be recovered. D4/D5 publication, fencing and ordered journal proofs remain
open. D6 and D7 stay blocked; no-removal capacity refusal remains operational.

D2 checks are document/source correspondence, ten pinned Git-blob hashes,
structured-analysis/link/diagram consistency, managed readback and publication
integrity. No new D2 behavioral, cryptographic, Windows restore or concurrency
test ran. Prior D1 local/CI evidence is not validation of the proposed D2 protocol.

### D2 direction selection recorded

The founder selected the separate offline owner-signing artifact design direction
for independent review after the D2 draft at ce0bd4c. The [decision record](c10-04d-d2-owner-roster-review-20261011/founder-decision.md)
and [conditional handoff](c10-04d-d2-owner-roster-review-20261011/implementation/offline-owner-root.md)
record the scope. This does not waive technical review or authorize artifact issuance,
keys, schema, service changes or activation. The five policies are unchanged.
The next gate is the pinned independent D2 review; D6/D7 remain blocked.

### D2 independent-review semantic correction round — 2026-10-11

The report at packet 0bf2e4ae supports direction only. [D2 remediation](c10-04d-d2-owner-roster-review-20261011/review-remediation.md) proposes event-bound independent-tip acknowledgements, both membership mismatch refusals, a separately reviewed fork join, honest copied-artifact limitations, bounded epoch/retry semantics, refreshed 21-line/five-file inventory and combined-freeze disclosure. Proposed semantics are not executed controls. No source/schema/root or service changes; D6/D7 remain blocked. Current eight-key no-removal capacity refusal remains the operational fallback.

### D2 re-review N-1..N-5 refinement package — 2026-10-11

The report at 1dd73683 closes earlier conditions at specification level and requests five refinements. [Refinement ledger](c10-04d-d2-owner-roster-review-20261011/re-review-remediation.md) records the narrow fork taxonomy, MAX_SAFE_INTEGER authority counters, one admitted owner/workspace event, provider-mediated divergence residual and isolated freshness fixture. This is documentation only, awaiting independent review before canonical encoding. ForkResolution remains proposed and unaccepted; D3/D4/D5 proofs open, D6/D7 blocked. No runtime, schema, key, service or activation change.

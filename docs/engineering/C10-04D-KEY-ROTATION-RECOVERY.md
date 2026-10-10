# C10-04D — Versioned keys and recovery proof

Status: independently reviewed with conditional approval; remediation locally
verified; founder selected safe compaction, with format review and retirement
protocol still pending. Not accepted or activated.
Customer Sync remains disabled. No command, UI, transport, hosted deployment,
release, activation or cost is introduced.

## Founder decision and security boundary

On 2026-10-10 the founder approved an encrypted recovery file plus a separately
held OS-generated 256-bit code, a capture date and saved checkpoint minimum.
An authentic older file cannot prove the latest cloud state after total device
loss. The new printable encoding and wire format require independent review.

KeyRing binds owner/workspace UUIDs and selects the exact authenticated record
or checkpoint version. Fresh keys come from the OS CSPRNG. Versions are contiguous,
with at most eight retained keys; capacity refuses rotation, never silently
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

Missing source boundaries: no hosted applied-device acknowledgement/compaction
action exists; current device transfer carries one key, not a full ring and
trusted floor; backups/conflicts/history keep old ciphertext. Completing them
is a separate protocol vertical slice, not supplied by sparse encoding.
Offline-device rebootstrap and historical-recovery retention consequences need
founder approval and independent design review before implementation can remove
keys. The default until then is no removal and fail-closed capacity refusal.

Local format validation: production cargo check, focused native tests, complete
Rust regression, formatting and the unique-fictional Windows storage drill.
Final local results: 172 unit + 28 integration tests passed (200 total), the
explicit protected-storage test passed, and 113 frontend tests, lint and build
passed. The patch review's legacy UUID/tag collision was reproduced, fixed
with exact framing, and covered by its own regression test before final checks.
No customer artifact was issued, hosted service altered, credential migrated
or key retired. Independent review of SDREC002 and the retirement plan is the
next gate; see the updated review instructions.

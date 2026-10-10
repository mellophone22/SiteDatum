# C10-04B — Versioned records and encrypted local queue

**Status:** Implemented and locally verified — 2026-10-10. Customer Sync remains disabled.

## Scope

This slice supplies explicit record codecs, authenticated encryption, a durable
encrypted outbox, receipt handling, and checkpoint-covered pull staging. It has
no Tauri command, UI, network transport, or hosted deployment. Ordinary project
mutations do not call it. Local encrypted staging is separate from applying
changes to the user's project tables.

## Record protocol

Protocol and content schema version 1 use explicit field allowlists from
`SYNC-V2-DATA-INVENTORY.md`. Every field is required; nullable fields must be
represented deliberately. Unknown fields, missing fields, unsupported schema
versions, wrong scalar types, inconsistent identity, invalid domain codes,
invalid dates/UTC timestamps, and inconsistent checklist counts fail closed.
New database columns and UI fields never enter a record automatically.

| Hosted kind | Encrypted content |
| --- | --- |
| 1 | Project |
| 2 | Task |
| 3 | RFI |
| 4 | Submittal |
| 5 | RFI/task relationship |
| 6 | Submittal/task relationship |
| 7 | RFI attachment reference |
| 8 | Submittal attachment reference |
| 9 | Registered file metadata |
| 10 | Note |
| 11 | Contact |
| 12 | Activity event |
| 13 | Work item or project template, selected by an encrypted strict subtype |

Stable random envelope IDs for relationships and locally integer-keyed activity
must be assigned by later project adapters; this slice does not derive IDs from
names, content, or paths. Attachment and project references use an explicit
`workspace_relative` component array or a single `external_name` component.
Absolute paths, traversal, separators, drive prefixes, alternate streams, and
control characters are refused. Rebasing and Windows reserved-device handling
remain part of the later filesystem adapter. No document bytes are serialized.

XChaCha20-Poly1305 uses a fresh 24-byte OS-generated nonce for each new seal.
Ciphertext includes the nonce and is bounded at 262,144 bytes, including the
authentication tag. Temporary serialized/decrypted byte buffers are zeroized.
Decoded content remains ordinary local memory for validation and future apply.
Keys are borrowed from the native caller and never saved in these tables.

Associated data uses the fixed `sitedatum.sync-v2.record.v1` label followed by
the owner, workspace, record and mutation UUID bytes, protocol version (u16),
kind (u8), key version (u32), expected server version (u64), and tombstone byte;
integers use big-endian encoding. The service's returned server version must
equal expected version plus one. The later transport adapter can reconstruct
the expected version from that returned version. Positive key versions and
JavaScript-safe version bounds match the existing trusted hosted bridge.

Tombstones encrypt an exact schema-versioned null content body and retain their
random record ID, revision, and authenticated tombstone flag. No compaction is
performed.

## Durable staging

Migration 12 adds four local-only tables: stream scope/cursor, encrypted record
snapshots, encrypted outbox, and device staging acknowledgements. They are not
replicated record content or part of complete CSV exports. The standard verified
pre-migration backup and transactional migration runner protect upgrades.

A local candidate and its outbox entry are stored in one SQLite transaction.
The random mutation ID and exact ciphertext survive process restart. An exact
queue retry succeeds without another entry; a different pending candidate for
the same record is refused without replacing the first. Receipts are processed
atomically, match the exact mutation and next revision, and allow exact retry.
A failed receipt anywhere in a batch leaves every pending mutation intact.

Pull staging validates all incoming ciphertext before a transaction, checks
contiguous sequence numbers and expected revisions, then verifies the complete
prospective manifest against the encrypted checkpoint. Snapshot changes, pull
cursor, checkpoint high-water anchor, and device acknowledgement commit
together. An exact checkpoint-covered redelivery is idempotent. Corruption,
omission, reordered sequences, rollback, altered redelivery, scope changes,
and pending local edits preserve all durable state.

These acknowledgements certify **encrypted staging**, not successful project
application. They must not authorize server tombstone compaction. C10-04C must
establish the separate applied/recoverable acknowledgement boundary first.

## Verification

Thirteen new focused Rust tests cover:

- explicit fixtures for all 13 kinds and the project-template subtype;
- authenticated header substitution, wrong key, corruption, nonce diversity,
  plaintext absence, schema skew, unknown/missing fields, and type/ID failures;
- content-free tombstones and portable-reference refusal;
- restart with exact ciphertext, receipt retry and all-or-nothing receipt failure;
- injected snapshot/acknowledgement storage failure with transaction rollback;
- durable cursor/anchor/ack state, exact redelivery and altered-device refusal;
- schema-11 upgrade preserving a fictional local note and checkpoint anchor;
- corrupted/reordered/omitted pages without partial writes;
- two fictional offline devices retaining competing candidates, followed by
  tombstone retention and older-checkpoint refusal; and
- pending-candidate replacement and extreme untrusted versions without panic.

Full Rust unit/integration tests, frontend tests, lint, production frontend
build, formatting, and diff checks passed. There is no visual surface to review.
Remote CI evidence is tied to the resulting source commit.

## Remaining C10-04 work

C10-04C owns full project adapters and relationship/domain validation, verified
safety backups before project application, atomic live-table apply and outbox
integration, record-scoped conflict candidates/resolution, partial-page staging,
multiple revisions of the same record in a page, and applied-device receipts.
Key rotation/mixed-key reads, transfer/recovery anchors, and the comprehensive
two-computer/recovery matrix remain later C10-04 gates. No customer release,
production Sync deployment, or feature activation follows from this slice.

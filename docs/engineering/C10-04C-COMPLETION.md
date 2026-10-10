# C10-04C — Complete local application boundary

Implemented 2026-10-10. This closes the remainder of C10-04C after C1–C5,
not all of C10. Customer Sync remains disabled behind `SYNC_DEFERRED`.
There is no production deployment, Tauri command, UI or runtime transport.

## Full adapter coverage

All thirteen schema-1 envelope kinds now have explicit live adapters: projects,
tasks, RFIs, submittals, both task-link kinds, both attachment-reference kinds,
registered files, notes, contacts, activity, and work items/project templates.
Table/column names are compile-time constants; there is no remote SQL identifier
or broad row serialization. Strict codecs, domain rules, parent/link checks and
dependency ordering reject invalid data or implicit cascading loss. Work-item
rules include daily-report dates, transmittal numbers and checklist counts.
Template title arrays are validated before conversion to local JSON storage.
Kind 13's encrypted subtype remains immutable after tombstoning.

Existing composite links can receive opaque stable IDs through an explicit
native bootstrap. Legacy hexadecimal activity IDs map to stable random envelope
UUIDs without renaming historical database rows. Existing activity content is
immutable; explicit tombstones remove metadata only. Historical entity IDs do
not require a surviving live entity.

## Document boundary

Portable relative references are validated and rebased under the selected local
root. Windows reserved names, traversal, length, collisions and reparse points
fail closed. A missing relative file is allowed; an existing non-file target is
refused. External filename references become the non-filesystem marker
`sitedatum-unresolved:<record UUID>`, never a guessed destination. The original
portable token remains encrypted for conflict resolution, including tombstones.
Registered `copy`/`move` values are historical metadata: application never
copies, moves, opens, uploads, downloads or deletes project-document bytes.
Explicit dependent tombstones precede parent deletion, including child-first
submittal revision deletion. Ordinary file handling remains unchanged.

## Atomic live/outbox writes

`mutate_sync_v2_live` validates a native sealed candidate and commits its live
row, exact encrypted outbox bytes and snapshot together. A separate encrypted
committed baseline prevents pending edits from masquerading as server state.
Exact retries preserve mutation/ciphertext identity; another unacknowledged edit
of the same record is refused without replacing the first. SQL/domain failure
rolls everything back. Receipt processing removes only an exact matched mutation.

These native hooks are dormant. Ordinary application commands are intentionally
not wired while Sync is disabled. Later consent/device integration must connect
them with correct multi-row application transaction semantics. No keys are saved
in these tables and no new dependency or networking is introduced.

## Pages and repeated revisions

Page assembly accepts at most 100 contiguous changes per page, 10,000 per batch
and 128 MiB of encoded envelopes. Scope, safe integer bounds, strict codec and
authenticated decryption are checked before durable staging. The starting
cursor/checkpoint is immutable; exact retries are safe. Omission, substitution
or reordering preserves staged work without advancing live rows/cursor/anchor.
Buffers survive process restart. Only a complete contiguous batch reaches apply.

Every revision is checked, including several revisions of one record, before
projecting the final revision. Kind, subtype and link identity cannot change.
Exact encrypted change history proves replay without accepting substituted
intermediate revisions. Whole-set checkpoint validation uses committed server
versions, not pending candidates; copied baseline envelopes fail scope/ID checks.
Older staging without schema-14 history cannot silently qualify as exact replay.
No customer Sync was activated under those earlier schemas. Incomplete buffers
and history are retained; there is no automatic abandonment or compaction here.

## Durable explicit conflicts

The complete remote batch is authenticated and rehearsed in a private SQLite
copy before storing conflicts. Divergent live work/nonidentical pending edits
produce per-record encrypted local, remote and baseline candidates, retaining
the original pending envelope and chosen resolution. Live rows and cursor stay
unchanged while review is required; there is no silent last-write-wins.

`KeepLocal` seals a fresh mutation against the verified remote revision.
`AcceptRemote` explicitly adopts that candidate. Exact repeated choices reuse
durable bytes; opposite choices are refused. Application rechecks current local
state so stale choices cannot overwrite new work. Deletion conflicts preserve
both tombstone and surviving candidate. Resolved candidates remain retained,
marked resolved rather than deleted. The final resolved graph is rehearsed again;
incompatible choices fail atomically. Locally naming records and exposing choices
accessibly belongs to the later consent/conflict UI.

## Backups and applied receipts

Private rehearsal is followed by a verified, uniquely identified safety backup.
Live rows, snapshots/history, cursor/anchor, resolved outbox, buffer cleanup and
applied receipt then commit together. Failure rolls back all application state;
the backup remains available. Essential export/recovery stays available.

An applied receipt is distinct from staging evidence. It binds only opaque
workspace/device IDs, counter/digest, through-change sequence and the exact
safety-backup UUID. Pending mutations or unresolved conflicts suppress it. The
typed reader also requires unchanged live projections, matching scope/anchor
and an existing verified recovery backup, otherwise returning no usable proof.
It exposes no path/content. It is local evidence, not a signed hosted compaction
request. Hosted acceptance, replacement checkpoints and safe compaction remain
separate unimplemented gates.

## Schema, privacy and validation

Schema 14 adds local committed baselines, page parts/history, encrypted retained
conflicts, applied receipts, subtype and activity identity bindings. Standard
verified pre-migration backup and transactional upgrades protect schemas 12/13.
The complete CSV manifest identifies schema 14 but excludes internal Sync state.
Protocol schema stays 1: no new content category, provider-facing field, secret,
billing change, document transmission or customer activation is added.

Tests cover all adapters/tombstones, immutable IDs, file sentinels, missing and
external references, domain/cascade/cycle rejection, repeated revisions/replay,
two fictional offline devices converging through explicit choices, conflict and
page database reopen, corrupt checkpoints, wrong-key records, copied baselines,
extreme counters, stale decisions, injected atomic failures, and backup-bound
receipts. Migration tests preserve protocol state/identity and verified backups.
Frontend tests (113), lint, TypeScript/Vite build, nine-page commercial-site and
six release-manifest checks pass. No UI changes require a visual review. Full
Rust and CI results are recorded against the resulting commit separately; local
success is not assumed to prove hosted CI. Final local Rust results: 151 unit
tests and 27 integration tests pass (four existing environment-dependent tests
ignored); 52 of the unit tests exercise safe application. `cargo fmt --check`
passes. Both existing review branches receive this package, not `main`.

## Remaining beyond this section

Mixed-key reads/rotation, transfer/recovery-anchor integration and the broader
recovery/compatibility matrix remain C10-04 gates. Signed hosted applied receipts
and compaction proof, remaining C10-03 reconstruction/security evidence, C10-05
consent/recovery UI, C10-06 operations drills and C10-07 fictional Windows beta
and founder release approval remain gated. This closeout enables none of them.

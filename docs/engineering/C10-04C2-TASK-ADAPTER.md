# C10-04C2 — Backed-up task adapter and dependency validation

Implemented 2026-10-10. This is a second **partial C10-04C slice**, extending
the C10-04C1 safe-apply boundary. Customer Sync remains disabled; no UI, Tauri
command, transport, dependency, schema migration, production deployment, or
release publication is added.

## Supported application

The internal method is now `apply_sync_v2_supported_records`. It supports
schema-1 tasks (kind 2), notes (10) and contacts (11), retaining the private-RAM
rehearsal, verified local safety backup, exact live-baseline comparison and single
live/staging transaction from C1. All other kinds still fail closed.

Tasks preserve all 14 allowlisted fields, including waiting state, dates,
priority, category, related contact and timestamps. Incoming task titles must be
nonblank; status/date/waiting-domain checks come from the authenticated codec
and SQLite constraints. Projects must already exist locally. A nonnull related
contact must exist locally or be created by this same verified page.

Existing RFI/submittal relationships must agree with the task's incoming project;
changing a linked task's project cannot produce a cross-project relationship.
Task tombstones respect SQLite RESTRICT relationships. They cannot orphan RFI
or submittal work. Contact tombstones cannot remove a still-referenced contact.
These checks preserve existing rows and stream state on refusal.

All baseline comparisons happen before SQL changes. The local adapter orders
contact upserts before task upserts, then note upserts; note/task deletions precede
contact deletion. This permits a task to be detached/deleted in the same page as
its contact without depending on provider row order. Authenticated sequence,
snapshot revisions, checkpoint manifests and stream cursors retain their original
verified order; local dependency ordering does not reorder the remote stream.

Supported record IDs and project/contact references must use canonical UUID
text. Noncanonical identity is refused rather than silently normalized when
mapping into SQLite's case-sensitive text keys.

## Verification

Six added focused scenarios (16 safe-apply tests total) cover:

- Task before contact in the stream, complete field preservation, pre-apply backup,
  and contact-before-task tombstones applied in safe local dependency order.
- Unknown project/contact refusal without partially applying another record.
- Competing offline task edits retained with unchanged pull cursor.
- Referenced contact deletion refused, then explicit same-page detachment accepted.
- Linked task deletion and cross-project reassignment refused without orphaning
  the RFI/task relationship.
- Blank title/noncanonical identity refusal, valid updates and exact redelivery.

All fixtures are fictional. No real workspace or document is used. No screen
changed, so a UI visual review is not applicable. Full Rust regression and
frontend tests/lint/build are run before review-branch push; remote CI results
are reported from the actual runs rather than assumed.

## Remaining gates

Project adapters and safe portable-path mapping are next, followed by remaining
RFI/submittal/file/relationship/activity/register/template adapters. Durable
record-scoped conflict candidates/resolution, atomic local mutation/outbox
integration, partial paging, multiple revisions per record and independent
applied-device receipts remain incomplete. This slice still refuses the entire
page on local conflict. Staging acknowledgements do not permit hosted tombstone
compaction. Key rotation, recovery anchors, comprehensive two-device acceptance,
security review and explicit rollout consent remain later C10 gates.

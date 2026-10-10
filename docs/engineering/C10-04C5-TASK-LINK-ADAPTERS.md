# C10-04C5 — Stable task-link adapters

Implemented 2026-10-10 as a partial C10-04C slice. Customer Sync remains disabled.
No transport, commands, UI, hosted service changes, dependencies or releases.

## Identity and application

Authenticated relationship envelopes 5/6 now map to the existing RFI-task and
submittal-task composite keys. Schema 13 adds a local-only identity map containing
opaque workspace/record/register/task IDs, not keys or document content. A record
ID cannot change kind or pair; a pair cannot be claimed by another envelope or
workspace. Bindings survive link tombstones, ensuring retries target the same
pair. Unknown tombstones and adoption of unmanaged existing links fail closed.
An explicit future ownership/bootstrap policy is required for those local links.

Compile-time SQL allowlists preserve exact canonical IDs and creation timestamps.
Both referenced rows must exist and share a project. Links may precede parents
in the authenticated stream: local SQL creates parents first, then links.
Explicit link tombstones run first, permitting subsequent task/register deletion
without silent cascades. Other links and attachment references remain protected.
Same-pair revival uses its original identity; changing a pair requires a new ID.

The private-memory rehearsal, verified pre-apply safety backup, encrypted
checkpoint, conflict baseline checks and atomic cursor/live transaction remain.
No production document operation occurs. Portable CSV export excludes internal
Sync identity state; recovery backups preserve it. Existing schema-12 databases
receive the normal verified pre-migration backup before schema 13 is applied.

## Verification

Seven new link scenarios bring safe-apply coverage to 35 tests: dependency order,
restart and exact retry; explicit unlink/parent/task deletion with recoverable
mapping; immutable identities after tombstones; duplicate-pair rollback; missing,
cross-project and unknown-tombstone refusal; local-edit/unmanaged-pair preservation;
valid timestamp updates and exact deletion preserving other links. A schema-12
upgrade test verifies unchanged encrypted snapshots/cursor and schema-12 backup.
Portable-export regression checks that no internal Sync table is exported.

Full Rust, frontend tests/lint/build, formatting and diff checks precede push.
No screen changed, so visual UI review is not applicable. The prior C4 GitHub
run 38033494917 passed, including Windows packaging and corrected path fixtures.
GitLab C4 pipeline 2933042547 could not start jobs because CI quota was exceeded.

## Remaining gates

Attachment/file references 7/8/9, activity 12 and work-item/template 13 adapters;
conflict inbox/resolution; live mutation/outbox wiring; partial pages/multiple
revisions; applied-device receipts; key rotation/recovery anchors; two-device,
security and consent rollout gates remain. Staging receipts cannot authorize
hosted tombstone compaction. No customer Sync activation is implied.

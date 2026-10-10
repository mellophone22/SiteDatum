# C10-04C1 — Backed-up atomic notes/contact application

Implemented 2026-10-10 as the first **partial C10-04C slice**, not completion
of C10-04C or authorization to enable customer Sync.

## Boundary

`sync_v2_safe_apply.rs` is dormant Rust code with no Tauri command, UI, network,
hosted deployment, production change, or new dependency. Free/local workflows
are unchanged. Legacy Sync remains contained. Only schema-1 notes and contacts
are supported; every other record kind is refused before application.

## Application sequence

1. Decrypt and validate supported records; require nonblank note body/contact
   name and chronologically ordered UTC timestamps. Keep imported values exact.
2. Compare the complete live row against the previously authenticated encrypted
   baseline (or exact incoming content for safe redelivery). Refuse divergent
   local edits/deletions or colliding IDs; do not overwrite or normalize them.
3. Copy the SQLite database into a private in-memory candidate, enable foreign
   keys, and rehearse the entire checkpoint-covered pull and adapter transaction.
   Unknown/missing project references, bad manifests, pending outbox work,
   malformed revisions/pages, and SQL/trigger failures fail before backup/apply.
4. Create a uniquely named, integrity-verified local pre-apply SQLite backup using
   the existing recovery boundary. An unavailable destination blocks application.
   Backups are local and contain workspace data; never upload them as telemetry.
5. Recheck live rows and run staging plus live adapters in **one transaction**.
   Any failure rolls back live rows, snapshots, cursor, anchor and staging ack.
   Preserve the safety backup if final application fails.

Notes retain project FK validation. Contact tombstones are refused while a task
still references the contact. Accepted tombstones delete only the explicitly
selected database row, retain its encrypted protocol tombstone and leave the
pre-apply row recoverable from backup. No document/file operation exists here.
All SQL table/column names are compile-time constants; errors contain no record
values. Callers must hold exclusive local database access throughout this method.

The existing C10-04B acknowledgement still proves **staging only**. This slice
does not issue an applied-device receipt or authorize hosted tombstone compaction.
Exact retries may create another uniquely named safety backup, but do not alter
record content or advance the cursor twice.

## Verification

Ten focused tests cover verified pre-apply backup and exact retry; preservation
of competing local edits; unavailable backup; invalid second-record reference;
recoverable tombstones; contact/domain validation; SQL trigger rejection; wrong
manifest and unsupported kinds; final adapter transaction rollback; fractional
UTC ordering. Fixtures are fictional and isolated temporary databases only.

No screen changed, so a UI visual review is not applicable. Full Rust regression,
format checks, frontend tests/lint/build, and review-branch CI are required before
handoff; their actual results are reported separately, not assumed here.

## Remaining C10-04C work

- All other project/task/RFI/submittal/file/relationship/activity/register/template
  adapters, complete domain/reference validation and safe portable-path mapping.
- Atomic live mutation/outbox integration and independent applied-device receipts.
- Durable encrypted **record-scoped** conflict candidates and explicit resolution.
  This slice conservatively refuses the whole page on a conflict; the current
  local row and encrypted request remain available, but no conflict inbox exists.
- Partial-page staging and ordered multiple revisions of one record per page.

Key rotation, mixed-key reading, transfer/recovery anchors, broader two-device
acceptance, security review and consent/retention/recovery rollout remain later
gates. No production activation or release publication is included.

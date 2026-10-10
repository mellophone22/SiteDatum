# C10-04C4 — RFI/submittal metadata adapters

Implemented 2026-10-10 as a **partial C10-04C slice**. Customer Sync stays
disabled. No transport, command, UI, production deployment, new dependency,
schema migration or release publication is added.

## Supported application

The backed-up safe-apply boundary now supports RFI (kind 3) and submittal (4)
metadata alongside projects, tasks, notes and contacts. The new
`sync_v2_register_adapters.rs` uses two compile-time table/column allowlists,
never `SELECT *` or provider-supplied SQL identifiers. All 21 RFI and 17 submittal
fields are mapped exactly, including the six RFI PDF-specific values, submittal
disposition, parent, revision, response dates and timestamps. No input cleaning
silently changes imported content. Errors omit record/provider values.

Application uses existing RFI/submittal validators in addition to the
authenticated schema codec and SQLite constraints. Blank required fields,
missing lifecycle recipients/responses, invalid dates, invalid resubmission
state and status/disposition disagreement are refused. Closed submittal content
must include its authenticated disposition and response date; application does
not invent a disposition or replay local UI actions.

Project references must exist locally or be applied in the same verified page.
Existing task links must stay within the same project. Revision parents must
exist and belong to the child's project. The complete local submittal parent
graph is checked for self-parenting and cycles, including existing descendants
outside the page. IDs/references remain canonical, exact UUID text.

Foreign keys are deferred only within the adapter transaction so a verified
submittal child can precede its parent in the stream. Final FK, cross-project and
ancestry checks run before commit. Checkpoint validation and authenticated stream
cursors/revisions are unchanged. The private-RAM rehearsal, verified local safety
backup, baseline conflict refusal and atomic staging/live transaction remain.

RFI/submittal tombstones are refused if existing task links or attachment
references would cascade. A submittal with revision children is also refused.
Only an unreferenced explicit database row may be deleted; its encrypted
tombstone and recoverable pre-apply backup remain. No document/file is opened,
read, created, moved or deleted by production adapters.

## Verification

Eight new focused scenarios bring safe-apply coverage to 28 tests:

- Full-field preservation, child-before-parent order, pre-apply backup and retry.
- Invalid recipients/required text/response dates/dispositions and unanswered
  closed RFI refusal with no partial page application.
- Missing/cross-project parents, new cycles/self-parenting and cyclic changes
  to an existing parent graph refused with unchanged prior state.
- Unknown projects and competing local RFI/submittal edits preserved.
- Attachment-cascading deletion refused; fictional document sentinel and both
  attachment-reference rows remain unchanged.
- Linked register project reassignment and cascading link deletion refused.
- Duplicate RFI number and parent deletion refused without losing existing rows.
- Valid closed revisions and unreferenced tombstones, with old rows in the backup.

Tests use fictional records and isolated temporary fixtures. Full Rust regression,
frontend tests/lint/build and formatting are required before push. CI results are
reported from actual review-branch runs. No screen changed, so visual UI review
is not applicable.

Prior C3 GitHub run `38032214279` exposed three Windows fixture assertions that
compared raw `TEMP` 8.3 aliases against production-canonical long paths. C4 fixes
the isolated mapping fixture to obtain its expected root from `selected_root`,
matching the production validation path. No production mapper behavior is
weakened or changed; the replacement Windows CI run must verify the correction.

## Remaining gates

New relationship envelopes (5/6), attachment/file references (7/8/9), activity
(12) and work-item/template (13) adapters remain unsupported. This slice validates
and preserves existing links; it does not implement new linked-record ID mapping,
relationship removal or attachment synchronization. Revision trees cannot be
silently cascade-deleted; explicit dependency deletion ordering is later work.
Simultaneous project moves of already linked task/register rows may be
conservatively refused until an explicit relationship adapter reconciles them.
Pre-existing invalid parent/link graphs block application for review rather than
being silently repaired.

Record-scoped conflict inbox/resolution, live mutation/outbox integration,
partial-page and multi-revision staging, applied-device receipts, key rotation,
recovery anchors and two-device/security/consent rollout gates remain pending.
Staging acknowledgements cannot authorize hosted tombstone compaction.

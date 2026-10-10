# C10-04C3 — Project adapter and local portable-root mapping

Implemented 2026-10-10 as another **partial C10-04C slice**. Customer Sync stays
disabled. No Tauri command, UI, transport, hosted deployment, dependency, schema
migration or release publication is introduced.

## Supported application

The backed-up safe-apply boundary now supports projects (kind 1), tasks (2),
notes (10) and contacts (11). Project fields are explicitly mapped, preserving
the existing project validator and SQLite domain/uniqueness/FK constraints.
The encrypted `portable_root` becomes a **local-only** `project_path` under the
computer's already selected project root. Absolute roots are never sent to the
provider or added to encrypted replicated fields.

The selected root must be an existing, readable Windows local/UNC directory.
The production mapper uses the existing root validator and rejects a root
reparse point before canonicalizing it. Linux CI uses an absolute temporary
directory only to exercise portable proof fixtures; the product stays Windows.

`sync_v2_project_paths.rs` accepts only nonempty `workspace_relative` components.
It refuses external roots, the shared root itself, traversal/drive/separator
escapes, control/Windows-invalid characters, trailing dots/spaces and reserved
device names (including superscript COM/LPT forms and console device aliases).
Components are bounded to 255 UTF-16 units and the mapped path to a conservative
240-unit ceiling. An existing path component must be a normal directory, not a
file, symlink, junction or other Windows reparse point. Permission failures refuse
the mapping. Path checks run again inside the adapter transaction.

No directory or document is created, moved, copied, opened, downloaded or deleted
by the production adapter. Missing directories remain missing references. New
project IDs cannot silently adopt a pre-existing directory. Existing projects
cannot have their local `project_path` redirected by a remote edit; a root/path
reassignment needs a future explicit local mapping workflow. Existing-project
paths are compared against the projected authenticated baseline before changes.
Different project IDs cannot claim the same mapped path: local path comparison
uses separator normalization and conservative Unicode uppercase folding rather
than SQLite's ASCII-only `lower`. Project-number uniqueness also remains enforced.

Project upserts precede contact/task/note application locally, so a task may occur
before its project in the authenticated stream. Project tombstones run after
dependent task/note tombstones, with all remaining FK references enforced. A
referenced project cannot be deleted silently or orphan existing work. Deletion
only affects the explicitly authenticated database row; the encrypted tombstone
and verified local pre-apply backup remain. No project folder is removed.

As in C1/C2, application rehearses the exact transaction on a private RAM copy,
requires a verified local safety backup, and commits live rows with the staging
cursor/checkpoint anchor. Local conflicts, unsupported kinds, bad mappings or
references refuse the entire page. No applied-device receipt is issued and no
hosted tombstone compaction is authorized.

## Verification

Seven new tests (20 safe-apply + 3 path tests total) cover:

- Windows escape/reserved-device forms and valid component distinctions.
- Nested typed mapping without creating folders; external/empty roots and file
  obstructions refused, with fictional sentinel content unchanged.
- Real isolated Windows junction (Unix symlink in CI) refused without touching
  the target; only explicitly created temporary fixtures are cleaned up.
- Task-before-project application, local rebasing, exact retry, verified backup,
  referenced-project tombstone refusal and safe same-page dependent deletion.
- Local project edits and remote path reassignment preserved on refusal.
- Missing selected root, reserved name, external reference and pre-existing-folder
  adoption refused while existing fictional document content remains unchanged.
- Case-insensitive mapped-path collision rolls back the whole page; valid project
  updates keep the original local path and do not create a folder.

Full Rust regression, frontend tests/lint/build and formatting checks are required
before push. CI states are checked on the actual review-branch runs. No screen
changed, so visual UI review is not applicable.

## Remaining work and limitations

RFI/submittal/file/relationship/activity/register/template adapters remain
unsupported. Missing project directories need an explicit local user workflow;
this slice must not auto-create them or auto-adopt existing directories. External
references need future explicit device-local mapping. The conservative path limit
and folding may refuse otherwise usable Windows names; no fallback guesses a path.
Future document operations must independently validate their targets/handles at
the operation boundary: metadata checks here are not a general filesystem race
guarantee. Callers must hold exclusive local DB access throughout preflight,
backup and application, as required by C1.

Durable record-scoped conflict resolution, live mutation/outbox integration,
partial-page staging, multi-revision pages, applied receipts, key rotation,
recovery anchors and broader two-device/security/consent rollout gates remain
pending. No production activation is authorized by this slice.

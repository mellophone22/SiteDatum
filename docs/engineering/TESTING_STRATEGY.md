# Testing Strategy

Unit: validation, Attention grouping, date/follow-up rules, status transitions, path sanitization, collision naming, relationship rules.

Integration: SQLite/migrations, project creation, file copy/move/register, recovery behavior, search, archive/restore.

UI: forms, tables, filters, inline edits, empty/error states, keyboard behavior.

E2E: create project -> folders -> task/RFI/submittal/drawing -> mark waiting -> Attention updates -> register file and reveal it in Explorer -> search -> archive.

Quality gates: typecheck, lint, tests, production build, no console errors, no placeholder controls, manual visual review with realistic data volume.

## WP2 verified coverage

Rust unit tests cover Waiting requirements, calendar-date validation (including leap-day and month bounds), mutually exclusive Attention categories, completed/cancelled exclusion, and the inclusive 14-day Upcoming boundary. SQLite integration coverage exercises task creation, Waiting entry/exit, atomic activity events, missing-task recovery errors, and persistence across database reopen. Frontend quality gates cover TypeScript build/typecheck, ESLint, and Vitest.

## WP3 verified coverage

Rust unit coverage validates RFI lifecycle gates: recipient before opening and response before marking received/closed. SQLite integration coverage creates an RFI linked to a same-project task, confirms it appears/disappears from the response-due Attention queue, persists it across reopen, and verifies attachment-reference add/remove activity without altering the original file.

## WP10 verified coverage

Rust coverage loads the embedded template, creates a readable one-page PDF with representative RFI fields, verifies key overlay text, refuses an existing destination, and persists the additional template fields across database reopen. Visual verification compares a rendered completed PDF with the supplied workbook's PDF rendering. Frontend build/typecheck, ESLint, Vitest, native launch, and production packaging remain required release gates.

## WP11 verified coverage

Frontend unit coverage verifies task-progress totals, completed percentages, status counts, cancelled-task exclusion, and the zero-task state. The Attention and Tasks views share the same semantic progress component, which uses a native `progress` element and textual counts so color is not the only indicator. Project-context filtering is applied consistently to task progress, task queues, RFIs, and submittals.

## WP12 verified coverage

Frontend unit coverage verifies saved task-view serialization and malformed-storage recovery. Manual interaction coverage verifies dialog focus containment, Escape/Cancel, required-field gating, record-type switching, named-view save/restore/delete, project-context inheritance, and Overview routing. Existing Rust domain and persistence tests verify that quick-captured records still pass the normal application boundary.

## Final MVP verification coverage

The persistence suite now covers a checkpointed backup copied and reopened as an independent SQLite database, including file/drawing metadata, current/superseded state, notes, and contacts. A realistic-volume integration test creates 500 tasks, verifies all corresponding activity events, and reopens the database to confirm no records were lost. Search focus has frontend tests for exact-record routing state, cross-screen preservation, and malformed-state recovery.

Release-safety coverage builds a representative version-9 workspace with project, task, RFI, submittal, relationship, attachment, registered-file, note, contact, sync-marker, and activity records plus an ordinary project file. It opens that workspace through the current version-10 application boundary and verifies the migrated database, the valid version-9 snapshot, foreign-key enforcement, record relationships, exact file-path preservation, unchanged file bytes, the version-10 table addition, and no duplicate snapshot on a current-schema reopen. A deliberately invalid migration verifies that partial schema work and its migration marker roll back while the snapshot remains intact. A compatibility test records a future schema version and verifies that an older executable refuses to open it instead of writing through an unsupported schema.

GitLab runs both Supabase pgTAP files on a disposable database inside an ephemeral hosted-runner VM. The CI job uses the pinned repository CLI, applies only committed migrations, has no hosted-project credentials, and destroys its local test volumes after completion. Persistent self-managed privileged runners are outside the accepted security boundary.

The operator-run C8 device allowance acceptance uses `supabase/tests/c8_device_allowance.sql` against the linked licensing sandbox. It creates a fictional Auth subject and subscription inside a transaction, asserts activation, same-device reuse, the two-device ceiling, third-device rejection, ownership-scoped deactivation, replacement activation, and entitlement availability, then rolls back. A separate read-only query verifies that no test Auth or licensing customer remains. The script never contacts Stripe, uses no live-mode resource, and stores no project or customer content.

The production gate builds both Windows bundle formats. Live UI automation was attempted during the final review but the host's trusted computer-control RPC service was not configured (`sky`); source-level keyboard/focus review and the Tauri development/runtime checks were completed, and the external automation limitation is recorded in `FINAL_VERIFICATION.md`.

## WP13-WP18 verified coverage

Rust domain tests cover operational type/status/priority constraints, daily-report and transmittal requirements, checklist bounds, database persistence, transactional bulk status changes, and transactional template application. Data-exchange tests cover CSV and native Excel generation, full-batch import preview, validation, and overwrite refusal. Report tests verify the visible template version and collision refusal. Recovery tests and release verification cover backup integrity preview, constrained restore selection, migration-forward behavior, and preservation of foreign-key enforcement.

Frontend release gates cover TypeScript typecheck/build, ESLint, Vitest, keyboard and focus inspection, reminder opt-in behavior, register filtering/editing/bulk selection, import preview, template application confirmation, report generation, recovery confirmation, and visual review of Operations, Calendar, Reports, Recovery, and Settings at supported window sizes.

## Complete portability coverage

Rust integration coverage creates archived user data and a Pro-created template, exports through the same accountless database boundary used by Free, and verifies that all 14 core entity CSVs plus `manifest.json` are finalized without a leftover partial directory. The test confirms row counts, archived-row inclusion, numeric preservation, and spreadsheet-formula neutralization. Frontend build, lint, and a Recovery-screen visual review cover the user-facing export action and its explanation that document bytes are not copied.

# Testing Strategy

Unit: validation, Attention grouping, date/follow-up rules, status transitions, path sanitization, collision naming, relationship rules.

Integration: SQLite/migrations, project creation, file copy/move/register, recovery behavior, search, archive/restore.

UI: forms, tables, filters, inline edits, empty/error states, keyboard behavior.

E2E: create project -> folders -> task/RFI/submittal/drawing -> mark waiting -> Attention updates -> register/open file -> search -> archive.

Quality gates: typecheck, lint, tests, production build, no console errors, no placeholder controls, manual visual review with realistic data volume.

## WP2 verified coverage

Rust unit tests cover Waiting requirements, calendar-date validation (including leap-day and month bounds), mutually exclusive Attention categories, completed/cancelled exclusion, and the inclusive 14-day Upcoming boundary. SQLite integration coverage exercises task creation, Waiting entry/exit, atomic activity events, missing-task recovery errors, and persistence across database reopen. Frontend quality gates cover TypeScript build/typecheck, ESLint, and Vitest.

## WP3 verified coverage

Rust unit coverage validates RFI lifecycle gates: recipient before opening and response before marking received/closed. SQLite integration coverage creates an RFI linked to a same-project task, confirms it appears/disappears from the response-due Attention queue, persists it across reopen, and verifies attachment-reference add/remove activity without altering the original file.

## Final MVP verification coverage

The persistence suite now covers a checkpointed backup copied and reopened as an independent SQLite database, including file/drawing metadata, current/superseded state, notes, and contacts. A realistic-volume integration test creates 500 tasks, verifies all corresponding activity events, and reopens the database to confirm no records were lost. Search focus has frontend tests for exact-record routing state, cross-screen preservation, and malformed-state recovery.

The production gate builds both Windows bundle formats. Live UI automation was attempted during the final review but the host's trusted computer-control RPC service was not configured (`sky`); source-level keyboard/focus review and the Tauri development/runtime checks were completed, and the external automation limitation is recorded in `FINAL_VERIFICATION.md`.

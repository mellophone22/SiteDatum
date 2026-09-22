# ADR-001 — WP0 Foundation Boundaries

**Status:** Accepted — 2026-09-21

## Context

The MVP is a local-first Windows desktop workspace. It must safely manage metadata and relationships while preserving project documents as normal Windows files. The database and filesystem cannot participate in one shared transaction.

## Decisions

1. **Rust owns persistence.** The frontend has no SQL capability. Typed Tauri commands call Rust application services and repositories.
2. **SQLite uses `rusqlite` with bundled SQLite.** This creates a reproducible Windows build without a separate SQLite installation. Foreign keys are enabled for every opened connection.
3. **Rust owns filesystem operations.** The React renderer cannot perform unrestricted filesystem work. It requests named, validated operations through Tauri commands.
4. **Application data lives separately from project documents.** SQLite, logs, state, and future backups use the OS application-local data directory, never the configurable project root.
5. **Migrations are repository-managed.** Ordered SQL migrations are applied transactionally and recorded in `schema_migrations`.
6. **Filesystem operations are recoverable workflows.** Future Copy, Move, Register, and folder-creation operations must plan, validate, execute, persist activity/metadata, and report any partial result with a recovery path. They must never claim success early.
7. **Windows path policy.** Accept existing absolute local paths and valid UNC roots. Canonicalize existing roots, reject relative paths and invalid Windows characters, require a readable directory, reject the selected root if it is a symlink/reparse point, compare paths case-insensitively in future path operations, and check child traversal before operations. Long paths receive an explicit warning and are tested during file work.
8. **Structured errors are stable at the boundary.** Command errors expose an error code, concise message, recovery guidance, and correlation ID. Technical details are retained only in local logs/stderr.
9. **Time and domain constraints.** Timestamps are stored as UTC ISO-8601 text. Future domain schemas will constrain statuses and priorities with check constraints rather than accepting arbitrary strings.

## Consequences

The first settings slice exercises the complete boundary: Rust validates and persists the Project Root setting; React can only invoke typed commands and display results. More application services are added one work package at a time.

## WP1 clarification — project folder identity

Editing a project's number or name changes metadata only; it never renames or moves the physical project folder. A future explicit rename/move workflow must preview the source and destination, prevent collisions, and recover from partial filesystem success. This prevents a routine metadata edit from silently moving user documents.

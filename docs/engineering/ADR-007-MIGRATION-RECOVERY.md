# ADR-007 — Migration recovery boundary

## Status

Accepted on 2026-10-02.

## Context

SiteDatum owns one local SQLite workspace and applies ordered repository migrations during desktop startup. Each migration was already transactional, but startup did not preserve a pre-upgrade snapshot. An application rollback could also open a database whose schema was created by a newer executable.

## Decision

1. Opening an existing database reads the highest recorded schema version before applying migrations.
2. When that version is older than the executable's current schema, SiteDatum creates and validates an app-local SQLite snapshot under `backups/` before executing migration SQL.
3. The snapshot name records the source and target schema versions: `pre-migration-v{from}-to-v{to}-{timestamp}.sqlite3`.
4. Backup creation uses SQLite's backup API so committed WAL content is included. The snapshot is written with an internal `.partial` suffix and receives its visible `.sqlite3` name only after `PRAGMA integrity_check` succeeds. Migration does not begin unless the validated snapshot is finalized.
5. Every migration remains an independent transaction. Failed SQL rolls back both its schema changes and its `schema_migrations` record.
6. An executable refuses to open a database with a schema version newer than it supports. The recovery message directs the user to reinstall the newer SiteDatum version without replacing or deleting the workspace.
7. Migration backups remain normal recovery inventory. SiteDatum never automatically deletes or overwrites them.

## Consequences

- A release that advances the schema creates a recoverable pre-upgrade snapshot before touching customer records.
- Reopening an already-current database does not create redundant backups.
- Rolling back only the executable is safe when the schema did not advance. After a schema advance, operators must reinstall the newer executable or explicitly restore the pre-migration backup using the runbook.
- A database from an unrelated or future SiteDatum build fails closed instead of being partially interpreted by an older binary.
- Backup-directory or integrity failures stop startup before migration; they do not weaken the safety boundary.


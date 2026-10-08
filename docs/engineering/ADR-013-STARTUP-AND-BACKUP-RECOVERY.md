# ADR-013 — Startup and backup recovery

Status: implemented 2026-10-08.

## Context

SiteDatum already created app-local snapshots before migrations and allowed a confirmed restore from its local backup inventory. Two trust gaps remained: a database-open failure could end startup without presenting an actionable recovery route, and routine backups could not be directed to a user-controlled second location or restored from an explicitly selected file.

## Decision

1. Database-open and migration failures no longer drop the user at an unexplained process exit. SiteDatum opens a temporary in-memory recovery session and renders a recovery-only screen with a stable error code, correlation reference, and the exact workspace and backup locations.
2. Startup recovery accepts only an existing `.sqlite3` file that passes SQLite integrity checking and contains the required SiteDatum schema markers. Recovery prepares and validates a migrated candidate before touching the unavailable database.
3. The unavailable database is moved to a uniquely named recovery archive before the candidate is activated. The archive is never overwritten or automatically deleted. Project folders and documents are not modified.
4. Every new backup uses SQLite's online backup API. It is written to a private `.partial` file, integrity-checked, checked for SiteDatum schema identity, and renamed to its visible `.sqlite3` name only after validation succeeds.
5. Recovery lists exact paths and timestamps for app-local and configured external backups. A user may also select and preview an arbitrary SiteDatum backup file before an explicitly confirmed restore.
6. A user may configure an existing external folder and choose Off, Daily, or Weekly. Scheduled backups are device-local and run only when SiteDatum is open. No Windows background task, cloud dependency, or account is introduced.
7. Restore and startup recovery remain Free and outside commercial enforcement.

## Consequences

- Startup failures become actionable without exposing project content to any service.
- Users can keep a second copy on another local folder, connected drive, or user-controlled synchronized folder.
- Scheduling is intentionally best-effort while the application runs; the UI states this limitation directly.
- Backup destination settings live in local `app_settings` and are not added to cloud snapshot payloads.
- A restored startup database takes effect after SiteDatum is closed and reopened because the current process is intentionally isolated on an in-memory recovery database.

## Verification

- Rust tests cover verified snapshot creation, schedule due logic, ordinary restore, and startup replacement with preservation of the unavailable database.
- Frontend build, lint, unit tests, and the route-level visual review cover the normal recovery workspace and the startup recovery screen.

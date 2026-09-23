# ADR-002 — Optional Supabase Metadata Sync and OneDrive Files

**Status:** Accepted — 2026-09-22

## Context

AnyDesk is local-first, but one user needs the same workspace on multiple Windows computers. Project documents must remain normal Windows files and the existing local SQLite database must not be placed in a synced folder.

## Decisions

1. **OneDrive owns project-file replication.** Each computer selects the same dedicated OneDrive-backed project root. AnyDesk neither scans nor mutates unrelated OneDrive content.
2. **Supabase owns optional metadata replication.** Local SQLite remains the application’s offline working store. Supabase holds only sync records for AnyDesk metadata; it never receives a SQLite database file.
3. **Sync is opt-in and does not make launch dependent on the network.** Local functionality continues when Supabase or OneDrive is unavailable.
4. **Sync records are user-owned.** `public.sync_records` has RLS enabled and every policy limits access to `auth.uid() = owner_id`.
5. **Concurrent edits are conflicts, never silent last-write-wins.** `public.sync_upsert_record` uses a caller-provided version and row lock. A stale version returns the current cloud record for a user-facing resolution workflow instead of overwriting it.
6. **Absolute local paths are device-local.** Future sync serialization must convert project-file references beneath the configured root to root-relative paths and resolve them on each device. App settings, access tokens, and raw local paths are not sync payloads.
7. **Authentication uses a manually provisioned, confirmed email/password user.** This avoids a custom SMTP dependency on the free Supabase tier. The password is never persisted by AnyDesk; session material is retained only in platform-protected credential storage. The Supabase publishable key is not secret and the service-role key is never shipped.

## Consequences

The first sync implementation must include sign-in, encrypted local session storage, project-root compatibility checks, a manual sync action with visible state, pull/push reconciliation, and a conflict-resolution screen. It must test two-device conflict behavior before automatic/background syncing is considered.

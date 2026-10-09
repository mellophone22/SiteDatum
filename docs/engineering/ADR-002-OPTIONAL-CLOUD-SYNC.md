# ADR-002 — Optional Supabase Metadata Sync and OneDrive Files

**Status:** Superseded by ADR-019 for Sync v2; retained for legacy compatibility — 2026-10-08

## Context

SiteDatum is local-first, but one user needs the same workspace on multiple Windows computers. Project documents must remain normal Windows files and the existing local SQLite database must not be placed in a synced folder.

## Decisions

1. **OneDrive owns project-file replication.** Each computer selects the same dedicated OneDrive-backed project root. SiteDatum neither scans nor mutates unrelated OneDrive content.
2. **Supabase owns optional metadata replication.** Local SQLite remains the application’s offline working store. Supabase holds only sync records for SiteDatum metadata; it never receives a SQLite database file.
3. **Sync is opt-in and does not make launch dependent on the network.** Local functionality continues when Supabase or OneDrive is unavailable.
4. **Sync records are user-owned.** `public.sync_records` has RLS enabled and every policy limits access to `auth.uid() = owner_id`.
5. **Concurrent edits are conflicts, never silent last-write-wins.** `public.sync_upsert_record` uses a caller-provided version and row lock. A stale version returns the current cloud record for a user-facing resolution workflow instead of overwriting it.
6. **Absolute local paths are device-local.** Future sync serialization must convert project-file references beneath the configured root to root-relative paths and resolve them on each device. App settings, access tokens, and raw local paths are not sync payloads.
7. **Authentication uses a manually provisioned, confirmed email/password user.** This avoids a custom SMTP dependency on the free Supabase tier. The password is never persisted by SiteDatum; session material is retained only in platform-protected credential storage. The Supabase publishable key is not secret and the service-role key is never shipped.

## Consequences

The first sync implementation must include sign-in, encrypted local session storage, project-root compatibility checks, a manual sync action with visible state, pull/push reconciliation, and a conflict-resolution screen. It must test two-device conflict behavior before automatic/background syncing is considered.

## Commercial-launch containment — 2026-10-08

Metadata synchronization is deferred pending a separate product, privacy, and hosted-security review. Fresh installations do not display Sync and the Rust command boundary rejects attempts to create a new legacy Sync connection. A computer is grandfathered only when it already has a saved legacy Sync credential or local evidence of prior synchronization. That eligibility is then preserved in the local workspace so an existing user can disconnect and reconnect without losing the recovery path. This containment does not delete local sync state, remove remote records, or decommission the legacy service.

The legacy path remains distinct from licensing. It is not included with Pro, is not advertised to new customers, and does not make ordinary local work dependent on a cloud service. Before Sync can return as a generally available capability, SiteDatum must revalidate the hosted project, RLS policies, grants, signup and recovery model, data minimization, retention, deletion, and user-facing consent.

ADR-019 defines the replacement architecture and exit criteria. It does not authorize reactivation of this legacy path.

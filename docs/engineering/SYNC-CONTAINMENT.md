# Sync Containment and Privacy Alignment

**Implemented:** 2026-10-08
**Scope:** initial commercial launch containment; no hosted-service mutation

## Outcome

The legacy Supabase metadata-sync implementation remains intact for recovery and compatibility, but it is no longer available to fresh installations. The desktop exposes Legacy Sync only when the computer already has a saved legacy Sync credential or the local workspace contains prior sync state. Once discovered, that eligibility is preserved in local `app_settings`, allowing an existing user to disconnect and reconnect later without opening enrollment to new users.

The same rule is enforced in Rust before sign-in, synchronization, conflict listing, conflict resolution, or disconnect operations. Hiding the React settings section is therefore not the security boundary. A fresh installation that attempts to invoke a legacy command directly receives `SYNC_DEFERRED`, and its local workspace remains unchanged.

## Data boundary

Ordinary Free use remains accountless and offline-capable. Licensing and billing remain separate from legacy Sync and never receive project content. For a grandfathered user who deliberately invokes Sync, the existing workspace service can receive the metadata tables enumerated in `cloud_sync.rs`; it does not receive document bytes, the SQLite database file, application settings, credentials, or absolute local paths. The public privacy notice now describes this exception and its limits.

## Hosted boundary

This package does not create users, inspect private hosted rows, change RLS, modify grants, deploy functions, delete remote snapshots, or decommission the legacy Supabase project. Repository evidence describes owner-scoped RLS and the `sync_upsert_record` function, but the currently deployed legacy project was not independently verified in this package. General availability remains blocked until the hosted schema, RLS and grants, authentication lifecycle, recovery, consent, retention, and deletion behavior pass a separate review.

## Reversal and recovery

Containment is non-destructive. Existing local records, sync baselines, conflicts, credentials, and project files are not removed. Disconnecting deletes the device session but retains local grandfathered eligibility. If the product later reintroduces Sync, it must use a newly reviewed entitlement and consent design rather than treating this compatibility path as the launch implementation.

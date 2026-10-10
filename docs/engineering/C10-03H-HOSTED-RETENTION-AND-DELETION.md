# C10-03H hosted retention and deletion

**Status:** Disposable local workspace lifecycle and deletion slice complete; production deployment and customer Sync remain disabled — 2026-10-09

## Scope

C10-03H implements the server-side portion of ADR-019's disabled-workspace and
hosted-deletion policy in the isolated Sync v2 proof. It does not add a desktop
command, change local SQLite, touch Windows project files, create a production
resource, or enable Sync.

## Implemented boundary

- `Disable Sync` changes only the opaque hosted workspace state and fixes its
  purge deadline at exactly 30 days.
- A signed request from an active device may restore the workspace before that
  deadline. Expired recovery windows fail closed.
- `Delete now` removes the workspace row and all cascading ciphertext,
  checkpoint, change, and idempotency rows immediately. Workspace-key transfer
  envelopes for that opaque workspace are removed explicitly.
- Account-level Sync-data deletion removes every hosted Sync workspace and
  pending enrollment/transfer row for the owner, then revokes all Sync devices
  and Sync sessions. It deliberately does not delete the Supabase Auth identity;
  the later account-deletion orchestration must perform that separate provider
  action after the Sync-data receipt is returned.
- Every destructive request is idempotent by `(owner_id, request_id)`. Reusing a
  request identifier for a different scope or workspace fails closed.
- Receipts contain no project content, filenames, paths, ciphertext, titles,
  notes, or record fields. The bridge returns only outcome, random receipt ID,
  completion time, and policy version. Private receipt rows expire after 90 days.
- A committed daily `pg_cron` job invokes the private retention sweep at 03:17
  UTC. The sweep deletes expired disabled workspaces, issues the same
  content-free receipt, and removes expired receipts.
- The `sync-lifecycle` Edge Function requires a valid user JWT, a live Auth
  session, a live Sync session/device, strict request shape, rate limit, and an
  Ed25519 signature over the owner, Auth session, device, action, and exact
  workspace/request identifiers. A bearer token alone is insufficient.

## Failure and recovery behavior

- Disabling, restoring, or deleting hosted data never operates on the desktop
  database or filesystem.
- Disabled workspaces are already refused by record push and pull operations.
- Exact deletion retries return the original receipt. Missing, cross-owner,
  revoked-device, altered-request, and invalid-signature calls expose no record
  content and do not partially delete.
- The retention sweep uses row locks with `skip locked`, so concurrent workers
  do not delete the same workspace twice.
- Provider backups are outside the active tables and age out according to the
  provider backup schedule; the receipt does not claim immediate backup erasure.

## Tombstone retention boundary

ADR-019 also requires tombstones to remain until all active devices acknowledge
them and at least 30 days pass, with a 45-day operational cap absent an incident
hold. C10-03H does **not** delete individual tombstones yet. Safe compaction needs
the C10-04 durable per-device cursor and authenticated replacement checkpoint;
deleting a tombstone before that proof exists could permit an offline computer
to resurrect a record or make the encrypted completeness checkpoint stale.
Until C10-04 supplies that proof, the backend retains tombstones rather than
performing an unsafe or unverifiable purge. This fail-closed deferral keeps
C10-03 open.

## Verification

- `retention_deletion.sql`: 35 pgTAP assertions cover forced RLS, grants,
  scheduled retention, owner isolation, exact 30/90-day clocks, disable/restore,
  immediate deletion, receipt replay, account-level revocation, and retention
  expiry.
- `sync-lifecycle/protocol.test.ts`: four unit tests cover exact shapes,
  canonical action separation, malformed input, and exact-message signature
  verification.
- `retention_deletion.integration.mjs`: disposable Auth/device enrollment,
  invalid-signature refusal, signed disable/restore, immediate deletion, exact
  receipt retry, account cleanup, and post-revocation refusal.

## Remaining C10-03 gates

1. C10-04-backed tombstone acknowledgement and authenticated compaction proof.
2. Clean repository reconstruction of the isolated backend.
3. Supabase security/performance advisor evidence.
4. Client bundle and repository-history secret scan evidence.

No customer beta is authorized until the later C10 gates pass.

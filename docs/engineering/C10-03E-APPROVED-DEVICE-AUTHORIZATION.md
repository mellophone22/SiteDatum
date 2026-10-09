# C10-03E approved-device authorization

**Status:** Durable service-only database boundary verified in the disposable local stack; trusted bridge, native integration, and production deployment remain unauthorized — 2026-10-09

## Purpose

Establish the durable authorization state required to add one approved second
Windows computer without weakening SiteDatum's local-first boundary. An
already active device must approve the exact target device after that target
proves possession of its own Ed25519 identity. The database also records a
separate X25519 public key for the later authenticated workspace-key transfer.

This slice does not transfer a workspace key, add a public RPC or Edge
endpoint, register a Tauri command, add UI, enable Sync, or change a production
Supabase project. The centralized `SYNC_DEFERRED` denial remains unchanged.

## Durable state machine

1. An active device registers one 32-byte X25519 HPKE Auth public key after a
   trusted bridge verifies possession of its existing Ed25519 key.
2. A live, confirmed target Auth session begins an enrollment for one exact
   source device and one exact target device identity.
3. The server creates a random 32-byte challenge with a ten-minute lifetime.
4. The target proves possession of its Ed25519 private key. Invalid or expired
   attempts consume the enrollment.
5. The exact source device approves the already verified target through its
   own live and unrevoked device-bound session.
6. One transaction creates the second device and binds the target Auth session
   to it. The active-device maximum remains two.

Only an exact accepted-response retry can return `already_accepted`. Altered,
expired, consumed, cross-owner, session-colliding, device-colliding, revoked,
or third-device attempts fail closed. Target and source proof attempts are
single-use and durably timestamped. Pending enrollment uniqueness and an
owner/session rate window constrain concurrent or repeated requests.

## Privilege and privacy boundary

- The enrollment table is private, has forced RLS, and grants no direct table
  access to `anon`, `authenticated`, or `service_role`.
- Only four narrow service-role functions are executable. The session helper
  remains private even from `service_role`.
- Every security-definer function uses an empty search path and fully qualified
  object references.
- Live Supabase Auth sessions are correlated with the internal device-bound
  session before source approval or transfer-key registration succeeds.
- Persisted authorization rows contain identifiers, public keys, challenges,
  timestamps, counters, and content-free outcomes only. They contain no
  project names, tasks, RFIs, submittals, contacts, notes, file paths,
  documents, database content, passwords, private keys, or recovery secrets.

## Verification evidence

- The approved-device pgTAP suite passes 46 assertions covering schema and
  function presence, direct-access denial, malformed keys, possession proof,
  exact-retry behavior, wrong-owner refusal, expiration, consumption,
  third-device refusal, atomic session binding, and durable attempt counts.
- The complete disposable database suite passes 160 assertions across the
  C10-03A through C10-03E boundaries.
- The migration applies in the disposable local project and does not alter any
  production resource.

## Remaining gates

C10-03F now supplies the trusted authenticated bridge, Windows-native X25519
key boundary, independent Ed25519 verification, and authenticated HPKE
workspace-key transfer for this state. Later C10-03 work still owns encrypted
hosted record operations, retention and deletion, clean disposable
reconstruction, advisor evidence, and repository/client secret proofs. C10-04
through C10-07 remain required before any customer Sync beta.

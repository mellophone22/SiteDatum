# C10-03D native device enrollment

**Status:** Windows-protected native enrollment verified against the disposable local bridge; runtime exposure and production deployment not authorized — 2026-10-09

## Purpose

Connect SiteDatum's Windows-native process to the C10-03C trusted enrollment
bridge without exposing a service credential, weakening the local-first model,
or enabling Sync. This slice generates one Ed25519 identity per Windows user
profile, protects its private seed with Windows Credential Manager, and proves
possession through the disposable local Edge Function.

The module is not registered as a Tauri command and has no UI control. The
central `SYNC_DEFERRED` denial remains unchanged, so ordinary releases cannot
invoke this work through the application.

## Local identity boundary

The credential uses the dedicated service
`com.sitedatum.sync-v2.device-identity` and a versioned account name. Its JSON
record contains only:

- schema and key version;
- a random device UUID;
- the 32-byte Ed25519 private seed; and
- a minimal enrollment state.

The record never contains project names, tasks, RFIs, submittals, contacts,
notes, file paths, documents, database content, passwords, service-role keys,
or recovery secrets. Key generation uses the operating-system CSPRNG. Decoded
seed buffers and serialized credential strings use zeroizing wrappers where
the Rust libraries permit it. This reduces secret lifetime but does not claim
perfect erasure of allocator, operating-system, or crash-dump copies.

## Crash-safe enrollment state

The local record moves through three explicit states:

1. `pending_new`: the key and device UUID are saved before the first request;
2. `pending_proof`: the server-derived owner, Auth session, one-time
   enrollment, challenge, and expiry are saved before completion; and
3. `active`: written only after the bridge returns the same device UUID.

A network failure after completion leaves `pending_proof` intact so the exact
signature can be retried. C10-03D narrows the server's idempotent path to an
enrollment already recorded as `accepted`, with the same owner, live Auth
session, device UUID, proof algorithm, and public key. The Edge Function still
rebuilds and verifies the original Ed25519 proof. A false or altered proof,
different identity, revoked Auth session, or non-accepted enrollment cannot
use this path.

An explicit server refusal resets only the pending enrollment context and
retains the same protected device key. A timeout or unavailable response keeps
the pending proof for safe retry. An active local identity short-circuits
without a network request; this does not itself authorize Sync, because later
operations must still pass server-side session/device checks.

## Network boundary

- The client accepts HTTPS only. Loopback HTTP is permitted solely in debug
  builds for the disposable proof.
- The caller supplies a public client identifier and short-lived user access
  token; no private server credential exists in the module.
- Connect and total request timeouts are bounded.
- Redirects are refused so credentials and enrollment bodies stay on the
  configured trusted origin.
- Successful response bodies are limited to 8 KiB and parsed with strict
  unknown-field rejection.
- Error handling records only content-free status and transport diagnostics;
  response bodies, access tokens, keys, signatures, and credential JSON are
  not logged.

## Verification evidence

- Five focused native tests pass canonical compatibility, fresh enrollment,
  active reopen, response-loss retry, explicit-refusal recovery, strict HTTPS
  configuration, and exact Ed25519 verification. The two live mutation tests
  remain ignored in ordinary suites.
- The production keyring implementation separately passed a create, reopen,
  compare-public-key, and delete drill using one uniquely named fictional
  Windows credential.
- The native client passed an end-to-end drill against the disposable local
  Supabase stack: create a fictional confirmed Auth user, sign in, create and
  protect a device key, begin enrollment, sign the server context, complete
  enrollment, reopen the active identity without another request, and delete
  both fictional user and credential.
- The C10-03D checkpoint database suite passed 114 pgTAP assertions,
  including exact accepted-completion retry, invalid-proof refusal,
  wrong-owner refusal, revoked-session refusal, privileges, RLS, and rate
  limits.
- The JavaScript Edge integration harness passes the same current response
  contract and exact-device idempotent retry.

## Remaining gates

C10-03D does not deploy a hosted function, add production coordinates, expose
a desktop command, or enable customer Sync. C10-03E now establishes the
durable service-only approved-second-device authorization state. A trusted
bridge, native integration, and authenticated workspace-key transfer for that
state are still required, as are the remaining encrypted hosted operations,
retention and deletion workflow, clean disposable reconstruction, advisor
evidence, and proof that no privileged credential enters the client or
repository. Later C10 packages still own local record synchronization, consent
UX, recovery, operations, and fictional two-profile beta evidence.

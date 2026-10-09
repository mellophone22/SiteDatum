# C10-03C trusted enrollment bridge

**Status:** Disposable local bridge verified; production deployment and native integration not authorized — 2026-10-09

## Purpose

Connect a validated Supabase Auth session to C10-03B's first-device proof of
possession without exposing a service credential to the desktop. This slice is
repository-owned and reproducible, but remains confined to the disposable
local Supabase project. It does not enable Sync, alter the SiteDatum desktop,
or create a production resource.

## Request boundary

`sync-device-enrollment` accepts only authenticated `POST` requests and uses
Supabase's user-authenticated Edge Function wrapper. The function derives the
owner and Auth session UUIDs from verified JWT claims; neither value is
accepted from request JSON. Before any enrollment work, a service-only RPC
also verifies that the exact owner/session pair still exists in
`auth.sessions`, so signing out or revoking the session immediately closes the
bridge even if a previously issued token has not yet expired.

The public request shapes are deliberately small and exact:

- begin: `action`, device UUID, and a 32-byte base64url Ed25519 public key;
- complete: `action`, enrollment UUID, and a 64-byte base64url signature.

Unknown fields, malformed identifiers, malformed key material, non-POST
methods, and declared bodies over 4 KiB are refused. Responses contain only a
content-free request identifier and the minimum enrollment result. Request
bodies, signatures, keys, user identifiers, and session identifiers are not
logged by application code.

## Trust separation

- The Edge Function uses the platform-provided service credential only inside
  the Edge runtime.
- Four public-schema RPC wrappers are executable only by `service_role`.
- `anon` and `authenticated` cannot execute those wrappers or read/write the
  private rate-window table.
- Each wrapper has a fixed empty `search_path` and revalidates the live Auth
  session before entering the private enrollment functions.
- The desktop receives no secret, service-role key, webhook secret, signing
  key, or database credential.
- The bridge carries no project name, record, document, path, contact, note,
  task, RFI, submittal, or database content.

## Proof and abuse controls

The function rebuilds the exact versioned, length-prefixed C10-03B message and
verifies the device's Ed25519 signature with WebCrypto before it can request
atomic completion. A consumed or invalid enrollment returns a generic refusal;
proof material cannot be replayed. A different public key, device, owner,
session, enrollment, challenge, or expiry changes the signed message.

Durable five-minute PostgreSQL rate windows are keyed by owner, live Auth
session, and action. Begin is limited to 10 requests and complete to 30. An
advisory transaction lock serializes a key's counter update, and stale windows
are removed opportunistically. C10-03B's separate five-per-hour begin limit
continues to provide a second durable control.

## Verified evidence

- The cross-language canonical-message digest matches in TypeScript and Rust.
- Focused protocol tests pass strict parsing, the shared vector, valid
  Ed25519 proof, and changed-context refusal.
- The focused Rust proof suite passes all five tests.
- The complete database suite passes 106 pgTAP assertions across identity,
  RLS, native authorization, enrollment, bridge privileges, session
  revocation, replay refusal, and rate limiting.
- Database lint reports no warnings for `public` or `sync_v2_private`.
- Local and repository migration histories match through C10-03C.
- A disposable end-to-end test creates a fictional confirmed Auth user, signs
  in, enrolls one device through the Edge Function, proves possession with a
  generated Ed25519 key, rejects proof replay, refuses an unapproved second
  device, refuses a missing bearer token, and deletes the fixture user.

## Remaining gates

C10-03C does not authorize production deployment. Before first customer use,
SiteDatum still needs the native Windows-protected key integration, the
approved-device path for a second computer, the remaining hosted Sync schema
and operations, retention/deletion workflow, a clean disposable rebuild, and
the later C10 verification gates. The legacy Sync boundary and centralized
`SYNC_DEFERRED` denial remain unchanged.

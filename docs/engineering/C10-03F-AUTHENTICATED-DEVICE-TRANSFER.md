# C10-03F authenticated approved-device transfer

**Status:** Trusted bridge, private encrypted relay, and non-command Windows-native proof verified in the disposable local stack; Sync and production deployment remain unauthorized — 2026-10-09

## Purpose

Complete the approved-second-computer path without giving SiteDatum's service
the workspace key. The target computer proves its Ed25519 identity, the active
source computer independently signs the exact transfer, and RFC 9180 HPKE Auth
mode encrypts the 32-byte workspace key directly from the source X25519 key to
the target X25519 key.

This slice does not register a Tauri command, add UI, enable Sync, deploy an
Edge Function or migration to production, or upload project records. The
central `SYNC_DEFERRED` denial remains unchanged.

## Protocol

1. The already active source computer generates and protects an X25519 private
   key in Windows Credential Manager. It signs a versioned registration
   message with its existing Ed25519 identity before the trusted bridge stores
   the public key.
2. A separate live target Auth session begins an approval for one exact source
   device, target device, Ed25519 public key, and X25519 public key.
3. The target signs a canonical server challenge binding the owner, target
   Auth session, source and target devices, enrollment, both target public
   keys, challenge, and expiry. The bridge independently verifies the
   signature before marking the target ready.
4. The source obtains that exact verified context. It encrypts the workspace
   key using RFC 9180 HPKE Auth mode with X25519/HKDF-SHA-256,
   HKDF-SHA-256, and ChaCha20-Poly1305.
5. The source signs a second canonical message binding both Auth sessions,
   both devices, the enrollment, public keys, challenge, random workspace ID,
   workspace-key version, expiry, HPKE encapsulated key, and ciphertext.
6. One database transaction enrolls the target and stores the bounded opaque
   HPKE envelope. Only the newly bound target session/device can fetch it.
7. The target verifies the authenticated HPKE sender, checks the six-digit
   exporter-derived comparison code locally, and recovers the 32-byte key in a
   zeroizing buffer.

The relay never receives the comparison secret, workspace key, either private
key, project names, tasks, RFIs, submittals, notes, contacts, file paths,
documents, or database content.

## Failure and retry behavior

- Target proof and source approval are independently Ed25519-verified.
- A wrong target proof or source approval consumes that approval attempt.
- Expired, revoked, wrong-session, wrong-device, cross-owner, third-device,
  altered-envelope, and altered accepted-response attempts fail closed.
- Only an exact accepted source-response retry returns `already_accepted`.
- Target fetch is idempotent during the ten-minute enrollment window and
  records only a content-free first-fetch timestamp.
- HTTPS is mandatory; debug-only loopback HTTP supports the disposable proof.
  Redirects are refused, requests and responses are bounded, and secret or
  response bodies are not logged.

## Privilege boundary

- Transfer and rate-window tables are private, force RLS, and grant no direct
  access to `anon`, `authenticated`, or `service_role`.
- Public-schema bridge functions are executable only by `service_role` so the
  authenticated Edge Function can reach them. Desktop clients receive only a
  publishable identifier and their short-lived user token.
- Every security-definer function has an empty search path and revalidates the
  live Supabase Auth session and exact device-bound internal session where
  required.
- No service-role key, secret API key, private signing key, or webhook secret
  enters the desktop module or repository.

## Verification evidence

- Three Edge protocol tests pass strict request parsing, stable canonical
  vectors, and exact Ed25519 verification.
- Five native tests pass protected X25519 key reopen/device binding,
  authenticated HPKE recovery, comparison-code refusal, cross-language
  canonical vectors, envelope signature binding, and trusted-origin checks.
- The C10-03F pgTAP suite passes 41 assertions covering forced RLS, grants,
  live sessions, exact device contexts, registration, target verification,
  atomic acceptance, exact retries, opaque bounded storage, and target-only
  fetch.
- The complete disposable database suite passes 201 assertions across
  C10-03A through C10-03F.
- A disposable two-session integration drill passes first-device enrollment,
  transfer-key registration, target challenge/proof, source preparation and
  approval, exact retry, authenticated fetch, and unauthenticated refusal.

## Remaining gates

C10-03 still requires the remaining encrypted hosted record operations,
retention/deletion workflow, clean reconstruction/advisor evidence, and
repository/client secret proof before it can close. C10-04 owns the local
record protocol and persistent workspace/checkpoint integration. C10-05 owns
the deliberate comparison/consent/device UX; no visible control is added by
this proof. C10-06 and C10-07 remain required before any customer Sync beta.

# C10-03B initial-device enrollment and proof of possession

**Status:** Local durable boundary verified; trusted bridge completed in C10-03C; native integration pending — 2026-10-09

## Purpose

Replace C10-03A's synthetic session/device insertion with a durable,
single-attempt first-device enrollment boundary. This slice remains isolated
from the application and does not enable Sync, expose a control, deploy a
hosted function, or change a production Supabase project.

## Decision

The first Sync device uses a dedicated Ed25519 signing key generated on the
Windows computer. The private key remains local and will be stored through the
approved Windows-protected key boundary in a later native-integration slice.
The server stores only the 32-byte public key.

The enrollment proof signs a canonical, versioned context containing:

- authenticated owner UUID;
- Supabase Auth session UUID;
- requested device UUID;
- one-time enrollment UUID;
- Ed25519 public key;
- 32-byte server-generated challenge; and
- server-issued expiry time.

The trusted service verifier must validate the user token and session, rebuild
that exact message, and verify the Ed25519 signature before calling the private
completion function. A boolean proof result is accepted only from
`service_role`; `anon` and `authenticated` cannot call either enrollment
function or read the private table. No service credential belongs in the
desktop or repository.

## Durable controls

- Challenges are generated in PostgreSQL using `pgcrypto`.
- Enrollment expires after five minutes using server time.
- Every completion attempt consumes the row, including invalid proofs,
  mismatched identity/session context, and expiry.
- Replays receive only the generic `not_accepted` result.
- One owner/session may begin at most five enrollments per hour.
- An owner with an active device cannot bootstrap another device; the
  approved-device transfer protocol is required instead.
- Owner-level advisory locks and row locks serialize competing starts and
  completions.
- Accepted completion atomically creates the device, binds the authenticated
  session, and makes the device eligible for C10-03A's server-derived JWT claim.
- Outcomes are content-free and contain no project data, paths, documents,
  secrets, comparison codes, or signatures.

## Current boundary

This slice intentionally did not create the trusted network bridge. C10-03C
now implements and locally verifies that separate boundary, including:

1. independently verify the Supabase JWT and required issuer/audience/session;
2. generate or retrieve the server enrollment context without exposing a
   service credential;
3. verify the Ed25519 signature over the exact versioned message;
4. pass only the verified subject/session and boolean result to the private
   functions; and
5. apply request-level rate limits in addition to the durable database limit.

The bridge evidence is recorded in
`C10-03C-TRUSTED-ENROLLMENT-BRIDGE.md`. Additional devices remain blocked until
the approved-device HPKE transfer is
connected to a separate durable enrollment mode. Recovery, revocation-triggered
rotation, and proof on ordinary Sync requests remain later C10 responsibilities.

## Verification evidence

- Four focused Rust proof tests pass for valid proof, wrong key, malformed
  signature, context replay, and public-key substitution.
- Thirty pgTAP assertions pass for privileges, challenge/expiry, single-use
  consumption, replay, owner/session binding, active-device refusal, malformed
  key input, durable rate limiting, atomic device/session creation, and token
  hook integration.
- The complete disposable database suite passes: 82 assertions across the
  identity/RLS, native authorization, and enrollment proofs.
- The complete Rust suite passes with 94 tests and one explicitly ignored
  Windows Credential Manager mutation test; the ignored test is not needed for
  this database-only slice.
- The frontend suite passes with 99 tests; formatting, lint, and the production
  frontend build pass.
- Supabase database lint reports no warnings for `sync_v2_private`.

These results verify the repository-owned local boundary only. They do not
authorize deployment, enable Sync, or substitute for the still-required Windows
key integration, hosted-project hardening, and later C10 gates.

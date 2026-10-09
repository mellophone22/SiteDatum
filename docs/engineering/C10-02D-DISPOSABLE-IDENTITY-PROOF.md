# C10-02D disposable identity proof

**Status:** Complete as an isolated engineering proof — 2026-10-08

## Boundary

This proof used only fictional identifiers and a disposable local Supabase stack. It did not create, reactivate, branch, or modify a hosted Supabase project. It did not use customer data, send email, change billing, enable Sync, or alter the production application command graph.

The proof has two independent parts:

1. `src-tauri/tests/sync_v2_identity_proof.rs` models a public native client completing an authorization-code flow with PKCE and receiving a short-lived native Sync session. The test signs tokens asymmetrically, then verifies the signature, key identifier, issuer, audience, subject, nonce, issued-at time, expiry, session identifier, and device identifier.
2. `proofs/sync-v2-identity/supabase/` is a reproducible disposable PostgreSQL/RLS boundary. Its committed migration grants an authenticated subject access only when the JWT subject owns the row and both the claimed session and device remain active.

The Rust proof is provider-neutral and intentionally does not add an OAuth client, consent page, signing key, callback listener, or login control to the shipping application. A production provider configuration and customer-facing consent flow remain later gates.

## Current provider constraints recorded

- Supabase's OAuth 2.1 server supports authorization code with PKCE for public clients. The authorization code is short-lived, single-use, and bound to the verifier and redirect URI.
- `state` is required by the SiteDatum design even when a provider labels it recommended. `nonce` is required whenever an ID token is requested.
- SiteDatum must accept any successful `2xx` token response. It must not depend on the older `201` response because Supabase changed the token endpoint to `200` on 2026-06-01.
- A production OIDC configuration requires an asymmetric provider signing key. No signing private key is committed by this proof.
- OAuth scopes describe OIDC data; they do not authorize database rows. Owner and active-session/device checks remain RLS responsibilities.
- Deleting an authentication user is not treated as token revocation. The future delete workflow must first revoke sessions/devices and the RLS authorization state must deny the next request.

References:

- <https://supabase.com/docs/guides/auth/oauth-server/oauth-flows>
- <https://supabase.com/docs/guides/auth/oauth-server/token-security>
- <https://supabase.com/docs/guides/database/postgres/row-level-security>
- <https://supabase.com/changelog/45468-breaking-change-oauth-token-endpoint-will-return-http-200-instead-of-201>

## Evidence

### Native identity protocol

Command:

```text
cargo test --test sync_v2_identity_proof
```

Result: 5 passed, 0 failed.

The focused tests prove:

- successful authorization-code plus S256 PKCE exchange;
- exact redirect binding and state validation;
- issuer, audience, non-empty subject, nonce, signature, key identifier, issued-at, and expiry validation;
- wrong verifier, wrong redirect, expired code, replayed code, mismatched state, bad signature, unknown key, and wrong claims fail closed;
- the verified immutable subject becomes the native owner identity; mutable user metadata is not consulted;
- logout, session revocation, device revocation, and account deletion deny the next native authorization check in under the one-second proof threshold; and
- a stale verifier rejects a token signed by a new key, a refreshed overlapping key set accepts both generations, and retirement removes the old generation on the next key-set refresh.

The one-second threshold is a harness safety bound, not a production service-level claim. C10-03 must define the JWKS refresh/overlap schedule and C10-05 must connect logout/deletion UX to authoritative revocation.

### Owner-scoped RLS

Environment:

- Supabase CLI 2.120.0;
- disposable local project ID `sitedatum-sync-v2-identity-proof`;
- local PostgreSQL 17.11 image;
- local GoTrue 2.197.0; and
- local PostgREST 16.4.

Result: 20 pgTAP assertions passed, 0 failed, followed by transaction rollback and `supabase stop --no-backup`.

The database proof verifies:

- RLS is enabled and forced on the exposed proof table;
- anonymous callers cannot use the schema or read/insert rows;
- authenticated callers cannot use the private session/device schema;
- an owner can select, insert, update, and delete only its own encrypted envelopes;
- `UPDATE` uses both `USING` and `WITH CHECK`, preventing ownership transfer;
- cross-owner reads, inserts, updates, and deletes fail;
- session revocation and device revocation remove access on the next SQL statement;
- deleting the subject's authorization state removes access on the next SQL statement; and
- negative tests do not modify the other fictional owner's row.

## Cost and cleanup

The connected organization was observed on the Free plan, but the connector could not return a cost quote for a hosted project or branch. Therefore no hosted resource was created or reactivated. The proof used free local tooling only. All disposable containers and volumes were removed without backup after the successful run.

## Production implications

- Continue with the existing SiteDatum customer identity direction; the separate Sync-account design remains the fallback if the real provider cannot meet the same mapping, revocation, deletion, or cost gates.
- The desktop remains a public client and must never contain a client secret, signing private key, service-role key, or webhook secret.
- A native session is authorized by immutable subject plus active session plus active device. Entitlement can enable the capability but never substitutes for row ownership.
- C10-02E independent review remains required before C10-03. This proof does not enable Sync or authorize a hosted schema.

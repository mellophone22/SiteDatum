# C10-03A native identity and authorization foundation

**Status:** Local disposable slice complete; C10-03 remains open — 2026-10-09

## Scope

C10-03A implements the first vertical slice authorized by the conditional
C10-02E approval. It remains an isolated local proof and does not enable Sync,
create a hosted project, add customer records, or change billing.

This slice addresses:

- fail-closed UUID parsing for `sub`, `session_id`, and `sync_device_id`;
- a private Supabase Custom Access Token Hook that derives the device claim
  only from a server-owned session binding;
- removal of any incoming or injected `sync_device_id` before derivation;
- session-to-device ownership binding with idle and absolute expiry;
- issuer/audience and UUID-subject validation in the native identity proof;
- malformed-JSON, malformed-subject, claim-mixing, device-borrowing, expired
  session, and hook-privilege negative tests; and
- Supabase's documented 10-second refresh-token reuse interval.

The disposable Auth configuration keeps global self-registration disabled.
The email provider is enabled only so an administrator-created fictional user
can complete the native sign-in proof; it does not open public sign-up.

The design follows current Supabase guidance: access tokens already contain a
UUID `session_id`; a Custom Access Token Hook may add custom claims; only
`supabase_auth_admin` may execute the hook; the hook is not exposed to `anon` or
`authenticated`; and refresh-token rotation retains the default 10-second
reuse interval for unreliable client/network recovery.

References:

- <https://supabase.com/docs/guides/auth/auth-hooks/custom-access-token-hook>
- <https://supabase.com/docs/guides/auth/auth-hooks>
- <https://supabase.com/docs/guides/auth/sessions>
- <https://supabase.com/docs/guides/local-development/cli/config>

## Security boundary

The token hook is a narrowly scoped `SECURITY DEFINER` function in the private,
unexposed schema. It has a fixed empty `search_path`, performs only a bounded
read of private session/device state, strips caller-provided device identity,
and returns no project content. Direct table privileges remain revoked. This is
an explicit exception to the general preference for invoker functions because
the Auth runtime must read private binding state without granting it table
access.

RLS authorizes an envelope only when all three UUID claims map to one active
owner/session/device tuple and neither session lifetime has expired. Missing,
malformed, mixed-owner, revoked, or expired state returns no rows.

## Remaining C10-03 work

This slice does not close C10-03. Later C10-03 slices still own:

- device bootstrap and proof-of-possession;
- the complete hosted envelope, quotas, monotonic versions, and idempotency;
- deletion cascade, retention, and content-free receipts;
- durable enrollment consumption, rate limiting, and server time;
- ephemeral loopback callback implementation, CSPRNG PKCE/state/nonce proof,
  bounded JWKS refresh, signing-key identifiers, and full session lifecycle;
- the Windows Credential Manager CI drill; and
- a disposable hosted reconstruction plus advisors, if it can be completed
  without unapproved cost.

No retired `cloud_auth.rs` token lifecycle may be reused.

The next bounded slice is documented in
`C10-03B-INITIAL-DEVICE-ENROLLMENT.md`.

## Verification

- Native Rust identity proof: 5 passed, including non-UUID subject rejection.
- Existing owner-scoped RLS proof: 31 passed.
- C10-03A token-hook/session/RLS proof: 21 passed.
- Local Supabase security advisor: no warning/error issues.
- Local Supabase performance advisor: no warning/error issues.
- Local migration history: both migrations present and applied in order.
- End-to-end local Auth hook proof: public self-registration was rejected; an
  administrator-created fictional user could sign in; its initial unbound
  token contained no device claim; after a server-owned session/device binding
  was created, token refresh preserved the subject and session identifiers and
  added exactly the server-derived device claim.
- Complete locked Rust suite: 90 passed, 1 deliberately ignored live Windows
  Credential Manager drill.
- Frontend Vitest: 99 passed across 31 files.
- ESLint and TypeScript/Vite production build: pass.
- Secret scan across the changed proof/doc/test scope: no local API, database,
  service-role, or JWT-secret values retained.
- Disposable local Supabase containers and volumes removed with
  `supabase stop --no-backup` after verification.

The malformed-subject issue from the independent re-review no longer
reproduces: non-UUID, empty, numeric, object-valued, missing, and malformed-JSON
claims all produce a clean zero-row denial. The legitimate control remains
intact: a valid owner/session/device binding receives the server-derived device
claim and sees only its owned row.

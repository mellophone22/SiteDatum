# C3 Licensing Service Foundation

Status: Completed locally 2026-09-30; not deployed

## Architecture decision

SiteDatum licensing uses a dedicated Supabase project that is separate from the optional metadata-sync project. Supabase Auth supplies paid-customer identity, Postgres stores the provider-neutral subscription projection and device activations, and Edge Functions issue signed entitlements. This service is not part of the local project workspace and never receives project names, records, documents, contacts, file paths, database contents, or sync payloads.

The C3 package was originally completed without a hosted project, production credential, merchant adapter, customer account, or secret. Hosted Edge Functions use Supabase's injected server-only service-role credential; it is never placed in the repository or desktop client. Provider webhooks and reconciliation remain C4 scope. Desktop sign-in, secure credential storage, entitlement caching, and subscription UI remain C5 scope.

## Local service layout

- `supabase/config.toml` defines the isolated local project, confirmed email identity, authentication rate limits, and function authentication requirements. Unused Realtime, Storage, vector, and analytics services are disabled.
- `supabase/migrations/20260930035439_create_licensing_foundation.sql` creates the private licensing schema and service-role-only RPC boundary.
- `supabase/functions/licensing-entitlement` validates the caller's Supabase user token, rate-limits issuance, activates or reuses a device, reads the projected subscription, signs a bounded entitlement, and records an audit event.
- `supabase/functions/licensing-health` reports only `ok` or `unavailable` plus the schema version.
- `supabase/functions/_shared/entitlement_contract.ts` defines the stable minimal claim set and the `sd1` Ed25519 token format.
- `supabase/tests/licensing_foundation.sql` verifies schema presence, RLS enablement, and denial of direct `anon` and `authenticated` access.
- `src/licensingContract.test.ts` provides host-side contract, minimization, signing, verification, and tamper tests.

The Supabase CLI is pinned as a development-only dependency so migration and local-test behavior is reproducible.

## Stored data

The licensing database stores only:

- the Supabase Auth user UUID;
- provider-neutral plan, status, paid-through time, source identifier, and external subscription reference;
- a locally produced SHA-256 device fingerprint hash, device UUID, and activation timestamps;
- hashed rate-limit keys and counters; and
- allowlisted audit event type, outcome, reason code, request UUID, customer/device UUIDs, and timestamp.

It deliberately does not duplicate email in licensing tables and has no free-form audit payload column.

## Authorization and isolation

All licensing tables live in a non-exposed `licensing` schema. Schema/table/sequence access is revoked from `public`, `anon`, and `authenticated`; RLS is also enabled on every table as defense in depth. Public-schema RPCs are `SECURITY DEFINER`, use an empty search path, revoke default execution, and grant execution only to `service_role`.

The entitlement Edge Function keeps platform JWT verification enabled and also validates the bearer token through Supabase Auth. This second check prevents a publishable API key from being mistaken for a user identity. The database secret key and Ed25519 private signing key exist only as Edge Function secrets. They must never be placed in the desktop, Git, project data, or a client-visible environment variable.

## Signed entitlement contract

The `sd1` token signs `sd1.<base64url claims>` with Ed25519 and appends the base64url signature. Its exact claims are:

- schema version and issuer;
- signing-key identifier;
- opaque Auth subject UUID;
- opaque device UUID;
- `pro_monthly` or `pro_annual`;
- `active`, `past_due`, or `canceled`;
- issued, refresh-after, and paid-through Unix timestamps.

The refresh horizon is seven days or the paid-through time, whichever comes first. Paid-through remains the hard entitlement boundary. No email, merchant payload, project content, path, or arbitrary metadata is signed.

Production key generation, offline backup, rotation, revocation, and public-key embedding require a separate founder-owned ceremony before C5 connects the desktop. The private key must be generated outside the repository and stored only in Supabase Edge Function secrets plus an independently protected recovery backup.

## Device and downgrade behavior

Device activation is serialized by locking the customer row. Reusing the same active fingerprint updates its last-seen time without consuming another slot. A third active device is rejected. Deactivation is ownership-scoped. Active, past-due, and canceled subscriptions remain eligible only before their trusted paid-through timestamp. An authoritative expired state may be signed only for an already-active matching device while its recorded paid-through timestamp is still in the future; the desktop interprets that claim as Free immediately. Expired or missing subscriptions can never activate a new device or receive Pro access.

These service decisions do not delete, hide, move, or modify local workspace data. C1 and C2 remain authoritative for non-destructive desktop behavior.

## Verification

Completed locally:

- Ed25519 generation/sign/verify and tamper rejection: pass;
- minimal-claim and request-validation tests: pass;
- clean database rebuild from the repository migration: pass;
- 29 pgTAP schema, RLS, privilege, subscription, device, rate-limit, and audit assertions: pass;
- Supabase security/performance advisors at warning level: no issues;
- public health endpoint with server-only local secret: pass;
- unauthenticated entitlement request rejection: pass;
- authenticated local customer/subscription/device entitlement issuance: pass;
- ESLint: pass;
- TypeScript/Vite production build: pass;
- dependency audit after installing the pinned CLI: zero vulnerabilities.

Required before hosted deployment or desktop connection:

- create a separate hosted Supabase project only with explicit authorization;
- repeat migrations, pgTAP tests, and advisors against a non-production hosted environment;
- complete the production Ed25519 key-generation and custody ceremony;
- configure custom SMTP, production Auth URLs, CAPTCHA/abuse controls, backups, and monitoring;
- review production grants/RLS and backups before deployment.

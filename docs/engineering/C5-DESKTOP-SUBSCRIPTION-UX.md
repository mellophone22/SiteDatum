# C5 — Desktop subscription UX

**Status:** Implemented locally; hosted account/checkout acceptance pending — 2026-10-01

## Delivered vertical slice

Settings now contains a real **Account & Subscription** section. Free remains usable without an account. The section reports the effective plan, verification state, active-project allowance, paid-through date, and the single recovery action appropriate to the current state.

The same surface provides:

- separate licensing account creation and sign-in;
- the real Free, Pro Monthly ($15/month), and Pro Annual ($150/year) comparison;
- server-created Stripe hosted Checkout and customer-portal sessions;
- signed entitlement refresh and bounded offline-grace messaging;
- recovery messaging for past-due, canceled, expired, invalid, or unavailable entitlement evidence;
- sign-out on the current computer without touching the workspace.

The comparison is a dense table rather than a marketing-card grid. Existing Settings typography, separators, focus behavior, responsive stacking, and Light/Dark surfaces are retained.

## Trusted desktop boundary

Licensing is separate from optional metadata Sync. It uses a dedicated Windows Credential Manager namespace for the Supabase Auth session, stable random device identity, and signed entitlement token. Passwords are never stored.

Rust verifies the `sd1` Ed25519 signature, exact schema/issuer/key ID, claim ordering, paid-through time, and refresh horizon before constructing the provider-neutral entitlement. Startup now initializes `CommercialAccess::Enforced` from verified cached evidence, falling back to the non-destructive Free policy when evidence is absent or invalid. Consequential project and Pro-feature checks therefore use the same effective entitlement shown in Settings.

Hosted billing URLs are created only by authenticated Edge Functions. Before Windows opens them, Rust restricts them to HTTPS on `checkout.stripe.com` or `billing.stripe.com`, and the Tauri opener capability repeats that allowlist. Stripe, Supabase licensing, and desktop payloads contain no project names, records, paths, files, or workspace database content.

## Current environment boundary

This checkout is wired to the dedicated SiteDatum licensing Supabase project and its rotatable publishable key. The checked-in Ed25519 public key and key ID are for the current sandbox signing key. No service-role credential, Stripe secret, webhook secret, reconciliation secret, signing private key, or password is present in the client or repository.

Production key custody, environment promotion, code signing, updater signing, and restrictive CSP remain C6 scope. A production release must replace the sandbox public-key configuration through a reviewed build configuration and repeat the complete acceptance matrix.

## Verification

Automated checks cover URL allowlisting, accountless Free status, provider-neutral entitlement evaluation, project/feature enforcement, existing persistence behavior, frontend lint, TypeScript/Vite build, and the full Rust suite.

Hosted acceptance still requires one founder-controlled sandbox account:

1. Create and confirm the licensing account from Settings.
2. Sign in and open monthly Checkout; complete it with Stripe sandbox data only.
3. Refresh entitlement and confirm Pro Monthly, unlimited projects, verified state, and paid-through date.
4. Open the customer portal and return to SiteDatum.
5. Repeat the annual checkout path on a clean sandbox customer or after cleanup.
6. Exercise past-due, recovery, cancellation, expiration, offline grace, sign-out/sign-in recovery, and the two-device limit without using live mode.

Until this manual pass succeeds, C5 remains locally implemented rather than commercially accepted.

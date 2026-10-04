# C9 production commerce promotion

## Decision

SiteDatum will offer paid Pro subscriptions through a controlled unsigned Windows Early Access channel. Code signing remains a future improvement, not a prerequisite for charging, provided the unsigned status, Windows compatibility risk, canonical installer hash, support route, and refund path remain disclosed before purchase and download.

Production commerce is a separate environment from the existing Stripe and Supabase sandboxes. The 1.4.1 sandbox-configured installer is not eligible to become the paid production binary. A new candidate must be built from a clean commit with the `commercial-production` Cargo feature and separate production licensing coordinates.

## Approved architecture

- Stripe-hosted Checkout in subscription mode with Managed Payments enabled.
- Flat-rate Pro Monthly ($15) and Pro Annual ($150) products.
- Accountless local Free tier; an account is required only to purchase, recover, verify, or manage Pro.
- Stripe Customer Portal and Link order management for customer self-service.
- Stripe webhooks are authoritative for paid entitlement state.
- Supabase Auth and Edge Functions form the narrow licensing boundary. Project content never enters Stripe or Supabase.
- Expiration, cancellation, refund, webhook delay, or licensing outage never deletes, moves, hides, or makes existing local records uneditable.

## Environment boundary

The production Supabase project, database, signing key, Stripe webhook endpoint, and Stripe prices must be distinct from sandbox. Server functions require `SITEDATUM_COMMERCE_ENVIRONMENT` and reject a Stripe secret whose mode does not match. They also verify the authenticated Stripe account ID before making billing requests. The desktop production build refuses to compile without an explicit production project URL, publishable key, entitlement verification key, and non-test key ID.

No Stripe secret, Supabase secret key, webhook secret, or entitlement private key is permitted in the desktop, repository, website, release manifest, CI logs, or support messages.

## Production resource checklist

1. Connect and fully activate the live Stripe account; confirm payouts, public business details, support email, statement descriptor, and Managed Payments eligibility.
2. Create live monthly and annual products/prices with the unsigned Early Access name and canonical disclosure URL.
3. Configure Checkout policies, Customer Portal cancellation at period end, payment-method updates, invoice history, and Stripe revenue recovery.
4. Create a dedicated SiteDatum production Supabase project. Apply the reviewed migrations and RLS policies; configure Auth email confirmation and only the canonical SiteDatum redirect URLs.
5. Generate a new production entitlement signing key outside the repository. Store the private key only as a production Edge Function secret and compile the public key/key ID into the production desktop candidate.
6. Configure production Edge Function secrets, including the live restricted Stripe key, expected Stripe account ID, live price IDs, canonical URLs, and an independently generated webhook secret.
7. Deploy functions and register the exact production webhook events. Verify signatures, replay handling, idempotency, delayed delivery, cancellation, refunds, and failed-payment projection.
8. Run a production health check and a live Checkout Session creation without completing payment. Inspect the hosted page for the correct product, amount, renewal terms, unsigned disclosure, support identity, and Managed Payments seller presentation.
9. Complete one founder-controlled low-risk live monthly purchase only after the preceding gates pass. Verify the receipt, webhook, entitlement, portal, cancellation-at-period-end, refund, and non-destructive Free fallback.
10. Build a new unsigned installer with `npm run tauri:build:production`, run the complete C8 acceptance suite, publish a new immutable hash/manifest, upload it without overwriting 1.4.1, and update the canonical Early Access page.

## Stop conditions

Do not enable public purchase or describe the release as paid-production-ready if any of the following is true:

- the connected Stripe account is sandbox-only or not fully activated;
- Managed Payments eligibility or live availability is unresolved;
- the production Supabase project or production entitlement signing key does not exist;
- a live secret, webhook secret, or private signing key appears in source, logs, desktop resources, or the website;
- account ID, price ID, environment, or webhook endpoint validation fails;
- the live lifecycle test has not demonstrated entitlement grant, cancellation, refund, and safe fallback;
- the downloadable installer was built for sandbox, differs from the published hash, or lacks the unsigned Early Access disclosures.

## Rollback

Disable the public purchase buttons first, then disable Checkout Session creation while leaving entitlement refresh, portal access, cancellation, refund processing, local editing, backup, and essential export available. Preserve audit records and correlation IDs without collecting project content. Correct forward, repeat the production gates, and publish a new candidate rather than changing an already published installer in place.

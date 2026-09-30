# C4 — Stripe Managed Payments adapter

Status: hosted Supabase licensing foundation verified; Stripe test configuration and lifecycle proof pending.

## Decision

SiteDatum uses Stripe Managed Payments as the primary merchant-of-record candidate. Stripe Checkout remains hosted. Stripe handles Managed Payments tax, refunds, disputes, and transaction support; SiteDatum retains responsibility for its local product behavior and its signed entitlement projection.

The integration stays behind the provider-neutral licensing boundary introduced in C3. No Stripe SDK, secret, billing record, or project content is added to the desktop application. Only opaque customer/subscription references, normalized subscription state, event identifiers, and payload hashes enter the private licensing schema.

## Implemented test-mode slice

- `stripe-checkout`: authenticated server endpoint; accepts only the two SiteDatum plan names, creates a 30-minute one-time correlation, enables Managed Payments, and returns a hosted Checkout URL.
- `stripe-webhook`: validates the raw request with Stripe's timestamped HMAC signature and five-minute tolerance, rejects malformed payloads, retrieves authoritative subscription state, and projects it transactionally.
- `stripe-portal`: authenticates the SiteDatum customer, resolves the server-held Stripe subscription reference, and creates a short-lived Billing Portal session.
- `stripe-reconcile`: separately authenticated operations endpoint that re-reads every known Stripe subscription and repairs drift.
- Event ledger: unique Stripe event IDs provide idempotency; payloads are represented only by SHA-256 hashes. Delayed snapshots cannot overwrite a newer provider projection.
- Expiration is non-destructive. It changes entitlement status only; local projects remain visible and editable under the C1 policy.

## Refund behavior

Stripe Managed Payments owns the refund transaction and customer workflow. SiteDatum does not infer entitlement loss from a refund object alone. The authoritative Stripe subscription state determines entitlement: if Stripe cancels or ends the subscription, the normal subscription webhook/reconciliation path projects that result. This avoids accidental loss of access from a partial refund while still converging after full cancellation.

## Required test-mode configuration

Create monthly ($15) and annual ($150) recurring test Prices, enable/configure the Billing Portal, and register a test webhook for:

- `checkout.session.completed`
- `customer.subscription.created`
- `customer.subscription.updated`
- `customer.subscription.deleted`
- `invoice.paid`
- `invoice.payment_failed`
- `invoice.payment_action_required`

Set the server-only values listed in `supabase/functions/.env.example`. Use an `sk_test_` key only; the adapter deliberately refuses a live secret key. Never place these values in Git, desktop environment files, or client bundles.

## Remaining acceptance proof

Before Stripe is selected for launch, execute the hosted test matrix for monthly/annual checkout, renewal, failed payment and recovery, cancel-at-period-end, expiration, full and partial refund, duplicate webhook resend, delayed delivery, portal access, and reconciliation repair. Record IDs only in private operator notes, not repository documentation. Live-mode enablement is a separate founder-approved step.

## Hosted test checkpoint — September 30, 2026

- Dedicated Supabase organization and `SiteDatum Licensing Test` project created in `us-east-1`.
- Project reference: `lirkgkiwbffhsmlrfsbp` (public project identifier, not a credential).
- Both C3/C4 migrations applied and matched in remote migration history.
- Hosted database lint returned no schema warnings.
- `licensing-health` deployed and returned schema version 1 with healthy status.
- `licensing-entitlement` deployed and rejected an unauthenticated request with HTTP 401.
- Test signing key ID: `test-2026-09-30-1`.
- Test Ed25519 public key (raw base64): `EG2rGjHrrOM3gUikZvU3s8PCul9IgRFUXwmgVFQ9NSA=`.
- Private signing material and the reconciliation secret exist only in Supabase Edge Function secrets.
- Hosted functions use Supabase's injected server-only service-role credential; no database credential is duplicated in project configuration.
- A sandbox monthly Checkout completed successfully against Stripe Managed Payments and projected an active `pro_monthly` subscription with a future paid-through date.
- Stripe delivered the initial subscription, checkout, and invoice events concurrently. The first event applied correctly, while the other two exposed a correlation-consumption race in the initial projection function. A follow-up migration serializes events per provider subscription with a transaction-scoped advisory lock so later events re-read the committed subscription mapping instead of rejecting a valid correlation.
- A separate sandbox annual Checkout completed successfully and projected an active `pro_annual` subscription with a future paid-through date. Both isolated Checkout correlations were consumed exactly once.
- The annual subscription's concurrent `customer.subscription.created`, `checkout.session.completed`, and `invoice.paid` events all applied without a correlation rejection, providing hosted confirmation of the serialization fix.
- Advancing the annual Stripe test clock through renewal kept the subscription `active` and extended paid access by exactly one annual period.
- The authenticated customer portal opened for the sandbox subscription and scheduled cancellation at the end of the current billing period.
- The cancellation update applied while the projected subscription remained `active` with a future paid-through date. A later out-of-order update was recorded as `ignored_stale`, confirming that cancellation does not prematurely remove paid access and delayed delivery cannot regress newer provider state.
- A monthly test-clock renewal with an unusable payment method produced `invoice.payment_failed` and projected `past_due` without deleting or hiding entitlement history. The first hosted run exposed that Stripe advances `current_period_end` before collection and can deliver `customer.subscription.updated` before the failed-invoice event.
- Two defensive migrations now preserve the last successfully paid-through date for failed/action-required invoice events and for every Stripe projection that remains `past_due`. The second rule is enforced at the subscription table boundary so webhook ordering cannot grant an unpaid period.
- Replacing the payment method and allowing Stripe's scheduled retry produced `invoice.paid`, returned the subscription to `active`, and advanced paid access only after successful collection.
- Immediate cancellation of the terminal test-clock subscription produced `customer.subscription.deleted`. The hosted run exposed that Stripe's terminal object can retain a later unpaid `current_period_end`; terminal Stripe `canceled` now maps to SiteDatum `expired`, while `active + cancel_at_period_end` remains the non-terminal SiteDatum `canceled` state.
- Database enforcement prevents `past_due`, `canceled`, or `expired` projections from extending the last paid period. The regression suite now contains 24 Stripe-adapter assertions (53 licensing database assertions total); local and hosted database lint both report no schema errors.

# C4 — Stripe Managed Payments adapter

Status: hosted Supabase licensing foundation and core Stripe sandbox lifecycle verified; launch audit scenarios remain in progress.

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

The authenticated Checkout boundary consumes a hashed-subject rate-limit bucket before creating a correlation or calling Stripe. The public webhook boundary reads at most 1 MiB by counting streamed bytes before signature verification; it does not trust `Content-Length` as the sole control. Oversized requests receive HTTP 413 and never reach Stripe or the licensing database. The cap is deliberately far above SiteDatum's small allowlisted event payloads while bounding pre-authentication memory use.

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
- A partial refund followed by refunding the full remaining payment completed in the Stripe sandbox. Refund/charge objects did not enter the licensing event ledger, and neither operation changed the authoritative `expired` subscription projection or its last paid-through boundary. This confirms that refund transactions alone cannot accidentally revoke or extend entitlement; subscription cancellation/reconciliation remains authoritative.
- Resending the exact terminal subscription webhook returned the adapter's duplicate result while retaining exactly one provider-event ledger row. The subscription projection remained `expired` with an unchanged paid-through boundary.
- A controlled test-only drift changed the expired monthly projection to active. The separately authenticated reconciliation endpoint re-read both known Stripe sandbox subscriptions and repaired the drifted row to `expired` without extending paid-through. The reconciliation credential was rotated securely in Supabase and was neither printed nor stored locally.

## C8 renewal-order correction — October 2, 2026

A fresh monthly test-clock run reproduced an ordering case that the earlier failure test did not cover: Stripe first delivered an `active` `customer.subscription.updated` snapshot with the next period boundary, then delivered the `past_due` update and `invoice.payment_failed`. The earlier non-active trigger could not undo the extension because the unpaid boundary had already been stored while the subscription still appeared active.

The projection boundary now allows an existing subscription's paid-through timestamp to advance only for an `invoice.paid` event or for payment-aware reconciliation whose expanded latest invoice is paid. Status, cancellation, and metadata updates can still converge without granting an unpaid period. A forward migration, payment-aware reconciliation update, and pgTAP coverage for the exact event order were deployed to the isolated sandbox. Hosted transactional verification confirmed that the pre-failure active update leaves the last successfully paid boundary unchanged. The affected fictional sandbox fixture was corrected back to its last paid invoice boundary; no customer or live-mode data was involved.

After the sandbox customer's payment method was replaced with Stripe's successful test card, the scheduled retry paid the renewal invoice on its second attempt. Stripe returned the subscription to `active`; the applied `invoice.paid` webhook then advanced SiteDatum's paid-through boundary by exactly one month. The following subscription-update delivery was safely classified as stale and could not overwrite the successful projection.

The recovered fixture was then scheduled to cancel at the end of that paid period. Advancing its test clock past the boundary produced `customer.subscription.deleted`; Stripe ended the subscription and the licensing projection changed to `expired` without deleting the row or moving the final paid-through timestamp.

A partial refund and a refund of the full remaining renewal payment both succeeded in Stripe's sandbox. Refund events remained outside the SiteDatum provider-event allowlist, and the subscription projection stayed `expired` with the same final paid-through boundary. This independently reconfirmed that refund transactions neither grant nor revoke product access; authoritative subscription state remains the licensing input.

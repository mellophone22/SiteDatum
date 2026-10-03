# C8 legal and public-surface review package

**Status:** Founder-approved implementation baseline — effective 2026-10-03; qualified legal review waived by founder

## Purpose

This package records verified product behavior, the founder’s policy decisions, and the decision to proceed without qualified legal review. The public policy pages are founder-approved and effective October 3, 2026. This record does not represent attorney review or guarantee compliance in every jurisdiction.

## Founder risk acceptance

On 2026-10-03, Francisco Cabrera directed SiteDatum to bypass qualified legal approval and proceed with implementation. The founder accepts the risk that the founder-drafted Terms, Privacy Policy, and Cancellation/Refund Policy may require correction for a particular jurisdiction, distribution channel, provider term, or future product change. Mandatory legal rights remain unaffected, and future legal review may revise these policies.

On 2026-10-03, the founder also approved a controlled unsigned Windows Early Access route instead of waiting for Microsoft Store Company-account eligibility or a paid signing service. The founder accepts the resulting installation friction, customer-trust, support, refund, and security-communication risk. This approval does not permit SiteDatum to call the build signed, trusted, certified, or generally available, and it does not permit support to advise customers to weaken device security.

## Verified product facts already reflected in the drafts

- SiteDatum is single-user, local-first Windows software created by Francisco Cabrera.
- Free local use is accountless and supports up to three active projects.
- Project records remain in local SQLite and project documents remain ordinary Windows files.
- Billing, licensing, support, and operational systems must never receive project content.
- Pro Monthly is planned at $15/month and Pro Annual at $150/year, with automatic renewal until cancellation.
- One individual Pro subscription supports two active Windows computers and a bounded 21-day unavailable-verification grace period.
- Cancellation preserves Pro through the paid-through date. Expiration never deletes, hides, moves, or makes existing records uneditable; backup and essential export remain available.
- Stripe Managed Payments is the intended merchant of record for eligible transactions. Supabase hosts the separate authentication and licensing boundary.
- No product analytics, remote crash reporting, or operational telemetry are authorized for the initial release.
- Private support, billing, and security intake is `supportsitedatum@protonmail.com`; external delivery has been verified.
- The planned Windows Early Access installer is unsigned. Windows or organization policy may warn or block it, and this must be disclosed before purchase and download.

## Blocking founder and legal decisions

| Decision | Current safe draft position | Required resolution before publication |
|---|---|---|
| Publisher identity and capacity | Founder confirmed “SiteDatum, software created and operated by Francisco Cabrera as an individual developer” on 2026-10-03 | Qualified counsel must confirm any required registration, assumed-name, seller, or address disclosure. Do not publish a home address unless legally required and reviewed. |
| Governing law and venue | Founder selected Florida, United States, on 2026-10-03 | Qualified counsel must supply final governing-law and venue language, dispute process, and mandatory consumer-law exceptions. |
| Retention schedule | Founder approved the periods below on 2026-10-03 | Qualified counsel must confirm regional requirements, legal/accounting holds, backup deletion, and provider-controlled retention. |
| Refund eligibility | Founder approved merchant terms plus mandatory legal rights, with no separate fixed SiteDatum refund window, on 2026-10-03 | Confirm the production merchant terms, regional withdrawal rights, processing disclosures, and effect of an approved full refund on the subscription. |
| Warranty and liability | Founder approved an “as is” warranty disclaimer, exclusion of indirect damages, and a 12-month Pro-fees aggregate cap, subject to non-waivable rights | Monitor provider and launch-market requirements and revise if future legal review identifies a conflict. |
| Age and customer eligibility | Product is not directed to children under 13 | Confirm whether the final terms must limit purchase/use to adults, businesses, or authorized professionals and whether additional age language is required. |
| Policy-change notice | Material changes should be communicated before taking effect | Define delivery method, minimum notice where appropriate, acceptance mechanics, and treatment of existing paid periods. |
| Privacy rights and international processing | Requests route privately; provider-controlled data may require provider action | Honor applicable mandatory rights and update the policy before expanding collection or launch markets. |
| Qualified legal review waived | Founder elected to proceed without counsel on 2026-10-03 | This is a risk acceptance, not evidence of legal compliance. Revisit after any material provider, market, data-collection, or publisher-entity change. |
| Unsigned Early Access distribution | Founder approved the controlled ADR-011 route on 2026-10-03, with pre-purchase and pre-download disclosure and no security-disable guidance | Confirm that the actual checkout and download journey use the approved warning, that the merchant supports the policy-blocked installation refund path, and that no listing implies trusted signing or Microsoft certification. |

## Founder-approved retention schedule for counsel to review

The founder approved these operational periods on 2026-10-03, and they are reflected in the effective Privacy Policy:

| Data class | Proposed period | Reason |
|---|---:|---|
| Active licensing account and entitlement projection | While active, then 24 months after account closure or final subscription end | Recovery, dispute handling, abuse prevention, and reconciliation |
| Content-free licensing audit events | 24 months | Security investigation, entitlement disputes, and idempotency evidence |
| Closed ordinary support messages | 12 months after closure | Follow-up and recurring-defect analysis |
| Security, privacy, billing-dispute, or legal-hold cases | 24 months after closure, or longer only when counsel requires | Incident response and claims |
| Provider identifiers required for accounting/tax | Provider or legally required period | Stripe Managed Payments and applicable law control |
| Local project data | Customer-controlled; no SiteDatum cloud retention | Local-first product boundary |

Backups must age out on a documented schedule after deletion from the active system. A deletion request must not silently delete records subject to an active transaction, security investigation, dispute, or legal hold.

## Founder-approved refund position for counsel and merchant review

1. Customers cancel through the hosted portal and retain Pro through the current paid-through date.
2. Refund requests use the Stripe/Link purchase-support workflow when available, with private SiteDatum Support available for routing.
3. The final policy should use the merchant-approved refund window and preserve all mandatory regional rights rather than promise a period that the merchant or law does not support.
4. A refund object alone does not delete local records or independently determine entitlement. Authoritative subscription state controls the plan projection.
5. No support channel requests full payment-card information.
6. A customer whose device or organization policy blocks the unsigned Early Access installer may request safe installation support or a refund through the original merchant workflow; support does not require disabling security controls.

## Implementation checklist

- [x] Founder confirms publisher identity and capacity.
- [x] Founder approves the retention schedule.
- [ ] Stripe production eligibility and controlling Managed Payments terms are confirmed.
- [ ] Refund eligibility and regional withdrawal rights are approved.
- [x] Founder approves Florida governing law, informal resolution, warranty, and liability language.
- [x] Founder accepts proceeding without jurisdiction-specific legal review.
- [x] Founder approves the controlled unsigned Windows Early Access risk position.
- [x] Terms, Privacy, and Cancellation/Refund pages carry the matching effective date.
- [ ] Checkout presents the applicable terms, renewal, cancellation, refund, and unsigned-installer disclosures before purchase.
- [ ] Public website and Store listing link to the effective policies, not review drafts.
- [x] Founder approval and risk acceptance are recorded in this package and repository history.

## Source notes

Stripe states that Managed Payments acts as merchant of record for enabled transactions and processes transaction and order-level information for checkout, tax, subscription, refund, dispute, and support functions. Stripe also states that a business remains responsible for its own privacy disclosures. These provider facts inform the drafts but do not replace advice from counsel for SiteDatum’s launch markets.

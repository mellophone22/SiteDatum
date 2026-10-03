# C7 — Commercial surfaces

**Status:** Implemented locally; policies founder-approved and effective; public hosting remains deferred — 2026-10-03

## Delivered surface

The repository now includes a static commercial site under `site/`. Its selected visual direction is **Command Center Proof**: a restrained product page centered on a real SiteDatum application capture, a single honest development call to action, a compact proof band, an interactive workflow explanation, the approved Free/Pro matrix, and explicit Windows requirements.

The page does not offer purchase or download controls. Public distribution remains deferred under C6 until a trusted Windows signing route is available. The only external action links to the public GitLab repository or its public issue tracker.

## Product claims and privacy boundary

- Free remains accountless and supports up to three active projects.
- Pro pricing is presented as the approved launch target: $15 monthly or $150 annually.
- Project records remain local in SQLite and project documents remain ordinary Windows files.
- Billing and licensing systems must not receive project names, records, documents, paths, contacts, or database contents.
- Subscription expiration is described as non-destructive; backup and essential machine-readable export remain available.
- Optional metadata synchronization is not advertised.

Screenshots are actual SiteDatum 1.4.0 application captures using illustrative project records. Self-hosted IBM Plex Sans assets and their license are stored with the site so ordinary rendering does not depend on a third-party font request.

## Support and policy status

The support page separates public, fictional-data software reports from private support. Billing, account or entitlement recovery, and security/privacy reports route to `supportsitedatum@protonmail.com`. The public GitLab bug template prominently warns users not to post private project, customer, billing, credential, or secret information. The founder runbook defines content-free internal triage and prohibits copying customer messages or private data into Slack or GitLab. External delivery to the private inbox was verified on 2026-10-03.

Privacy, terms, and cancellation/refund pages are founder-approved and effective October 3, 2026. They document the verified product boundary, minimum licensing data, providers, retention schedule, renewal/cancellation behavior, device and grace policy, non-destructive expiration, private contact, merchant-of-record workflow, warranty and liability terms, and Florida governing-law position. `docs/commercial/C8-LEGAL-REVIEW-PACKAGE.md` records the founder’s decisions and explicit choice to proceed without qualified legal review; it does not claim attorney review or universal legal compliance.

## Verification

Run `npm run test:site`. The validator checks all five pages, local references, required product statements, and policy-draft labels. GitLab frontend quality also runs this check.

## Remaining launch gates

1. Keep the published private support inbox and its recovery method available to the founder.
2. Select and configure public hosting and its domain.
3. Complete the C6 trusted Windows signing/distribution gate.
4. Replace the development-only call to action only after a signed public release is genuinely available.

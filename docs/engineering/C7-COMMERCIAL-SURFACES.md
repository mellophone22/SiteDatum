# C7 — Commercial surfaces

**Status:** Implemented locally; founder/legal review and public hosting remain deferred — 2026-10-02

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

The support page points to the public GitLab issue tracker and prominently warns users not to post private project, customer, billing, or secret information. A private billing-support channel is still required before paid launch.

Privacy, terms, and cancellation/refund pages are included only as review drafts. Each is explicitly labeled **Founder/legal review draft — not yet effective**. They must not be represented as effective policies until publisher identity, contact information, jurisdiction-specific terms, retention details, and merchant-of-record behavior have been reviewed and completed.

## Verification

Run `npm run test:site`. The validator checks all five pages, local references, required product statements, and policy-draft labels. GitLab frontend quality also runs this check.

## Remaining launch gates

1. Complete founder and qualified legal review of all policy drafts.
2. Establish public support and private billing-support contact channels.
3. Select and configure public hosting and its domain.
4. Complete the C6 trusted Windows signing/distribution gate.
5. Replace the development-only call to action only after a signed public release is genuinely available.

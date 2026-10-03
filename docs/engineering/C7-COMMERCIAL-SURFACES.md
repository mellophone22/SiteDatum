# C7 — Commercial surfaces

**Status:** Implemented and hosted; controlled unsigned Early Access route approved but purchase/download remain closed pending a final immutable candidate — 2026-10-03

## Delivered surface

The repository now includes a static commercial site under `site/`. Its selected visual direction is **Command Center Proof**: a restrained product page centered on a real SiteDatum application capture, a single honest development call to action, a compact proof band, an interactive workflow explanation, the approved Free/Pro matrix, and explicit Windows requirements.

The page does not yet offer purchase or download controls because no final Early Access artifact has been published. It now gives the required pre-purchase warning that the Windows installer will be unsigned, may trigger Microsoft Defender SmartScreen, and may be completely blocked by Smart App Control or organizational policy. A dedicated Early Access guide repeats the warning before download, prohibits weakening device security, explains artifact verification, and routes policy-blocked installation to private support and the refund process.

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

Run `npm run test:site`. The validator checks all six pages, local references, the Early Access disclosures, required product statements, and policy-draft labels. GitLab frontend quality also runs this check.

## Remaining launch gates

1. Keep the published private support inbox and its recovery method available to the founder.
2. Produce the immutable Early Access candidate and publish its version, commit, SHA-256, Authenticode status, and date.
3. Put the same unsigned-installer disclosure into the real hosted checkout before accepting payment.
4. Verify the public-origin download/install and policy-blocked refund/support paths before replacing the development-only call to action.

# SiteDatum 1.4.4 publication plan

**State:** Verified publication candidate for the authorized controlled soft-launch channel on October 8, 2026

## Immutable candidate identity

- Source commit: `c6dd1f78846555c5acfb7621e6ac4acf780319f4`
- Installer: `SiteDatum_1.4.4_x64-setup.exe`
- Size: 4,731,187 bytes
- SHA-256: `264D91B38AF4C3251FB04CF23193744FF4B7BFC1DDB58892CC4F04480CD633CF`
- Authenticode: `NotSigned`
- Production entitlement key ID: `prod-2026-10-06-2`
- Intended canonical URL: `https://sitedatum.site/downloads/SiteDatum_1.4.4_x64-setup.exe`

The local staged installer must remain byte-for-byte identical to this identity. If any source, configuration, packaging input, or executable byte changes, discard this publication plan and generate a new candidate, hash, and manifest.

## Included changes

- Added the in-app Help section for local-first workflow, accountless Free use, Pro capabilities, backup and recovery, licensing, privacy, and private support.
- Added explicit startup-recovery and backup-validation states that preserve the unavailable database rather than replacing customer data.
- Expanded Free portability to a complete protected CSV export of the core workspace.
- Added signed-in self-service for reviewing active Windows computers and removing another device without sending device names or workspace information.
- Hardened Windows-path, filesystem, PDF, licensing-transition, and release-provenance behavior with focused regression coverage.

## Production boundary

- The production build used only the approved Supabase project URL, modern publishable key, entitlement public verification key, and key ID.
- The compiled application contains the production Supabase origin and replacement key ID, excludes the sandbox origin and prior key ID, and contains no private-key, Stripe-secret, webhook-secret, or reconciliation-secret identifiers.
- Project names, tasks, RFIs, submittals, notes, contacts, file paths, documents, and database contents remain outside billing and licensing systems.

## Automated acceptance

- 31 frontend test files and 98 frontend tests passed.
- 64 Rust tests passed both normally and with `commercial-production` enabled.
- Lint, production frontend build, nine-page commercial-site validation, release-manifest verification, and the high-severity npm audit passed.
- The complete C8 audit reported 20 passed, 3 documented deferrals, and 0 failed.
- The Windows CI workflow now exercises the repository's canonical manifest generation and verification commands before confirming the CI artifact remains unpublished.

## Focused acceptance

- The exact candidate is staged in the established disposable Windows profile test folder.
- Publication remains blocked until the operator confirms version 1.4.4, Help access, startup, and preservation of the existing fictional workspace and records.

## Public-origin verification

- Publication must add `SiteDatum_1.4.4_x64-setup.exe` without replacing or deleting preserved earlier versioned installers.
- After deployment, download the canonical public-origin copy, verify the exact size and SHA-256 above, confirm `NotSigned`, and verify every commercial page returns HTTP 200 with the 1.4.4 release identity.

## Controlled soft-launch boundary

This candidate is limited to the already authorized controlled unsigned Windows Early Access channel. It does not authorize a founder self-purchase, synthetic live transaction, signed-general-availability claim, or broad promotion.

No founder self-purchase or contrived live transaction is required. Simulated lifecycle acceptance remains in Stripe sandbox. Keep broad promotion paused until the first genuine customer transaction demonstrates the correct receipt, successful webhook projection, Pro entitlement, and customer-portal route. Production cancellation or refund is performed only for a legitimate customer request or policy obligation.

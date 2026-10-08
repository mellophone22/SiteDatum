# SiteDatum audit remediation closeout

**Status:** Engineering remediation complete; controlled soft-launch gates remain in force
**Date:** 2026-10-08
**Audit baseline:** `3e8bcebc5961efd82766b14f381edd5fdc198839`

## Purpose

This ledger closes the implementation roadmap created from the full static SiteDatum audit. It does not relabel deferred work as complete, authorize broad promotion, publish a release, or weaken the local-first and customer-data boundaries. It distinguishes corrected findings from accepted Early Access constraints and future general-availability work.

## Completed remediation packages

| Package | Result | Evidence commit |
| --- | --- | --- |
| 2. Immediate trust and accessibility | Dark action/focus/placeholder contrast, CSV formula neutralization, website security/metadata/purchase guidance, and file-opening documentation corrected | `66e27cb` |
| 1. Sync containment and privacy | Deferred Sync hidden and rejected for fresh installations; controlled grandfathered path and public privacy disclosure retained | `49ea761` |
| 3. Startup recovery and backup resilience | Recoverable startup failure surface, external/scheduled backup support, visible locations, and restore-from-file safeguards | `b7c2f01` |
| 4. Complete Free data portability | Accountless export of every core entity with manifest/count evidence and spreadsheet-safe text | `6b51d70` |
| 5. Device self-service and continuity | Two-computer visibility/deactivation/replacement, continuity messaging, and expired-session recovery | `f4571c8`, `8124517`, `dc4d824` |
| 6. Help and product guidance | Permanent offline Help, actionable empty states, field errors, keyboard focus, and command discovery | `214df16` |
| 7. Windows/PDF/filesystem validation | Native path/file tests strengthened; PDF structure and rendered layout verified; collision and artifact-preservation evidence recorded | `8eac3f1` |
| 8. Windows CI and provenance | Hosted Windows test/package/smoke job plus staged manifest generation and independent artifact verification | `a999028` |
| 9. Closeout polish | Current-channel rollback guidance, provenance in release gates, verification-count refresh, and this residual-risk ledger | Current package |

The immediate trust package was implemented first under the audit's requested Work Package A label. Its roadmap position corresponds to the audit's immediate trust/accessibility work; the naming difference does not represent an omitted package.

## Current launch boundary

SiteDatum may remain available through the already approved **controlled unsigned Windows Early Access** soft launch. Free remains accountless and local. The website, installer disclosure, support/refund route, production licensing boundary, and public-origin 1.4.3 artifact were separately accepted before this remediation closeout.

Broad promotion remains paused until the first genuine customer transaction demonstrates all of the following without founder self-purchase or a contrived live payment:

- correct live Checkout product, amount, interval, and Early Access disclosure;
- successful verified webhook projection;
- correct Pro entitlement and paid-through boundary;
- customer access to the hosted billing portal; and
- no loss or restriction of local project work if the commercial perimeter fails.

Every future candidate also requires a successful GitHub Actions run for its exact commit, including the Windows native/provenance job introduced in package 8. Local success alone is not hosted verification.

## Accepted controlled-Early-Access risks

| Risk | Current control | Revisit trigger |
| --- | --- | --- |
| Unsigned installer and possible SmartScreen, Smart App Control, antivirus, or organization-policy block | Prominent pre-purchase/pre-download disclosure; immutable version/hash metadata; support never instructs customers to weaken security | Before describing SiteDatum as trusted, signed, certified, or generally available |
| Manual updates only | Each installer is an independently verified immutable candidate; no unsigned automatic updater | When a trusted signing and updater-custody route is qualified |
| Founder proceeded without qualified legal review | Founder-approved policies and explicit risk acceptance are recorded; private support/refund process exists | Material policy, jurisdiction, seller-identity, data-use, or distribution change |
| Local database is not application-level encrypted | Windows account protections and BitLocker/device security are the stated controls; project records remain local | Enterprise/security requirement or credible threat model requiring separate at-rest encryption |
| No product telemetry or remote crash reporting | Structured local errors and content-free correlation references; support is deliberate and private | Separate data-minimization, consent, retention, redaction, and privacy approval |
| Registered documents are revealed in Explorer rather than executed by SiteDatum | Avoids silently executing customer-controlled files; documentation matches behavior | Separately reviewed file-opening threat model and explicit user experience |
| Cross-volume Move may be refused by Windows rename semantics | Failure is explicit and non-successful; Register and Copy remain available; no implicit copy/delete fallback | Dedicated partial-failure-safe cross-volume move design |
| Activity and contacts remain workspace-wide | UI states the scope honestly; no fabricated project relationship | Schema/product requirement for project-scoped activity or contact relationships |

## Deferred—not accepted as complete

- Trusted Authenticode or Microsoft Store signing and a signed automatic updater remain general-availability work.
- Broader metadata Sync remains deferred pending redesign, hosted authorization/RLS review, consent, retention/deletion, and recovery validation. The legacy grandfathered path must not be offered to new users.
- The first genuine production purchase and its content-free operational evidence remain a broad-promotion gate.
- The new Windows GitHub Actions job is locally validated but remains **hosted verification pending** until this commit is pushed and the exact run succeeds.
- Future release candidates still require disposable-profile install/upgrade/reinstall, offline/local work, backup/export, keyboard, scaling, migration, and public-origin hash acceptance as applicable.

## Explicitly not introduced

The remediation added no AI/LLM capability, collaboration, organizations, role-based access, seat billing, mandatory account, project-content telemetry, direct merchant credentials in the desktop, or cloud requirement for ordinary project work. It did not delete or relocate customer records, project files, legacy hosted data, or Git history.

## Closeout decision

The audited engineering gaps in the remediation roadmap are addressed or bounded by explicit controls. Remaining items above are operational gates or consciously accepted Early Access constraints, not silent claims of completion. A new audit is required before general availability, trusted automatic updating, broader Sync, or any material expansion of collected customer data.

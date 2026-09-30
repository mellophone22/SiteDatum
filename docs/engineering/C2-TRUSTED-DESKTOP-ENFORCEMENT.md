# C2 Trusted Desktop Enforcement

Status: Completed 2026-09-29

## Outcome

C2 connects the provider-neutral entitlement policy to consequential Rust command boundaries. The checks run before database or filesystem mutations and do not rely on disabled interface controls.

The checked boundaries are:

- creating a project;
- restoring an archived project to active status;
- bulk work-item status changes;
- creating and applying project templates;
- previewing and committing CSV or Excel batch imports;
- Excel work-item export while retaining CSV export;
- RFI PDF generation; and
- formatted operational report generation.

Project editing, record editing, manual register workflows, project archiving, local backup/restore, recovery, ordinary Windows file access, and CSV work-item export remain outside paid enforcement.

## Project-limit behavior

The database provides archive-aware queries for the active-project count and a project's archived state. Creation checks the active count before creating a folder. Restoring checks only when the target project is currently archived, so repeating an active-state update does not consume capacity. The database mutex keeps each count-and-mutation command serialized within the desktop process.

Free activation is allowed only when the resulting active count is three or fewer. Existing workspaces already above the limit remain visible and editable, and projects can still be archived.

## Pre-commercial compatibility

C3 is responsible for producing authenticated, tamper-resistant entitlement evidence. Until that trusted source exists, the application starts in an explicit `Precommercial` access mode so this intermediate package does not disable existing controls without an upgrade or recovery path.

`Precommercial` is not a customer plan, is not stored in project data, and is not a plaintext Pro flag. Every command boundary is already connected to the enforcement layer. C3 must replace the startup mode with `CommercialAccess::Enforced(effective_entitlement)` before commercial gating is enabled or a paid build is released.

## Denials and privacy

Enforced Free denials return stable structured codes:

- `ACTIVE_PROJECT_LIMIT_REACHED` for project creation or restoration at the limit; and
- `PRO_FEATURE_REQUIRED` for a Pro-only workflow.

Technical details contain only the active-project count or the policy feature identifier. They never contain project names, record content, paths, files, or customer data.

## Verification

Pure tests cover pre-commercial compatibility, the Free activation boundary, unlimited effective Pro activation, Free portability, and each enforced Pro feature. Persistence coverage verifies that archived projects are excluded from the active count and correctly identified for restoration checks.

No visual review is required because C2 changes no rendered interface. Subscription status and contextual upgrade UI remain C5 scope.

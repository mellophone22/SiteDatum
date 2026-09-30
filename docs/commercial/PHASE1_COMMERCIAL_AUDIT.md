# Phase 1 commercial architecture audit

**Audited:** 2026-09-29

**Repository state:** SiteDatum 1.4.0 on `codex/ux-1.1-project-centric-workspace`

**Scope:** Documentation and architecture only; no billing behavior, accounts, secrets, provider resources, or telemetry were added.

## Executive recommendation

Proceed with an accountless Free edition and a narrowly isolated Pro licensing perimeter. Do not reuse the current optional-sync login as the commercial identity and do not connect the desktop directly to privileged merchant APIs. Evaluate Lemon Squeezy first as merchant of record, keep Paddle as the fallback, and place provider behavior behind a small trusted licensing service.

The entitlement decisions required for the local C1 policy package are resolved. Work that depends on external services should wait for a dependable source/release host and explicit provider authorization. Paid public launch additionally requires code signing, signed updates, legal/support surfaces, provider approval, and end-to-end payment/recovery testing.

## 1. Desktop, packaging, and updates

- Tauri 2.11, React 19, TypeScript 6, Rust 2021, Vite 8, and bundled SQLite form the desktop stack.
- `tauri.conf.json` builds Windows bundles under the stable application identifier `com.cabre.project-engineer-workspace`, preserving existing application-local data across the SiteDatum rebrand.
- Version 1.4.0 NSIS output and standalone executable are checked into `releases/1.4.0` with SHA-256 manifests.
- Releases are not Windows code-signed and currently produce an unrecognized-publisher warning.
- No Tauri updater plugin, updater public key, update endpoint, or updater artifact configuration exists.
- No tracked CI/CD pipeline exists. Builds and verification are local.
- The configured GitHub account is suspended, so the present GitHub download/release path is not dependable.

**Commercial implication:** signed installer and updater design is a launch gate, not a post-launch enhancement. Tauri requires signatures for updater artifacts; the private signing key must never enter the repository and needs an independent recovery backup.

## 2. Identity and existing cloud behavior

- Free local operation currently requires no account and starts offline.
- Optional Supabase synchronization uses email/password against one hard-coded Supabase project.
- The documented operating model manually provisions and confirms a user in Supabase rather than offering customer registration or recovery.
- Access and refresh tokens plus email are serialized into a Windows Credential Manager entry. Passwords are not persisted.
- The Supabase project URL and publishable client key are duplicated in `cloud_auth.rs` and `cloud_sync.rs`. The publishable key is not a server secret, but its authorization safety depends entirely on RLS and database functions.
- Optional sync uploads a workspace metadata snapshot; it is therefore intentionally more sensitive than a licensing identity and must remain separately consented and entitled.

**Decision:** do not silently turn this manually provisioned sync identity into the commercial account. A later implementation may use Supabase infrastructure for a licensing service only after a separate schema/RLS/API design and live security review. New exposed tables require explicit Data API grants as well as RLS under current Supabase behavior.

## 3. Backend and network dependencies

- The normal application has no backend dependency.
- Rust uses blocking `reqwest` only for optional Supabase authentication and sync.
- There is no billing backend, public application API, webhook receiver, subscription projection, customer portal integration, entitlement signing service, or rate limiting.
- No merchant or billing dependencies are installed.

**Commercial implication:** the trusted licensing service is a new deployable component. It must be designed and reviewed independently rather than embedded in the desktop or appended to the sync snapshot.

## 4. Data and storage architecture

- SQLite is stored under the Windows application-local data directory and migrated through ordered repository SQL migrations.
- Rust owns persistence, backups, restore, imports/exports, reports, and filesystem operations.
- Project documents remain ordinary Windows files beneath a configured local or UNC root.
- Local backup/restore, integrity preview, missing-file recovery, and path rebasing already provide a strong non-destructive foundation.
- Optional sync serializes implemented metadata and translates paths relative to the project root. The live SQLite file is never placed in OneDrive or Supabase.

**Commercial implication:** subscription state should not be mixed into project documents or used as the source of truth for customer work. Local UI cache and secure entitlement material should be device state, while authoritative subscription state remains in the licensing service.

## 5. Launch feature inventory

The current product already implements Projects, Tasks, Attention, RFIs, Submittals, registered files/drawings, Notes, Contacts, Activity, Search/Command Palette, Project Controls, field and commissioning registers, calendar/reminders, project templates, bulk updates, CSV/Excel import/export, printable reports, backup/restore, recovery, appearance modes, first-run setup, and optional sync.

The complete initial Free/Pro allocation is recorded in `docs/commercial/ENTITLEMENT_MATRIX.md`. Free includes full manual register workflows within three active projects plus backup and machine-readable portability. Pro adds unlimited projects, templates, bulk operations, advanced interchange, and professional outputs. Optional sync is deferred from initial public launch. Pro uses a 21-day verification grace period and supports two active Windows computers.

## 6. Settings and account UI

- Settings currently contains Appearance, Workspace, optional Sync, Notifications, and Data & Recovery.
- There is no Account or Subscription section, pricing screen, upgrade flow, checkout recovery, billing portal link, entitlement status, or offline-grace explanation.
- First-run setup deliberately excludes cloud sign-in, which is compatible with accountless Free.

**Commercial implication:** future subscription UI belongs in Settings and contextual limit explanations. It must not interrupt first-run local setup or turn ordinary project work into an authentication gate.

## 7. Analytics, diagnostics, and support

- No analytics or telemetry SDK is installed.
- No remote crash/error reporting is installed.
- Structured application errors include stable codes, recovery guidance, correlation IDs, and technical detail written to stderr; there is no support bundle or Send Feedback flow.
- No privacy-safe event schema, consent policy, retention policy, or diagnostics redaction contract exists.

**Decision:** telemetry remains unauthorized in Phase 1. Billing-provider dashboards should cover initial revenue metrics. Any product telemetry or remote error reporting requires a separate data-minimization proposal and privacy-policy update.

## 8. Existing licensing and billing

- No licensing, billing, plan, entitlement, trial, or subscription code exists.
- No local `isPro` flag or hidden monetization placeholder exists.
- No prices are embedded in product code.

This is a favorable starting point for a centralized capability service rather than retrofitting scattered UI checks.

## 9. Security observations

Strengths:

- Rust-owned database and filesystem boundaries.
- Parameterized SQLite operations and constrained domain validation.
- Windows Credential Manager for optional-sync session material.
- Explicit conflict resolution and non-overwrite filesystem behavior.
- Locked dependency versions and committed lockfiles.

Commercial blockers and review items:

- Tauri CSP is currently `null` and must be replaced with a tested restrictive policy before public paid launch.
- Releases are unsigned and there is no signed updater.
- The commercial service needs authenticated authorization, webhook signature verification, idempotency, replay/ordering handling, rate limiting, secret rotation, redacted logs, and cross-customer isolation.
- Client-side controls cannot be authorization. Consequential entitlement limits must be checked at the Rust mutation boundary and, for server resources, by the server.
- The existing sync RLS/function deployment is documented but not live-audited during this local Phase 1 review.
- The duplicated Supabase public configuration should later be centralized; no service-role or merchant secret was found in the repository.

## 10. Test and release pipeline

- Package scripts cover ESLint, Vitest, TypeScript/Vite production build, and Tauri commands.
- Rust has domain, persistence, filesystem, sync snapshot, reports, recovery, and realistic-volume tests.
- The last verified state passed 59 frontend tests, 28 Rust tests, lint, TypeScript/Vite build, Rust formatting, and NSIS packaging before reproducible build caches were removed.
- A Playwright visual runner exists but Playwright is not declared in this checkout, so it is not a reproducible default gate.
- No automated CI, secret scanning, dependency-audit gate, signed build pipeline, or deployment promotion/rollback pipeline exists.

## Merchant-of-record evaluation

### Primary candidate: Lemon Squeezy

Reasons to evaluate first:

- Merchant-of-record handling for software/subscriptions.
- Hosted storefront/checkout and hosted customer portal.
- Subscription lifecycle webhooks with a signing secret.
- License keys tied to subscription lifecycle and an HTTPS validation API.
- Test mode and customer-visible self-service billing.

Important constraints:

- Merchant/store approval and identity verification are required.
- Published fees are higher than direct processing and may include international, PayPal, and subscription additions.
- License API limits and activation behavior must be tested.
- Direct desktop validation alone is insufficient for the full desired trust model; webhook projection and signed SiteDatum entitlements still favor a minimal service.

### Fallback: Paddle

Paddle should be evaluated if Lemon Squeezy approval, payout support, economics, lifecycle coverage, or support experience is unsuitable. The provider-neutral adapter prevents the entitlement model from depending on either provider's vocabulary.

### Selection gate

Do not choose a provider in code until test mode proves checkout, cancellation, renewal, failed payment, expiration, refund, duplicate/delayed webhook handling, customer self-service, and entitlement recovery.

## Identity and offline-entitlement design

1. Free starts with a local anonymous workspace identity and no registration.
2. Upgrade opens hosted checkout. A one-time correlation value associates the completed purchase with a later activation without exposing merchant credentials.
3. Pro activation verifies email/purchase through the licensing service. The service stores the provider customer/subscription mapping and returns a bounded signed entitlement containing only subject, plan, status, issued/expiry timestamps, and entitlement schema version.
4. Secure refresh material and the cached entitlement are stored using Windows Credential Manager or an equivalently protected platform facility.
5. The desktop verifies entitlement authenticity locally and refreshes opportunistically, not on every Pro action.
6. Temporary network failure enters a visible grace state. Grace exhaustion applies Free creation limits but retains editing and data access.
7. Sign-out removes local licensing credentials without touching the workspace. Sign-in/activation on a supported replacement device restores the entitlement.

## Threat model summary

| Threat | Required control |
| --- | --- |
| Editing local plan state | Signed/tamper-resistant entitlement; Rust-boundary checks |
| Shipping merchant secrets | Backend-only secrets and artifact scanning |
| Forged provider events | Constant-time webhook signature verification and timestamp/replay policy |
| Duplicate or reordered webhooks | Persist provider event IDs; idempotent state projection; reconcile from provider state |
| Cross-customer access | Subject-bound authorization, ownership checks/RLS, negative integration tests |
| Offline clock rollback | Bounded grace, monotonic last-verification evidence where practical, conservative recovery |
| Stolen cached credential | Windows protected credential storage, revocable refresh material, minimal claims |
| Provider outage | Cached grace and accountless local operation |
| Licensing-service outage | Cached grace; no impact on local data or Free workflows |
| Accidental project-content collection | Explicit payload schemas, allowlisted logs/events, redaction tests |
| Bad release or migration | Signed updates, pre-update backup, migration tests, rollback runbook |
| Lost updater signing key | Offline protected backup and documented key custody/rotation plan |

## Implementation work packages after Phase 1

1. **C1 — entitlement domain:** pure provider-neutral policy, three-project limit, downgrade semantics, tests; no network.
2. **C2 — Rust enforcement:** enforce project activation/creation and selected advanced operations at mutation boundaries; retain all existing editing and recovery behavior.
3. **C3 — licensing-service foundation:** identity, subscription projection, signed entitlements, migrations, authorization, rate limiting, audit logging, health checks.
4. **C4 — provider test adapter:** hosted checkout, verified/idempotent webhooks, portal, reconciliation jobs, refunds/cancellation test matrix.
5. **C5 — desktop subscription UX:** Account/Subscription settings, pricing comparison, upgrade, pending/recovery states, portal, grace/expiration messaging.
6. **C6 — release safety:** CSP, Windows code signing, signed updater, stable hosting, automated build/test/audit pipeline, rollback.
7. **C7 — commercial surfaces:** website, system requirements, real screenshots, pricing, support, founder-reviewed legal policies.
8. **C8 — launch audit:** disposable-profile install/upgrade, payment lifecycle, reinstall/device recovery, offline behavior, incident and refund runbooks.

Each package requires its own plan, tests, verification, documentation, and commit. No package authorizes the next one's provider accounts or production deployment automatically.

## Founder actions

Resolved commercial policy:

- Free includes complete manual register creation/editing within three active projects.
- Backup, complete machine-readable CSV export, and user-data portability remain Free.
- Excel interchange, batch import, templates, bulk actions, formatted reports, and generated professional deliverables are Pro.
- Optional metadata synchronization is deferred from the initial commercial launch and may later return as Pro after redesign/security validation.
- Pro receives a 21-day unavailable-verification grace period and supports two active Windows computers.

Required before provider integration:

- Confirm business/legal seller identity and payout country.
- Apply for Lemon Squeezy test/live store approval; evaluate Paddle in parallel only if needed.
- Establish support email/domain and public website ownership.
- Obtain founder/legal review of Terms/EULA, Privacy, Refund/Cancellation, and pricing disclosures.

Required before paid launch:

- Restore or replace dependable source/release hosting after the GitHub suspension.
- Acquire and protect Windows code-signing and updater-signing credentials.
- Select licensing-service hosting, database, secret manager, monitoring, and backup ownership.
- Complete merchant test/live configuration without committing credentials.

## Phase 1 exit status

Phase 1 architecture and product policy are documented, and the repository operating rules permit only the narrow commercial boundary described by ADR-006. All five entitlement decisions required for C1 are confirmed. No paid feature gating or external commercial dependency is implemented. C1 may begin as a local, provider-neutral domain package; external accounts and deployment remain separately gated.

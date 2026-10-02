# Implementation Plan

WP0 Foundation: validate stack, scaffold, SQLite/migrations, shell, settings, logging/errors, tests.

WP1 Projects/filesystem: CRUD/archive/pin, root, folder preview/creation, Projects list, Overview, Open Folder.

WP2 Tasks/Attention: task lifecycle, due/follow-up/waiting, Attention engine, dashboard, filters, quick-add. (Completed 2026-09-22)

WP3 RFIs: lifecycle, table/detail, attachments/relationships, Attention integration. (Completed 2026-09-22)

WP4 Submittals: lifecycle/dispositions/revisions, table/detail, Attention integration. (Completed 2026-09-22)

WP5 Drawings/files: registration, Copy/Move/Register, collisions/missing-file recovery, Open/Explorer. (Completed 2026-09-22)

WP6 Notes/Contacts/Activity. (Completed 2026-09-22)

WP7 Search/command/navigation polish: global search, command palette, recents, remembered state, keyboard UX.

WP8 Hardening: backup/export, edge cases, realistic-volume performance, accessibility, final UI audit, Windows packaging. (Completed 2026-09-22)

WP9 Optional cloud sync: dedicated OneDrive project root for normal files; authenticated Supabase metadata sync over a Rust-owned boundary; explicit version conflicts; device-local path mapping; offline-safe local operation. (Completed 2026-09-22.)

WP10 RFI PDF export: persist template-specific RFI fields; render saved RFI/project data onto the approved Excel-derived PDF template; explicit destination; no overwrite; register generated output as an attachment reference. (Completed 2026-09-22.)

WP11 Task progress: add a compact, data-driven completion gauge to Tasks and Attention; scope it to the selected project; exclude cancelled tasks from the denominator; retain text counts and accessible labels. (Completed 2026-09-22.)

WP12 Workflow acceleration: saved task views, global quick capture for tasks/notes/RFI drafts/contacts, and a project-context command-center overview. (Completed 2026-09-22.)

WP13 Batch and project setup: transactional bulk status actions plus reusable project templates for task checklists and milestones. The standard project folder tree remains automatically created by the existing project workflow. (Completed 2026-09-22.)

WP14 Data mobility and documents: previewed transactional CSV/Excel import, filtered CSV/Excel export, and versioned printable HTML reports for submittal covers, transmittals, meeting minutes, contact lists, open-item reports, and weekly status reports. (Completed 2026-09-22.)

WP15 Planning and reminders: opt-in Windows notifications for due and overdue work while the app is running, plus a restrained dated register for tasks, follow-ups, RFIs, submittals, and milestones. (Completed 2026-09-22.)

WP16 Audit and recovery: searchable activity, missing-file diagnostics and recovery routing, current sync-conflict state, backup inventory, integrity/count preview, and confirmed restore with a pre-restore safety backup. (Completed 2026-09-22.)

WP17 Project controls registers: meeting minutes/action ownership, procurement, change events, transmittals, and milestones with domain-specific validation and project relationships. (Completed 2026-09-22.)

WP18 Field and commissioning registers: punch list, daily reports, startup/commissioning checklists, and commissioning issues with domain-specific validation and project relationships. (Completed 2026-09-22.)

Each work package must be independently usable and tested before proceeding.

## Commercial launch track

Commercial work is a post-MVP track governed by `ADR-006-COMMERCIAL-LICENSING-BOUNDARY.md`; it does not replace the local-first product architecture.

- **Commercial Phase 1 — audit and architecture:** repository audit, narrow operating-rule amendment, Free/Pro baseline, licensing boundary, threat model, provider evaluation criteria, launch blockers, and founder decisions. (Completed 2026-09-29; no billing behavior implemented.)
- **C1 — entitlement domain:** provider-neutral capability policy, three-active-project Free limit, complete manual Free workflows, portability guarantees, 21-day unavailable-verification grace, two-device Pro allowance, non-destructive downgrade rules, and pure tests. (Completed 2026-09-29; dormant policy only, with no enforcement or provider integration.)
- **C2 — trusted desktop enforcement:** enforce consequential limits at Rust mutation boundaries while preserving all existing data access, editing, backup, and recovery. (Completed 2026-09-29; command-boundary enforcement is wired and tested, with pre-commercial compatibility retained until C3 supplies trusted entitlement evidence.)
- **C3 — licensing service:** minimal paid identity, subscription projection, signed entitlements, authorization, rate limiting, audit logging, and operational health. (Completed 2026-09-30 and deployed to the dedicated SiteDatum Supabase sandbox; production promotion remains outside this package.)
- **C4 — merchant test adapter:** hosted checkout, verified and idempotent webhooks, customer portal, reconciliation, cancellation, refund, and failure-path tests. (Completed 2026-09-30 in the Stripe and Supabase sandboxes; no live-mode resources or charges were used.)
- **C5 — subscription UX:** Account/Subscription settings, real Free/Pro comparison, upgrade, pending/recovery states, portal access, and offline-grace messaging. (Implemented locally 2026-10-01; founder-controlled hosted account/checkout acceptance remains pending.)
- **C6 — release safety:** restrictive CSP, Windows code signing, signed updater, stable hosting, automated verification, migration safety, and rollback.
- **C7 — commercial surfaces:** website, support, pricing, system requirements, and founder/legal-reviewed policies. (Site, support guidance, pricing, and requirements implemented locally 2026-10-02. Policy pages remain clearly marked drafts pending founder/legal review; public hosting and launch remain gated.)
- **C8 — launch audit:** disposable-profile install/upgrade, full payment lifecycle, reinstall/device recovery, offline behavior, incident response, and founder operations runbook. (Preflight harness and acceptance matrix implemented 2026-10-02; automated local gates are available while disposable-profile, sandbox lifecycle, legal, and trusted-distribution evidence remain pending.)

No commercial work package authorizes production provider accounts, secrets, deployment, or telemetry for a later package. Resolve and record its required founder decisions before implementation.

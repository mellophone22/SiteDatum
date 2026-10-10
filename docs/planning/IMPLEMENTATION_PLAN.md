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

WP9 Optional cloud sync: dedicated OneDrive project root for normal files; authenticated Supabase metadata sync over a Rust-owned boundary; explicit version conflicts; device-local path mapping; offline-safe local operation. (Completed 2026-09-22; contained to grandfathered computers for the initial commercial launch on 2026-10-08.)

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
- **C5 — subscription UX:** Account/Subscription settings, real Free/Pro comparison, upgrade, pending/recovery states, portal access, and offline-grace messaging. (Completed and accepted in the isolated Stripe/Supabase sandbox on 2026-10-03; no live-mode resources or charges were used.)
- **C6 — release safety:** restrictive CSP, automated verification, migration safety, rollback, and controlled distribution. (Completed for the unsigned Early Access route on 2026-10-03; trusted signing and automatic updates remain deferred to general availability.)
- **C7 — commercial surfaces:** website, support, pricing, system requirements, and founder-approved policies. (Completed and publicly hosted on `sitedatum.site` on 2026-10-03 with the canonical Early Access warning, download, support, and refund journey.)
- **C8 — launch audit:** disposable-profile install/upgrade, full payment lifecycle, reinstall/device recovery, offline behavior, incident response, and founder operations runbook. (Completed for controlled unsigned Early Access on 2026-10-03; public-origin verification and matching GitLab CI passed.)
- **C9 — production commerce promotion:** separate production licensing coordinates, least-privilege live Stripe access, production webhook validation, controlled soft-launch monitoring, and a new immutable production installer. (Controlled soft launch open as of 2026-10-06; the production boundary, provider configuration, webhook, authenticated no-payment Checkout inspection, immutable 1.4.3 installer, focused acceptance, publication, and public-origin hash verification have passed. Broad promotion remains gated on validation of the first genuine customer transaction. Simulated payment lifecycle testing remains in Stripe sandbox.)
- **C10 — optional Sync v2 reintroduction:** a new encrypted record-level protocol, isolated and reproducible hosted boundary, explicit consent/retention/deletion, owner-scoped RLS, device/key recovery, and verified operational restore. (C10-01 and C10-02 are complete. Disposable local C10-03A through C10-03H now cover identity/RLS, first-device enrollment, the trusted initial bridge/native path, approved-device authorization, authenticated HPKE workspace-key transfer, atomic encrypted hosted record/checkpoint operations, and signed workspace disable/recovery/deletion with content-free receipts and scheduled retention. C10-04A adds the desktop's canonical encrypted workspace-checkpoint verification and durable local anti-rollback anchor. No hosted production resource, customer runtime access, or Sync UI is enabled; Legacy Sync remains contained behind `SYNC_DEFERRED`. Safe tombstone compaction still depends on later C10-04 durable device acknowledgements and replacement-checkpoint proof; remaining C10-03 reconstruction, advisor, and secret-proof work is pending.)

- **C10-04B — encrypted local queue:** versioned allowlisted record codecs and tombstones, authenticated record encryption, transactional encrypted staging/outbox, exact receipt retries, and atomic checkpoint-covered pull cursor/anchor/device staging acknowledgements. (Implemented and locally verified 2026-10-10. No project-table application or runtime Sync activation; live adapters, backup-before-apply, conflicts, partial paging, key rotation, and applied-device acknowledgements remain C10-04C/later work. Staging acknowledgement does not permit hosted tombstone compaction. Evidence: `docs/engineering/C10-04B-ENCRYPTED-LOCAL-QUEUE.md`.)

- **C10-04C1 — backed-up notes/contact apply:** the first partial C10-04C slice rehearses a complete verified page in a private SQLite candidate, creates a verified local safety backup, and atomically applies supported live rows with the staging cursor/anchor. Divergent local work and unsupported kinds are refused. No runtime Sync activation, applied-device receipt, full adapter coverage, or conflict inbox exists yet. Evidence: `docs/engineering/C10-04C1-BACKED-UP-RECORD-APPLY.md`.

- **C10-04C2 — backed-up task adapter:** extends C1 with exact task application, existing-project/contact validation, safe local dependency ordering, and refusal of linked-task deletion or cross-project reassignment. Six new focused scenarios preserve local edits, backups, references and cursor state. Other adapters, conflict resolution and runtime activation remain pending. Evidence: `docs/engineering/C10-04C2-TASK-ADAPTER.md`.

- **C10-04C3 — project adapter/local mapping:** adds backed-up project application with explicit portable-root projection under the existing device-local root, project/task dependency ordering, reference-preserving deletion and collision/reparse-point/path reassignment refusal. No document operation or customer activation is added. Remaining adapters and other C10-04C gates stay pending. Evidence: `docs/engineering/C10-04C3-PROJECT-ADAPTER.md`.

- **C10-04C4 — RFI/submittal adapters:** adds exact allowlisted register metadata application, lifecycle validation, project/link/parent checks, cycle refusal and cascade-preserving tombstone guards. Eight focused scenarios extend safe-apply coverage to 28 tests. New relationship/attachment/file envelopes, other adapters and runtime activation remain pending. Evidence: `docs/engineering/C10-04C4-RFI-SUBMITTAL-ADAPTERS.md`.

- **C10-04C5 — task-link adapters:** adds RFI-task/submittal-task envelopes, schema-13 immutable local identity bindings, dependency-ordered linking/explicit unlinking, restart/retry and conflict/cross-project refusal. Seven new scenarios bring safe-apply coverage to 35 tests. Sync remains disabled; attachment/file adapters are next. Evidence: `docs/engineering/C10-04C5-TASK-LINK-ADAPTERS.md`.

No commercial work package authorizes production provider accounts, secrets, deployment, or telemetry for a later package. Resolve and record its required founder decisions before implementation.

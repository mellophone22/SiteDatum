# SiteDatum

SiteDatum is a local-first Windows desktop workspace for project engineers. It brings tasks, RFIs, submittals, project controls, file references, notes, contacts, planning, and recovery into one application while keeping real project documents as ordinary Windows files.

SiteDatum is built with Tauri 2, React 19, TypeScript, Rust, and bundled SQLite. The project workspace works offline, and Free use does not require an account.

## Current release

**SiteDatum 1.4.3** is the current controlled Windows Early Access release, published October 6, 2026.

- Channel: unsigned Windows Early Access
- Installer: `SiteDatum_1.4.3_x64-setup.exe`
- Size: 4,684,165 bytes
- SHA-256: `CB9853455DC77D397E86E08CD0BE2EF80370148202BF813F78271BBE6FB3E54B`
- Source commit: [`2344f184cd8c4ff6d3fa7e24cbd0a29b5668119c`](https://github.com/mellophone22/SiteDatum/commit/2344f184cd8c4ff6d3fa7e24cbd0a29b5668119c)
- Download and installation guide: [sitedatum.site/early-access.html](https://sitedatum.site/early-access.html)
- Release notes: [sitedatum.site/release-notes.html](https://sitedatum.site/release-notes.html)

The installer is not Authenticode-signed. Windows SmartScreen, Smart App Control, antivirus software, or organizational policy may warn or block it. Do not weaken device security controls to install SiteDatum. Verify the SHA-256 digest before running the installer. Automatic application updates are not enabled for this channel.

This is a monitored soft launch, not a signed general-availability release. Broad promotion remains gated on successful observation of the first genuine production customer transaction. The application, installer, production licensing boundary, and public website have completed the acceptance documented in [`docs/engineering/release-candidates/SiteDatum-1.4.3-publication-plan.md`](docs/engineering/release-candidates/SiteDatum-1.4.3-publication-plan.md).

## Product model

SiteDatum is single-user and local-first:

- SQLite is the source of truth for application records and relationships.
- Project documents remain user-owned files under a selected local or UNC Windows project root.
- Ordinary project work does not depend on a network service.
- Free starts without registration and supports up to three active projects.
- Pro supports unlimited active projects and workflow-acceleration features for $15 monthly or $150 annually.
- Subscription expiration is non-destructive: existing projects and records remain visible and editable, and backup and essential CSV export remain available.
- Licensing and billing systems never receive project names, tasks, RFIs, submittals, notes, contacts, file paths, documents, or database contents.

Optional Supabase metadata synchronization exists as a separately consented legacy capability, but it is not the licensing authority and is deferred from the initial commercial launch. Project-file replication, when used, remains the responsibility of a dedicated OneDrive project root; the SQLite database must stay in application-local storage.

## Implemented functionality

### Daily workspace

- Home overview across all active projects or one selected project
- Deterministic Attention queues for overdue, due-today, upcoming, waiting, and follow-up work
- RFI response and submittal disposition queues
- Global search, command palette, recent-project navigation, and exact-record routing
- Quick capture for tasks, notes, draft RFIs, and contacts
- Saved task views, filters, sorting, inline status changes, and completion summaries
- Light, Dark, and Windows-default appearance modes

### Projects and tasks

- Project creation, editing, pinning, archiving, restoration, and phase-aware workspace navigation
- Previewed creation of a standard project folder tree beneath the configured project root
- Separate project metadata and physical-folder identity—editing a project does not silently rename or move its folder
- Task priority, status, category, due date, waiting state, follow-up date, description, and related-record context
- Reusable project templates for task checklists and milestones
- Transactional bulk status updates

### RFIs and submittals

- Validated RFI and submittal lifecycle transitions with project-scoped number suggestions
- Dates, recipients, responsibility, responses, dispositions, relationships, and attachment references
- Locally generated one-page RFI PDFs using the neutral SiteDatum layout
- Optional customer-owned PDF templates stored as local references rather than copied into the database
- Collision-safe export that never silently overwrites an existing file

### Files, drawings, notes, contacts, and activity

- Explicit Copy, Move, or Register workflows for project files
- Registered-file and drawing metadata, including missing-file detection and recovery
- Safe Show in Explorer commands owned by the Rust application boundary; SiteDatum does not directly open registered documents
- Project and workspace notes, a shared contact directory, and searchable activity history
- Removing a reference does not delete the physical file

### Project Controls

The Project Controls workspace includes constrained registers for:

- Meeting minutes
- Procurement items
- Change events
- Transmittals
- Milestones
- Punch-list items
- Daily reports
- Startup checks
- Commissioning checks
- Commissioning issues

These registers support project context, search and filtering, validated status and priority values, responsibility, dates, domain-specific fields, archive state, and checklist progress where applicable.

### Planning, interchange, reports, and recovery

- Calendar-style register combining task dates, follow-ups, RFI/submittal dates, and milestones
- Opt-in Windows reminders while SiteDatum is running
- Full-batch CSV and Excel import preview and validation before any rows are committed
- Filtered CSV and native Excel exports without overwriting existing files
- Versioned printable HTML reports for weekly status, open items, meeting minutes, transmittals, submittal covers, and contacts
- Local SQLite backups, integrity preview, record counts, confirmed restore, and pre-restore safety backup
- Pre-migration snapshots, forward migration, and refusal to write through a newer unsupported schema
- Recovery views for missing files, sync conflicts, backup inventory, and activity

## Free and Pro boundary

Free includes the complete manual workflow for up to three active projects, including manual project-control registers, local backup and restore, normal file access, global search, and complete machine-readable CSV export.

Pro adds:

- Unlimited active projects
- Project templates
- Bulk operations
- CSV/Excel batch import
- Native Excel export
- Formatted reports and generated professional deliverables
- Two active Windows computers
- Up to 21 days of offline verification grace when a previously verified entitlement cannot be refreshed

The desktop enforces consequential limits at Rust mutation boundaries rather than relying only on disabled controls. Billing uses hosted Stripe Managed Payments surfaces behind a dedicated licensing service. Merchant credentials, webhook secrets, service-role credentials, and entitlement signing private keys are never shipped in the desktop client or repository.

See [`docs/commercial/ENTITLEMENT_MATRIX.md`](docs/commercial/ENTITLEMENT_MATRIX.md) for the complete policy.

## Safety and architecture

- React communicates through typed application-level Tauri commands; it does not access SQLite or the filesystem directly.
- Rust owns persistence, filesystem operations, imports, exports, reports, recovery, and entitlement enforcement.
- SQLite foreign keys are enabled on every connection, and ordered migrations are tracked in `schema_migrations`.
- Database mutations use transactions. Filesystem operations use explicit recoverable workflows because SQLite and NTFS cannot share a transaction.
- Local paths and UNC roots are validated and canonicalized; project files are never executed.
- Structured errors include a stable code, user-facing message, recovery guidance, local technical detail, and correlation ID.
- The Tauri window uses a restrictive content security policy.
- Customer project content is excluded from licensing, billing, and operational service payloads.

Architecture decisions and implementation evidence are under [`docs/engineering`](docs/engineering).

## Install the Early Access release

1. Read the disclosures and download SiteDatum 1.4.3 from the [Early Access page](https://sitedatum.site/early-access.html).
2. Verify the installer in PowerShell:

   ```powershell
   Get-FileHash .\SiteDatum_1.4.3_x64-setup.exe -Algorithm SHA256
   ```

3. Confirm the result is:

   ```text
   CB9853455DC77D397E86E08CD0BE2EF80370148202BF813F78271BBE6FB3E54B
   ```

4. Run the installer only if your device policy permits unsigned applications.

Upgrades preserve the application-local workspace, settings, backups, project-root configuration, and secure account state. Real project files remain in their existing Windows folders.

## Development

### Windows prerequisites

- Windows 10 or 11 with WebView2
- Node.js 24 and npm 11
- Rust 1.97 with the `x86_64-pc-windows-msvc` target and `rustfmt`
- Visual Studio 2022 Build Tools with **Desktop development with C++**

### Run locally

```powershell
git clone https://github.com/mellophone22/SiteDatum.git
cd SiteDatum
npm ci
npm run tauri -- dev
```

Frontend-only development is available with `npm run dev`. The browser preview exercises the React interface but not native SQLite, filesystem, credential-store, notification, or window behavior.

### Quality gates

```powershell
npm ci --prefer-offline --no-audit
npm audit --audit-level=high
npm run lint
npm test -- --run
npm run test:site
npm run build

cd src-tauri
cargo fmt --all -- --check
cargo test --locked
```

The GitHub Actions workflow also audits Rust dependencies and runs the licensing database tests against a disposable local Supabase stack. The Supabase suite requires Docker:

```powershell
npm run supabase:start
npm run test:licensing-db
npm run supabase:stop
```

Build an ordinary local Windows NSIS bundle with:

```powershell
npm run tauri -- build --bundles nsis
```

The production commercial build is fail-closed and requires release-only public configuration supplied outside the repository. It must not be substituted with invented values or used to expose server credentials.

## Verification status

For the current repository checkout, the reproducible frontend gates pass with:

- 28 Vitest files and 88 tests passed
- ESLint passed
- TypeScript and Vite production build passed
- Eight-page commercial website validation passed
- npm high-severity audit reported zero vulnerabilities
- Rust formatting passed

The immutable 1.4.3 release candidate separately records 51 Rust tests with the production feature, release-manifest validation, a complete C8 audit with 17 passes and no failures, focused disposable-Windows-profile install/reinstall acceptance, and public-origin artifact verification. See the [1.4.3 publication plan](docs/engineering/release-candidates/SiteDatum-1.4.3-publication-plan.md) for the exact evidence boundary.

## Data locations

SiteDatum stores its SQLite database, structured logs, settings, and backups under the Windows application-local data directory assigned to `com.cabre.project-engineer-workspace`. The legacy identifier is intentionally retained so upgrades preserve existing data across the SiteDatum rebrand.

Project documents remain normal Windows files beneath the Projects root selected in Settings. Do not place the SQLite database inside OneDrive or another file-synchronization folder.

## Scope

SiteDatum does not provide AI/LLM features, collaboration, organizations, role-based access, web/mobile clients, OCR or content indexing, PDF/CAD markup, engineering calculations, time tracking, or a plugin marketplace.

The authoritative product scope is documented in [`MASTER_PLAN.md`](MASTER_PLAN.md), [`docs/MVP_SCOPE.md`](docs/MVP_SCOPE.md), and [`docs/PRODUCT_SPEC.md`](docs/PRODUCT_SPEC.md).

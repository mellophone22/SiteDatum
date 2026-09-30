# SiteDatum

SiteDatum is a local-first Windows desktop workspace for project engineers. It keeps project controls, day-to-day follow-up, document references, and operational reporting in one fast desktop application while leaving real project documents as ordinary Windows files.

Built with Tauri, React, TypeScript, Rust, and bundled SQLite, SiteDatum works offline and does not require a cloud account. Optional Supabase metadata synchronization and a dedicated OneDrive project root can be enabled when work needs to follow the user between trusted workstations.

## Current status

The latest packaged Windows release is **SiteDatum 1.4.0**. It includes the complete local project-engineering workspace described below, Light/Dark/Windows-default appearance modes, and corrected Windows project-root handling.

The current development branch also contains the first two commercial-foundation packages: a provider-neutral Free/Pro entitlement policy and tested Rust enforcement boundaries. SiteDatum remains in explicit pre-commercial compatibility mode, so the packaged application does not currently require an account, accept payments, activate subscriptions, or restrict existing controls. The planned Free/Pro launch work is being added incrementally without moving project records or documents out of the local workspace.

## What SiteDatum can do

### Daily command center

- Shows a project overview with actionable counts, recent activity, upcoming work, and compact completion gauges.
- Surfaces overdue tasks, due-soon items, pending follow-ups, aging RFIs, and submittal items needing attention.
- Provides global search, recent-project navigation, a command palette, keyboard shortcuts, and project-context preservation.
- Captures tasks, notes, draft RFIs, and contacts without leaving the current workspace.
- Saves reusable task views with filters for status, due date, ownership, priority, and archive state.

### Projects and tasks

- Creates project records with constrained status and phase values, including custom phases.
- Creates and validates a standard project folder tree beneath a configured local or UNC project root.
- Supports pinning, archiving, recovery views, safe folder opening, and separate project status versus archive state.
- Tracks task status, priority, assignee, due date, waiting state, follow-up date, completion, and archive state.
- Provides reusable project templates containing task checklists and milestones.
- Applies bulk status updates to selected operational records through one transactional Rust command.

### RFIs and submittals

- Tracks RFI lifecycle, dates, responsibility, questions, answers, relationships, and attachments.
- Generates a PDF RFI from the approved Excel-derived company template without overwriting an existing file.
- Tracks submittal packages, revisions, review status, dispositions, dates, relationships, and attachments.
- Integrates RFI and submittal attention items into the daily workspace.

### Drawings, files, notes, contacts, and history

- Registers existing files or explicitly copies and moves them into a project; collisions require a user decision.
- Tracks drawing metadata and supports missing-file location and recovery.
- Opens registered files and their containing folders through constrained Rust-owned commands.
- Stores project notes and contacts and maintains a searchable activity history.

### Project controls and field operations

The Project Controls workspace provides dedicated registers for:

- Meeting minutes and action ownership
- Procurement items
- Change events
- Transmittals
- Milestones
- Punch-list items
- Daily reports
- Startup checks
- Commissioning checks
- Commissioning issues

Each register supports search, project filtering, constrained status and priority values, editing, dates, responsible parties, domain-specific fields, archive state, and applicable checklist progress.

### Planning, reminders, import, export, and reports

- Combines task due dates, follow-ups, RFI/submittal dates, and operational milestones in a restrained calendar-style register.
- Offers opt-in Windows reminders for due and overdue work while SiteDatum is running.
- Previews CSV or Excel imports, validates every row, and commits the batch only when the full import is valid.
- Exports filtered operational registers to CSV or native Excel workbooks.
- Generates versioned printable HTML reports for weekly status, open items, meeting minutes, transmittals, submittal covers, and contact lists.

### Backup, recovery, and optional sync

- Stores the database, logs, backups, and application state in Windows application-local data rather than the Projects root.
- Creates local SQLite backups and provides backup inventory, integrity preview, record counts, and confirmed restore.
- Shows missing-file diagnostics, current sync conflicts, and searchable activity in the Recovery workspace.
- Supports optional Supabase metadata synchronization with explicit conflict resolution.
- Supports a dedicated OneDrive project root for normal project documents; the rest of an existing OneDrive is not scanned or managed.
- Stores cloud credentials through Windows Credential Manager rather than in the repository or SQLite database.

## Safety and architecture

- SQLite and filesystem operations are Rust-owned.
- React communicates only through typed application-level Tauri commands.
- SQLite foreign keys are enabled on every connection and repository-managed migrations are tracked in `schema_migrations`.
- Database-only changes use transactions.
- Filesystem changes use explicit, recoverable workflows because SQLite and NTFS do not share a transaction.
- Local Windows paths and UNC project roots are supported with canonicalization and reparse-point checks.
- Destructive or overwriting file operations are never implicit.
- Structured errors contain a stable code, user-facing message, recovery guidance, local technical details, and correlation ID.

## Install

SiteDatum is now hosted on [GitLab](https://gitlab.com/Kikito22/sitedatum). The current GitLab project is private, so you must be signed in with project access before downloading a release artifact.

### Recommended: Windows installer

1. Download [`SiteDatum_1.4.0_x64-setup.exe`](https://gitlab.com/Kikito22/sitedatum/-/raw/main/releases/1.4.0/SiteDatum_1.4.0_x64-setup.exe).
2. Optionally verify the download in PowerShell:

   ```powershell
   Get-FileHash .\SiteDatum_1.4.0_x64-setup.exe -Algorithm SHA256
   ```

   Expected SHA-256:

   ```text
   913ABE895868D04BE3E7F7F1F27F464BF0DD4F3AD7C82A144F982D62C233D1F2
   ```

3. Run the installer. Existing SiteDatum installations keep their local database, settings, backups, project-root configuration, and optional synchronization session.

### Portable executable

For a no-installer copy, download [`SiteDatum.exe`](https://gitlab.com/Kikito22/sitedatum/-/raw/main/releases/1.4.0/SiteDatum.exe). Its expected SHA-256 is:

```text
F7985DB5AAB5ED885625F4991B70C994F697207A3316B047A6BA47ACE14249E2
```

Both files, release notes, and the checksum manifest are available in the [`releases/1.4.0`](https://gitlab.com/Kikito22/sitedatum/-/tree/main/releases/1.4.0) directory.

The 1.4.0 installer and executable are not code-signed, so Windows SmartScreen may show an unrecognized-publisher warning. Verify the SHA-256 value before running a downloaded file. Windows code signing and signed application updates remain required before the public paid launch.

## Continue development on another workstation

Prerequisites:

- Windows 10 or 11 with WebView2
- Node.js 24 and npm 11
- Rust stable with the `x86_64-pc-windows-msvc` target
- Visual Studio 2022 Build Tools with the Desktop development with C++ workload

```powershell
git clone https://gitlab.com/Kikito22/sitedatum.git
cd SiteDatum
npm ci
npm run tauri -- dev
```

## Quality gates

```powershell
npm run lint
npm test -- --run
npm run build
cd src-tauri
cargo fmt --all -- --check
cargo test
```

Build the Windows installer with:

```powershell
npm run tauri -- build --bundles nsis
```

## Data locations

SiteDatum stores its SQLite database, structured logs, settings, and backups under the Windows application-local data directory assigned to `com.cabre.project-engineer-workspace`. This legacy identifier is intentionally retained so upgrades preserve existing local data and settings. Real project documents remain normal Windows files beneath the Projects root selected in Settings.

For optional multi-workstation use, configure a dedicated OneDrive folder as the Projects root and enable Supabase metadata sync in Settings. Keep the database itself in application-local data; do not place the SQLite database inside OneDrive.

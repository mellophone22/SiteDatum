# WP9 — Optional Supabase and OneDrive Sync

WP9 adds opt-in, single-user synchronization between Windows workstations. OneDrive remains responsible for normal project files beneath a dedicated project root. AnyDesk synchronizes its implemented metadata through Supabase and never places the live SQLite database in OneDrive.

## Delivered behavior

- Email/password authentication against the AnyDesk Workspace Supabase project using a manually provisioned, confirmed user.
- Access and refresh tokens stored in Windows Credential Manager rather than SQLite, browser storage, or source-controlled configuration.
- Manual **Sync now** action; local operation remains available without a network connection.
- A complete metadata snapshot covering projects, tasks, RFIs, submittals, relationships, attachment references, registered files, notes, contacts, and activity.
- Project-root-relative path serialization so two computers may have different absolute OneDrive paths. References outside the selected root become visible missing references on another computer instead of exposing or guessing another device's path.
- Server-side RLS and atomic version comparison. Concurrent offline edits create a durable local conflict and require **Keep this computer** or **Use cloud version**.
- Automatic app-local SQLite safety backup before every sync and conflict resolution. Sync never copies, overwrites, moves, or deletes OneDrive file bytes.

## Operational notes

Both computers must select their local copy of the same dedicated OneDrive project folder before syncing. On first connection, an empty local workspace downloads the cloud workspace. If both the cloud and local workspace already contain different metadata, AnyDesk requires an explicit conflict choice.

The Supabase Free plan may pause after inactivity and does not provide downloadable managed backups. AnyDesk's local backups remain part of the recovery model.

## Verification

The Rust suite covers a full metadata export/import between two different local project roots and verifies path rebasing. The existing persistence and domain suite remains green. Frontend build, lint, Vitest, native development launch, Supabase security advisor, and NSIS production packaging pass. The owner creates and confirms one application user in Supabase Authentication → Users, then uses the same email and password on each workstation.

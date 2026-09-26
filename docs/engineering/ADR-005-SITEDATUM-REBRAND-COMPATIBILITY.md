# ADR-005: SiteDatum rebrand and upgrade compatibility

## Status

Accepted for release 1.2.0.

## Context

The product is being renamed from AnyDesk to SiteDatum. Visible application branding, executable names, installer names, window titles, notifications, report footers, documentation, and repository presentation must use the new name. Existing installations already store the user's SQLite workspace, settings, backups, and secure cloud session under identifiers established by earlier releases.

Changing those identifiers during a visual rebrand would make an upgrade appear to lose data or disconnect cloud synchronization.

## Decision

- Use **SiteDatum** for every current user-facing product surface.
- Release the rebrand as version **1.2.0** with `SiteDatum.exe` and `SiteDatum_1.2.0_x64-setup.exe`.
- Retain the Tauri application identifier `com.cabre.project-engineer-workspace` so Windows resolves the same application-local data directory.
- Retain the existing internal Rust crate names, secure credential service key, sync marker names, and temporary-file prefixes. These are implementation and persistence identifiers, not product branding.
- Preserve historical release notes and verification records under their original product name.

## Consequences

Existing users can install SiteDatum over the prior release and continue using the same local workspace and saved cloud session. Some internal identifiers continue to contain the former name until a future explicit data migration is designed, tested, and made reversible.

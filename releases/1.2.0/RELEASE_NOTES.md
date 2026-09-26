# SiteDatum 1.2.0

SiteDatum 1.2.0 completes the product rebrand and introduces the new application icon and in-product wordmark.

## Highlights

- Renames the Windows application, title bar, notifications, reports, installer, and documentation to SiteDatum.
- Adds the new SiteDatum application icon across Windows package sizes and the in-product SiteDatum wordmark.
- Preserves existing local databases, settings, backups, sync markers, and Windows Credential Manager sessions during upgrade.
- Retains all project-centric workspace capabilities delivered in version 1.1.0.

## Install

Download and run `SiteDatum_1.2.0_x64-setup.exe`. The installer is not code-signed, so Windows SmartScreen may show an unrecognized-publisher warning.

Existing users can upgrade without relocating project data. SiteDatum intentionally keeps the previous application data identity for continuity.

## Verification

The release is gated by frontend linting, TypeScript/Vite production compilation, Vitest, Rust formatting, Rust tests, a Tauri production build, an NSIS package build, checksum generation, and startup smoke testing.

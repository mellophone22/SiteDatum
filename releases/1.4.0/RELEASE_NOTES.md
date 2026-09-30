# SiteDatum 1.4.0

SiteDatum 1.4.0 adds complete appearance controls and repairs Windows project-root paths returned by native folder dialogs.

## Changes

- Adds Light, Dark, and Windows-default appearance modes.
- Adds a compact header switch for immediate Light/Dark changes and a complete three-choice Appearance section in Settings.
- Follows Windows theme changes live when Windows default is selected and persists the device-local preference across launches.
- Applies a purpose-built dark palette to every workspace, register, form, panel, dialog, and onboarding screen.
- Normalizes Windows verbatim drive paths such as `\\?\C:\Projects` before validation, display, and persistence.
- Restores standard UNC syntax from extended paths such as `\\?\UNC\server\share`.
- Fixes Browse → Check location → Save project root so a valid native-dialog selection cannot reject its own canonical path.

## Install

Run `SiteDatum_1.4.0_x64-setup.exe`. The installer is not code-signed, so Windows SmartScreen may show an unrecognized-publisher warning.

The application identifier is unchanged. Existing SiteDatum installations retain their local workspace database, settings, backups, project-root configuration, and optional synchronization session during the upgrade.

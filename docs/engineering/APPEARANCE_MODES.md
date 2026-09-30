# Appearance modes

## Decision

Appearance is a device-local presentation preference. `ThemeProvider` owns the selected policy (`light`, `dark`, or `system`) and resolves the effective theme against `prefers-color-scheme`. The resolved value is applied to `document.documentElement` as `data-theme` plus the native `color-scheme` property. It is initialized before React mounts to prevent a mismatched first frame.

No Rust command, database field, cloud-sync record, filesystem operation, network dependency, or new package is involved. The local storage key is `appearance.mode`; missing or malformed values safely fall back to `system`.

## Interaction

- The app header exposes one accessible switch for fast movement between the effective Light and Dark themes.
- If the current policy is Windows default, using the header switch creates an explicit preference for the opposite effective theme.
- Settings exposes three radio-style, text-labeled choices: Light, Dark, and Windows default.
- Windows default responds live to operating-system changes and persists across restart.
- The choice is communicated through text, `role="switch"` / `role="radio"`, and `aria-checked`, not color alone.

## Verification

Pure tests cover malformed preference recovery, all stored policies, and system resolution. The populated visual runner verifies the header switch, persisted storage, Settings selection, live Windows Light/Dark adoption, reload persistence, reduced motion, no page overflow, and no error notices. Its dark sweep covers Home, Attention, Projects, Tasks, RFIs, Submittals, Files, Project Controls, Notes & Contacts, Recovery, About, and Settings with 25 fixture projects and 240 fixture tasks. The default 960x680 and minimum 760x540 application windows remain within the viewport.

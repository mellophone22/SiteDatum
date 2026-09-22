# WP6 — Notes, Contacts, and Activity

WP6 adds local timestamped notes (optionally tied to a project), reusable contacts with company, role, email, and phone, and a compact chronological Activity view backed by the existing activity event log. The screen is intentionally a dense context register rather than a dashboard.

Notes and contacts remain SQLite-owned through typed Tauri commands. Validation requires non-empty note text and contact names. Existing project, task, RFI, submittal, and file events are shown without fabricating activity.

## Verification

Rust tests, TypeScript build/typecheck, and ESLint pass. WP7 remains unstarted.

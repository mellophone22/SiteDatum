# WP13-WP18 Operations Expansion

Completed: 2026-09-22

## Delivered boundaries

- WP13 adds explicit transactional bulk status changes and reusable task/milestone project templates. The existing standard folder-tree creation remains the only automatic filesystem setup.
- WP14 adds previewed all-or-nothing CSV/Excel import, filtered CSV/Excel export, and printable versioned HTML report generation. Generated files require an explicit destination and never overwrite.
- WP15 adds a combined date register and opt-in Windows reminders for due and overdue work while the application is running. AnyDesk does not install a background scheduler.
- WP16 adds searchable audit activity, missing-file recovery routing, current cloud-conflict visibility, backup inventory, integrity/count preview, and confirmed restore with a safety backup.
- WP17 adds meeting minutes/action ownership, procurement, change-event, transmittal, and milestone registers.
- WP18 adds punch-list, daily-report, startup-check, commissioning-check, and commissioning-issue registers.

## Architecture

Operational persistence and validation remain Rust-owned behind typed Tauri commands. One constrained `work_items` table shares common register behavior without exposing arbitrary SQL. Machine-readable type, status, and priority values are protected at the application and database layers. The UI renders specialized fields only when they apply to the selected register.

Spreadsheet parsing and generation occur locally. Imports validate every row before a transaction starts. Reports are local HTML documents intended for browser print-to-PDF, preserving a minimal dependency surface and a visible template version.

Backup restore uses SQLite's backup API, accepts only canonical app-local backup files, reapplies repository migrations, and restores foreign-key enforcement before returning control to the UI.

## Intentional limitations

- Reminders are evaluated while AnyDesk is open; background Windows scheduling is outside the MVP boundary.
- Startup and commissioning checklists track aggregate completed/total counts plus notes rather than nested checklist-step records.
- The date register shows the existing submitted date for submittals because a separate submittal due-date field is not currently part of the data model.
- Recovery shows the current unresolved sync-conflict state. A durable historical conflict ledger is deferred until a product requirement defines retention and privacy expectations.

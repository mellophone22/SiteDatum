# WP-08 — Projects List Cleanup

The Projects register now treats project identity as its unambiguous primary action. Selecting an active project number/name opens that workspace immediately; archived projects open their overview instead. Double-clicking a non-control area of an active row follows the same behavior.

## Register behavior

- Project number and name are presented together as the leading identity.
- Status, phase, customer, and target date remain directly scannable.
- The misleading Folder column was removed; folder access is an explicit action.
- Pinned and archived state are communicated with text rather than icon-only treatment.
- View details, Edit project, Open folder, Pin/Unpin, and Archive/Restore live in a keyboard-accessible More menu.
- Archiving an active project still requires confirmation, and no file or folder is moved or deleted.
- Opening a project through the primary action still updates recent-project ordering.

No schema, migration, or backend command change was required. The browser-only preview can verify the empty register and responsive shell; populated native register verification remains part of release hardening because project rows depend on Tauri commands.

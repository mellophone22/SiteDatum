# WP17 — Feedback and Dialog Standardization

## Shared primitives

`Feedback.tsx` provides reusable status, empty, loading, and confirmation components. The confirmation hook supplies consistent asynchronous decisions without coupling record screens to a global state store.

The confirmation dialog:

- uses alert-dialog semantics with an accessible title;
- moves initial focus to Cancel and restores the initiating control;
- traps Tab focus and supports Escape;
- uses explicit action labels and text, not color alone;
- supports a destructive action treatment without making Cancel ambiguous.

## Migrated consequences

All former frontend `window.confirm` calls now use the shared dialog: project archive, file Move, file-reference removal, RFI/submittal attachment-reference removal, note/contact removal, cloud disconnect/conflict resolution, and backup restore.

Filesystem language remains explicit. Removing a reference never claims to delete a physical file. File Move shows both paths and states that the original leaves its current location. Restore retains the pre-restore safety-backup guarantee.

## Feedback states

Shared status notices are used by the shell, Files, and Recovery. Shared loading and empty states establish the same semantic and visual contract for incremental adoption across registers. The loading treatment respects reduced-motion preferences.

# WP12 — Workflow Acceleration and Overview

WP12 reduces navigation and repeated filter setup without changing domain lifecycles.

## Delivered behavior

- Global Quick capture opens from the header or `Ctrl+Shift+N` and creates real Tasks, Notes, RFI drafts, or Contacts through the existing typed Tauri commands.
- Quick capture inherits the selected project context. Task and RFI creation require a project; Notes may be workspace-wide; Contacts remain reusable.
- Task filters and sort order can be named, restored, and explicitly deleted as local saved views. Deleting a view never deletes task data.
- Overview aggregates existing project, task, RFI, submittal, and note records for the current project context. Summary values route to their source workspaces and do not introduce a second source of truth.

## Boundary

Saved task views are device-local UI preferences in this package. Cross-device view synchronization will only be added through an explicit, versioned sync-schema change. Quick capture does not bypass validation and cannot create opened or submitted RFIs.

## Verification

Frontend tests cover saved-view serialization and malformed local state recovery. TypeScript build, ESLint, Vitest, Tauri development launch, keyboard/focus review, visual review, Rust tests, and Windows packaging are release gates.

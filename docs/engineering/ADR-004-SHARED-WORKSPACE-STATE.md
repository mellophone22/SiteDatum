# ADR-004 — Shared Workspace State

**Status:** Accepted — 2026-09-25

## Context

The frontend previously coordinated the active screen, current project, exact-record focus, restore refreshes, and project-context changes through a mixture of `App.tsx` state, direct `localStorage` reads and writes, one-shot focus records, and `workspace:*` browser events. This made normal navigation depend on indirect communication and allowed a removed or archived persisted project ID to remain active after launch.

UX 1.1 requires a project-centric shell, reliable project inheritance, direct search-to-record navigation, and immediate project-list refresh after creation. These behaviors need one lightweight source of truth before the shell and individual registers are reorganized.

## Decision

1. A React Context and reducer own frontend workspace state: current screen, current project ID, focused record, navigation revision, and project-list revision.
2. The reducer and persisted-state parser remain pure and receive direct unit coverage, including invalid screens, stale projects, record opening, and independent refresh revisions.
3. Only stable user context is persisted: current screen and current project ID. Focused records and revision counters remain session state.
4. Loaded project IDs reconcile the persisted current project. A missing project clears the context to All Projects instead of leaving an invalid selection active.
5. Search record selection updates project context, navigation, focus, and the destination refresh as one reducer action.
6. During incremental migration, the provider writes the existing `workspace.focus` compatibility record and emits the existing `workspace:project` event. These bridges remain only for consumers not yet migrated to the context.
7. Existing filter and saved-view preferences remain screen-owned. Shared workspace state does not absorb unrelated UI preferences.
8. Redux or another state dependency is not introduced.

## Consequences

- `App.tsx` and `useProjectContext` now read current screen/project state from one source.
- Project-list and whole-workspace refreshes are explicit operations rather than unrelated local counter updates.
- Search results can establish their project before opening the destination record.
- Later work packages can migrate creation flows and focus consumers without changing the state model again.
- Compatibility storage/events temporarily duplicate outward notifications, so they must not be treated as independent sources of truth and should be removed after all consumers use the shared context.
- No database, migration, filesystem, cloud schema, or Rust command change is required.

## Verification

- Workspace reducer/parser tests cover valid and invalid persisted screens, record opening, stale-project reconciliation, and refresh revisions.
- Frontend lint passes.
- All frontend tests pass (5 files, 12 tests).
- TypeScript and the Vite production build pass.

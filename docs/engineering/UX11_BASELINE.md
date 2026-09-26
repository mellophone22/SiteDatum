# UX 1.1 Baseline and Repository Audit

Date: 2026-09-25

## Repository baseline

- Branch created for the work: `codex/ux-1.1-project-centric-workspace`
- Baseline commit: `bcaceee6afb0c6fe310c7d3717705e8fb7b0f087`
- `origin/main` was fetched before the audit and matched the baseline commit (`0` ahead, `0` behind).
- The working tree was clean before the audit.
- Stack: Tauri 2, React 19, TypeScript 6, Rust, and bundled SQLite through `rusqlite`.
- Database migrations are forward-ordered from `0001_foundation.sql` through `0010_operations.sql` and recorded by the Rust persistence layer.

## Baseline quality gates

| Gate | Result | Notes |
| --- | --- | --- |
| `npm ci` | Pass | Installed 161 packages; npm reported the configured ESLint version as deprecated but found no vulnerabilities. The first sandboxed attempt could not write the user npm cache; the approved retry completed. |
| `npm run lint` | Pass | Zero errors. |
| `npm test -- --run` | Pass | 4 test files, 8 tests. |
| `npm run build` | Pass | TypeScript and Vite production build completed. |
| `cargo fmt --all -- --check` | Pass | Cargo emitted the existing non-fatal path canonicalization warning for `C:\Users\fcabrera`. |
| `cargo test` | Pass | 27 Rust tests passed. The first restricted run could not reach crates.io; after dependencies were fetched with approved network access, compilation and all tests passed. |

No product behavior was changed during WP-00.

## Current architecture findings

### Frontend navigation and workspace state

- `src/App.tsx` owns the active screen and the selected project ID with component state and persists both values directly to `localStorage`.
- Project context is duplicated by `src/projectContext.ts`, which reads the same storage key and listens for a `workspace:project` browser event.
- Global navigation is a local `setScreen` callback. The current shell presents most registers as peer destinations rather than separating global and selected-project navigation.
- Exact-record navigation writes a one-shot `workspace.focus` object to `localStorage`; destination screens consume it with `takeFocus()` after loading their complete register.
- Workspace refresh after restore uses a `workspace:restored` custom event and a local navigation revision.
- `Ctrl+N` dispatches a generic `workspace:new` event. Projects, Tasks, RFIs, Submittals, Files, and Notes/Contacts independently subscribe and decide what creation means.
- `Ctrl+Shift+N` opens Quick Capture and `Ctrl+K` opens Search. These shortcuts and their existing behavior are compatibility requirements for later packages.

This confirms the need for WP-01 to introduce a typed shared workspace state while retaining the old event/storage paths until consumers are migrated and tested.

### Project context and creation entry points

- The application already has project filtering and a lightweight `useProjectContext` hook, but individual create forms commonly initialize with an empty project and load their own project lists.
- Quick Capture has its own project defaulting behavior. Screen-level create buttons and the generic `workspace:new` path are separate entry points.
- Project creation is implemented in `Projects.tsx`; after creation it reloads that screen's local list, but there is no centralized project-list revision for all consumers.
- The current Project form exposes both minimum and secondary metadata in one form. Folder preview and Rust-owned folder creation are already implemented and must remain intact.

### Detail and register patterns

- RFIs, Submittals, and Files already use list/detail layouts, but each implements selection, close behavior, focus, forms, and accessibility independently.
- Tasks provide dense list/Attention behavior and inline status handling, but do not yet use the same full record-detail pattern.
- Filters and saved views are stored locally by individual screens. Shared navigation and drawers must not reset these screen-owned preferences.
- Several large components are highly compressed, which increases regression risk when migrating cross-cutting state. Changes should remain small and package-scoped.

### Dialogs, feedback, and accessibility

- Native `confirm()` is used for cloud disconnect/conflict resolution, backup restore, file Move, reference removal, note/contact deletion, project archive, and attachment-reference removal.
- Existing forms generally use visible labels; status and error messages often use `role="status"` or `role="alert"`, and active shell navigation already applies `aria-current`.
- Existing list/detail selection commonly uses both a selected-row style and `aria-selected`, but focus movement/restoration is not centralized.
- Quick Capture and Search implement Escape behavior; later work must verify full focus trapping and restoration rather than assuming it from keyboard close support.

### Backend, persistence, and safety

- React invokes typed application commands; Rust owns SQLite, filesystem operations, backups, recovery, sync, reports, and imports/exports.
- The Tauri command boundary is registered centrally in `src-tauri/src/lib.rs`; domain validation is split across project, task, RFI, submittal, file, work-item, sync, and recovery modules.
- SQLite foreign keys and ordered migrations are managed by `persistence.rs`. Historical migrations must remain unchanged.
- Existing tests cover explicit Register/Copy/Move behavior, collision refusal, project folder creation, lifecycle validation, backup/restore, cloud snapshot rebasing, and realistic task volume.
- No schema change is currently required for WP-01 through WP-03. Any later relationship or sync-field change must add a forward migration and update snapshot serialization.

## Proposed work-package boundaries

1. **WP-01 — shared workspace state:** add a typed reducer/context for screen, current project, focused record, and project-list revision; add reducer/storage tests; adapt `App.tsx` and compatibility bridges without redesigning screens.
2. **WP-02 — project-centric shell:** separate Global, Current Project, and System navigation; make All Projects versus selected-project context explicit; retain every existing destination.
3. **WP-03 — project opening and creation:** add Open Workspace, immediately refresh shared project state after creation, navigate into Overview, and progressively disclose secondary project fields while preserving folder preview and safety.
4. **WP-04/WP-05 — Home and inherited context:** consolidate Overview/Attention presentation using existing commands, then migrate each creation path to the selected-project default with explicit All Projects behavior.
5. **WP-06/WP-07 — detail infrastructure and Tasks:** implement one accessible reusable drawer pattern, validate it on a representative register, then add complete Task detail/edit/status workflows.
6. **WP-08 through WP-18:** proceed in handoff order for Projects cleanup, RFI/Submittal lifecycle actions, Project Controls hierarchy, phase prioritization, command palette, conservative numbering defaults, scope clarification, Settings/Recovery, reusable dialogs/feedback, and optional first-run flow.
7. **WP-19/WP-20 — hardening and release validation:** complete rendered UI, accessibility, offline, filesystem-safety, migration, performance, production Tauri build, and end-to-end verification before release recommendation.

Each package must leave lint, tests, typecheck/build, and any affected Rust gates passing. Significant UI packages also require rendered review under the Project UI Design Review skill.

## Known risks

- A partial state migration could create two competing sources of truth. Compatibility bridges must be temporary, explicit, and removed only after every consumer is migrated.
- Stale persisted project IDs can currently survive independently of the loaded project list. Shared state must reconcile persisted context against actual projects before applying defaults.
- Search currently loads multiple complete registers and coordinates focus indirectly. UX changes should not combine a search backend rewrite with the initial workspace-state migration.
- Project restore or sync conflict resolution can invalidate current project and focused-record state. The shared state needs a deliberate post-restore reconciliation path.
- Filesystem Move and restore are high-consequence flows currently protected by explicit confirmation text. Replacing browser dialogs must preserve exact consequence and path information before execution.
- Cloud snapshots cover implemented metadata. Any future schema or relationship change risks silent cross-device loss unless serialization, conflict behavior, and tests change together.
- Large, compressed screen components make broad edits hard to review. Extract only stable interaction infrastructure and avoid speculative abstraction.
- The dependency install warned that the resolved ESLint 9.39.5 release is deprecated. This is not a current gate failure and is outside WP-00 behavior changes, but it should be handled in a dedicated dependency-maintenance change rather than mixed into UX work.

## Recommended next package

Begin WP-01 with a pure, tested workspace reducer and persistence parser. Integrate it at the app shell first, then provide narrowly scoped compatibility behavior for existing project and focus consumers. Do not change navigation layout until the shared state passes stale-project, restore, search-focus, and All Projects tests.

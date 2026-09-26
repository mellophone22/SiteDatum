# WP-12 — Phase-Aware Prioritization

The selected project's phase now prioritizes project-module order in the sidebar. Prioritization changes proximity only: it never hides, disables, or restricts a module. A short textual cue identifies why the order changed, so the behavior is not conveyed by position or color alone.

## Mapping

- Preconstruction: Project Controls, Tasks, Files
- Engineering: RFIs, Submittals, Tasks
- Submittals: Submittals, RFIs, Tasks
- Procurement: Project Controls, Submittals, Tasks
- Construction: Project Controls, Tasks, RFIs
- Programming: Tasks, Files, Project Controls
- Startup and Commissioning: Project Controls, Tasks, Files
- Closeout: Files, Submittals, Tasks

The remaining modules follow each priority group, and all six modules are always returned exactly once. All Projects, custom phases, and unknown phase values use the stable default order.

The mapping lives in one pure frontend module with tests for representative ordering, completeness, uniqueness, and fallback behavior. No schema, migration, Rust command, permissions, or data-access behavior changed.

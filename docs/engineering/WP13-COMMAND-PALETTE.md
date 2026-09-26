# WP-13 — Command Palette and Search

Ctrl+K now opens a combined command palette and local record search. The palette remains fully local and reuses the existing typed list commands; no full-text-search migration or network dependency was introduced.

## Blank query

- Implemented actions: Open Home, Attention, Projects, and Project Controls
- Recently opened records, stored as a small local preference after a palette selection
- Recently opened projects, shared with the Projects register
- A real-project fallback when no project recency exists

No demo history or unavailable action is shown.

## Typed search

Results cover Projects, Tasks, RFIs, Submittals, Files, Notes, and Contacts. Entity group headings and textual type/project context make result identity scannable without relying on color. Selecting a project enters its Home workspace. Selecting a record establishes its project context, navigates to the owning module, and opens or focuses the exact record through shared workspace state.

Arrow Up/Down, Enter, Escape, focus trapping, focus restoration, loading, error, and empty states remain supported. Pure helper tests cover blank-query content, recent data, project fallback, and matching across label/type/project text.

No schema, migration, Rust command, or FTS implementation was required. The design remains compatible with replacing the frontend aggregation with a future backend search command.

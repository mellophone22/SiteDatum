# WP-07 — Task Detail and Editing

Tasks now use the shared detail panel for complete editing while retaining the quick-create form for rapid entry. Selecting a task preserves filters, sorting, saved views, progress, and Attention grouping.

## Data and commands

The existing `tasks.description` and `tasks.category` columns are now included in the Task read model. No migration was required. A validated `update_task` command updates the task and its Waiting fields atomically and records one activity event.

Changing a task's project is rejected when an RFI or Submittal relationship would cross project boundaries. Existing records remain compatible.

## Detail behavior

- Task title, project, priority, status, due date, category, and description are editable.
- Waiting requires a `Waiting on` value and supports an optional follow-up date.
- Complete Task is a direct status action.
- Related RFIs and Submittals are shown read-only when the current relationship model supports them.
- Escape and Close restore focus to the selected task; if a completed task leaves Attention, focus falls back to the page heading.

The browser-only preview verified the unchanged empty and quick-create states. Populated detail behavior is covered by frontend helper tests and native persistence tests; final native rendered verification remains part of the release hardening package.

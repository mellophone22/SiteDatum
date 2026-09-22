# WP2 — Tasks and Attention

## Delivered scope

- Local SQLite-backed task creation with project, title, priority, due date, category, and description.
- Constrained task statuses: Open, In Progress, Waiting, Blocked, Completed, Cancelled.
- Inline status changes. Selecting Waiting reveals a real Waiting On field and optional follow-up date; saving is disabled until Waiting On is non-blank.
- Search and project, status, and priority filters.
- Attention sections: Overdue, Today, Follow-up, Waiting, Upcoming. Sections have counts and use local calendar days (today through 14 days inclusive for upcoming work).
- Completed/cancelled tasks are excluded from Attention. Waiting tasks are grouped separately from due-date groups; follow-ups due today or earlier are shown under Follow-up.
- Task create/status activity events commit atomically with the task write.

## Scope boundary

This package does not add task deletion, RFI/submittal records, global search, dashboard metrics, a command palette, or project-specific task detail screens. No automatic task/project status or phase transitions are introduced. WP3 and later remain unstarted.

## Verification

See docs/engineering/TESTING_STRATEGY.md for coverage and run results recorded in the WP2 completion report. The Attention command receives local date boundaries from the UI; Rust validates those boundaries and owns the grouping rules.

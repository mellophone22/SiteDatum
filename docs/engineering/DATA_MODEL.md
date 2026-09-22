# Data Model

Core tables: projects, tasks, rfis, submittals, drawings, documents, notes, contacts, project_contacts, attachments, entity_relationships, activity_events, app_settings, recent_items, schema_migrations.

Use stable internal IDs independent of visible project/RFI numbers. Project number should be unique unless deliberately changed later. RFI/submittal numbers unique within project where present. Store timestamps consistently. Prefer archive/soft-delete for major records. Physical file deletion must never cascade from ordinary metadata deletion.

## WP2 task rules

Task priorities are low, medium, high, and urgent; task statuses are open, in_progress, waiting, blocked, completed, and cancelled. Dates are validated ISO calendar dates (YYYY-MM-DD) and are not timestamps. A waiting task requires a non-blank waiting_on; entering Waiting sets waiting_since_utc, and leaving Waiting clears both waiting metadata and its optional follow-up date. Task creation and status changes write activity events in the same SQLite transaction as the task mutation. Archived projects are excluded from task and Attention lists.

Attention grouping is exclusive: overdue (due before local today), today, follow-up (Waiting with follow-up due today or earlier), waiting (other active Waiting tasks), and upcoming (non-Waiting active tasks due from tomorrow through local today + 14 days, inclusive). Completed and cancelled tasks are excluded. Phase/status of the parent project does not drive task grouping. The client supplies local civil-day boundaries; Rust validates them and owns categorization.

## WP3 RFI rules

RFIs use stable UUIDs and a project-scoped, case-insensitive visible number. Their constrained statuses are `draft`, `open`, `response_received`, and `closed`. Every RFI requires project, number, subject, question, and created date. An RFI cannot leave Draft without a recipient, and it cannot enter Response Received or Closed without a recorded response. Opening an RFI supplies a submitted date when one has not been entered; receiving/closing supplies a response-received date when one has not been entered.

An RFI can reference one task in the same project through `rfi_task_relationships`. Attachment references store a path and filename only; they do not copy, move, delete, or otherwise own the referenced Windows file. Removing a reference deletes only the metadata reference. Open RFIs with a response due date through the Attention window are surfaced in the RFI Attention queue.

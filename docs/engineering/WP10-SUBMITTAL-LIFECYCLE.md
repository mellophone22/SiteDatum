# WP-10 — Submittal Lifecycle UX

The Submittal detail view now presents one explicit next action for the normal workflow while retaining the complete manual status selector for exceptions.

## Behavior

- Draft and Preparing records offer Submit submittal and require a recipient.
- Submitted and Under Review records offer a disposition selector plus Record disposition.
- Supported dispositions remain Approved, Approved As Noted, Revise & Resubmit, and Rejected.
- Recording a disposition supplies the current local date only when Response date is empty.
- Revise & Resubmit marks a revision as required and offers Create revision using the existing parent-child record flow.
- Final dispositions offer Close submittal; Closed records have no further normal lifecycle action.
- Existing dates are never replaced by lifecycle defaults.
- Lifecycle transitions save all current edits through the existing transactional `update_submittal` command.
- Attachments, related tasks, revision lineage, filters, and manual status editing remain available.

No schema, migration, or new backend command was required. Pure lifecycle helpers cover action availability, disposition handling, and non-destructive date defaults.

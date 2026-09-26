# WP-09 — RFI Lifecycle UX

The RFI detail panel now presents one explicit next action for the normal lifecycle: Submit RFI, Record response, or Close RFI. Closed RFIs have no further normal lifecycle action. Manual status editing remains available for exception handling.

## Behavior

- Submit RFI requires a recipient and supplies the current local date only when Submitted date is empty.
- Record response requires response text and supplies the current local date only when Response received is empty.
- Close RFI preserves the recorded response and dates.
- The response section opens automatically for Open, Response Received, and Closed RFIs.
- Secondary PDF-template fields remain available but start collapsed so they do not compete with lifecycle work.
- Disabled actions explain which required information is missing in adjacent text.
- Lifecycle actions save all current edits through the existing `update_rfi` command, so the RFI fields, related-task relationship, and activity event remain one SQLite transaction.
- Save only, Save and create PDF, attachment references, related tasks, filters, and manual status control are unchanged.

No schema, migration, or new backend command was required. Pure lifecycle helpers cover action availability and non-destructive date defaults.

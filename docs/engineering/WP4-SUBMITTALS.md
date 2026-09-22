# WP4 — Submittals

WP4 provides a usable local submittal register: constrained lifecycle states, dispositions, a searchable/filterable project register, a detail editor, same-project task relationships, and an Attention queue for Submitted and Under Review work.

Valid status values are `draft`, `preparing`, `submitted`, `under_review`, `approved`, `approved_as_noted`, `revise_and_resubmit`, `rejected`, and `closed`. A recipient is required after Draft/Preparing; a response date is required to record a disposition; closing retains the prior recorded disposition. Resubmission is only valid after Revise & Resubmit.

When resubmission is required, the user explicitly creates a child revision from that record. No status or phase is changed automatically. SQLite stores stable machine values and enforces status/disposition constraints; Rust validates the complete lifecycle and same-project relationships.

Attachment references point at existing Windows files. Adding or removing a reference never copies, moves, overwrites, or deletes the file. File ownership, copy/move/register workflows, and recovery behavior remain WP5 scope.

## Verification

Rust unit coverage validates the lifecycle gates. SQLite integration creates a submitted record, confirms Attention visibility, records Revise & Resubmit, creates an explicit child revision, closes the original while retaining its disposition, and confirms persistence after database reopen. TypeScript typecheck/build, ESLint, and Vitest also pass.

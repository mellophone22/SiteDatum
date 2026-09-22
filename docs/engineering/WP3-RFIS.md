# WP3 — RFIs

## Delivered scope

- Local SQLite RFIs with project-scoped number, subject, question, recipient, lifecycle status, submission/response dates, response, notes, and a related task.
- Lifecycle safeguards: Draft → Open requires a recipient; Response Received and Closed require a response. No automatic close, submission, or response-state transition occurs.
- Dense RFI list with search, project/status filters, and a context-preserving detail editor.
- A related-task selector limited to tasks for the chosen project.
- Explicit attachment references. Selecting a file registers only its existing path; it is neither copied nor moved. A reference can be shown in Explorer or removed without touching the original file.
- Open RFIs whose response due date falls within the Attention window appear under “RFIs awaiting response,” with a direct route to the RFI workspace.
- RFI create, update, and attachment-reference changes add atomic local activity events.

## Scope boundary

This package deliberately does not provide document registration, copying, moving, collision handling, or file relocation. Those remain WP5 work. Attachment references are intentionally path-only to avoid implying a managed-file workflow before it exists. Submittals, drawings, search, contacts, and broader activity views remain unstarted.

## Verification

The Rust suite verifies lifecycle validation, relationship persistence, attachment-reference behavior, response-due Attention behavior, activity events, and restart persistence. Frontend typecheck/build, lint, and Vitest are run as WP3 quality gates.

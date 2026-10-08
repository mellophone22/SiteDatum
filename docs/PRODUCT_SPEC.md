# Product Specification

## Project
Number, name, status, phase, customer, GC, engineer, PM, superintendent, location, start/target dates, description, important notes, project path, pinned/archive state, timestamps.

## Tasks
Title, project, description, priority, status, category, due date, follow-up date, waiting since, waiting on, related contact, timestamps, relationships/attachments. Statuses: Open, In Progress, Waiting, Blocked, Completed, Cancelled. Priorities: Low, Medium, High, Urgent.

## RFIs
Project, number, subject, question, created/submitted dates, recipient, status, response due/received, response, notes, RFI location, drawing number, cost impact, time delay, suggested solution, requested by, attachments/relationships. Statuses: Draft, Open, Response Received, Closed. A saved RFI can produce a one-page PDF from the approved RFI template without overwriting an existing document.

## Submittals
Project, number, name, package, revision, created/submitted dates, recipient, status/disposition, response date, resubmission required, notes, attachments/relationships. Statuses: Draft, Preparing, Submitted, Under Review, Approved, Approved As Noted, Revise & Resubmit, Rejected, Closed.

## Drawings
Project, drawing number, title, discipline, revision, revision/received dates, current/superseded state, file path, relationships.

## Other entities
Timestamped Notes; reusable Contacts; meaningful Activity events; generic entity relationships.

## Attention
Surface Overdue, Today, Upcoming, Waiting/Follow-up, RFIs awaiting response, and submitted submittals awaiting disposition.

Attention and Tasks show task completion as a compact progress gauge. The calculation is completed tasks divided by all non-cancelled tasks in the current project context. The gauge also reports open, in-progress, waiting, and blocked counts; it is operational feedback, not a workflow gate.

## Search
Search projects, tasks, RFIs, submittals, drawings, notes, contacts, and registered filenames.

Database = metadata/relationships source of truth. Filesystem = file bytes source of truth. Missing/moved files must be visible and recoverable.

## Operations workspace

Project controls use constrained registers for meeting minutes, procurement, change events, transmittals, and milestones. Field and commissioning work uses constrained registers for punch items, daily reports, startup checks, commissioning checks, and commissioning issues. These records support project relationships, status, priority, responsibility, dates, notes, archive state, and checklist counts where applicable. Phase and status remain organizational metadata rather than workflow engines.

Reusable project templates create task checklists and milestones in one database transaction. Bulk status changes are explicit, scoped to the current selection, and transactional.

## Planning, data exchange, and reports

The calendar is a compact dated register combining task due dates, waiting follow-ups, RFI/submittal dates, and operational milestones. Windows reminders are opt-in and run only while SiteDatum is open; no background service is installed.

Operational CSV and Excel imports require a preview and validate the complete batch before writing any rows. Filtered CSV and native Excel exports never overwrite existing files. Printable HTML reports use a visible template version and are generated for weekly status, open items, meeting minutes, transmittals, submittal covers, and contacts.

Complete data portability is separate from the filtered Operations export. Recovery can create an accountless, Free export directory containing one machine-readable CSV per core entity plus a JSON manifest with schema version and row counts. Active and archived projects and records are included. Project-document bytes are not copied; registered files and attachment references retain their Windows paths so the filesystem remains the file source of truth. Device settings, licensing state, and legacy synchronization internals are excluded.

## Audit and recovery

Recovery exposes searchable activity, missing-file diagnostics, complete CSV data portability, current unresolved cloud conflicts, and app-local plus configured external backup inventory with visible paths and timestamps. Users can create an external backup, opt into a daily or weekly schedule that runs while SiteDatum is open, or select a SiteDatum backup file directly. A restore requires schema and integrity validation, a record-count preview, explicit confirmation, and creation of a pre-restore safety backup. The restored database is migrated forward before normal use resumes. If the workspace database cannot open during startup, a recovery-only screen provides the failure reference and a non-destructive restore path while preserving the unavailable database.

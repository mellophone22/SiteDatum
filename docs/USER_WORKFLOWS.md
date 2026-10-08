# User Workflows

## Morning
Launch -> Attention shows overdue/today/follow-up/waiting/upcoming -> act without hunting for project -> changes update immediately.

## New project
Quick action -> enter project number, name, phase, and optional customer -> confirm exact path/folder preview -> create DB record + folder tree -> report partial failures precisely -> Open Workspace -> project Home. Complete secondary project metadata through Edit when needed.

## First run
On a completely empty installation -> choose and validate an existing project root -> enter the first project's number, name, phase, and optional customer -> review the exact folder path -> create the project -> enter its Home workspace. Choose Set up later to use normal local Settings instead. Local setup never requires an account or internet connection.

## Open or manage a project
Projects -> scan project number/name, status, phase, customer, and target date -> select the project identity to open an active workspace. Use More for View details, Edit project, Open folder, Pin/Unpin, and Archive/Restore. Archived project identities open a read-only overview first so restoration remains deliberate.

## Quick task
Open from a project workspace -> selected project is preselected but can be deliberately changed -> title -> optional due/priority/status -> save without leaving context. From All Projects, choose a project before saving.

## Quick capture
Ctrl+Shift+N or Quick capture -> choose Task, Note, RFI draft, or Contact -> inherit the selected project where applicable, or choose one from All Projects -> enter the minimum required information -> save -> open the corresponding workspace with refreshed data.

## Search and command palette
Ctrl+K -> with no query, choose a real recent record, recent project, or implemented workspace action -> or type to search projects, tasks, RFIs, submittals, files, notes, and contacts -> use Arrow keys and Enter -> establish the record's project context -> open the correct module and exact record. Escape closes the palette and restores focus to the search trigger.

## New project record
From Tasks, RFIs, Submittals, Files, Notes, or Project Controls -> New -> inherit the selected project -> deliberately override it when needed. In All Projects context, project-owned records require an explicit project choice; notes may remain general and reports may cover all projects.

## Notes, contacts, and activity
Open Notes & Contacts -> capture a note for the selected project or deliberately make it workspace-wide -> use the scope filter to review the current project, workspace-wide notes, or all notes. Contacts are a workspace-wide directory; project-contact relationships are not tracked. Audit activity is also workspace-wide and does not change with project context.

## Project Controls
Open Project Controls -> choose Coordination, Commercial, Schedule / Field, or Startup & Commissioning -> choose the required register -> create, filter, select, bulk-update, import, or export records. Calendar, Project Templates, and Reports remain available from the top view tabs.

## Saved task view
Choose filters and sort -> Save current view -> name it -> restore it from the Saved view selector. Changing a restored filter returns the selector to Current filters; deleting the saved view does not delete any records.

## Home
Choose All projects -> Home -> review active work, deterministic Attention buckets, RFIs awaiting response, submittals awaiting disposition, projects needing attention, upcoming tasks/milestones, notes, and workspace activity -> open a project or underlying register to act.

Choose a working project -> Home -> confirm project identity/status/phase -> review the same operational queues scoped to that project -> use project notes for project-specific history. The audit activity list remains explicitly workspace-wide until activity events carry project identity.

When a working project is selected, its phase places the most relevant workspace modules first in the sidebar. The sidebar states that its order is phase-prioritized; every module remains available regardless of phase.

## Waiting
Set Waiting -> capture `Waiting on` + optional follow-up -> record waiting start -> resurface when follow-up is due.

## Task detail
Select a task without losing the current filters or saved view -> review/edit project, priority, status, due date, category, and description -> save atomically. Use Complete task for the common completion action. Related RFIs and Submittals appear when linked; changing the task project is blocked if it would invalidate those relationships.

## RFI
New RFI -> receive a project-scoped canonical number suggestion when a project is known -> keep or edit it -> Draft -> enter a recipient -> Submit RFI -> enter the received response -> Record response -> review -> Close RFI. Each lifecycle action saves current edits and advances the RFI atomically. Manual status editing remains available for exceptions. Preserve linked tasks/files/drawings. Save only, or save and create a PDF from the approved RFI template in an explicitly selected location.

## Submittal
New Submittal -> receive a project-scoped canonical number suggestion when a project is known -> keep or edit it -> Draft/Preparing -> enter recipient -> Submit submittal -> choose and Record disposition -> Create revision when Revise & Resubmit requires one, otherwise Close submittal after final disposition. Lifecycle transitions save current edits atomically; manual status editing remains available for exceptions.

## File
Drop/select -> explicitly Copy, Move, or Register -> prevent overwrite -> store path/metadata -> Show in Explorer. SiteDatum reveals the registered file in Windows Explorer rather than directly opening the document.

## Missing file
Show Missing; offer Locate File / Remove Reference. Never silently delete metadata.

## Settings and recovery
Settings -> configure the Workspace project root, optional legacy Sync, and Notifications in distinct sections -> review Data & Recovery health -> create a verified app-local backup or open the full audit and recovery workspace. In Recovery, choose a user-controlled external folder, optionally enable a daily or weekly schedule while SiteDatum is open, create an immediate external backup, inspect exact backup locations, or preview and restore a selected `.sqlite3` file. Recovery remains visually quiet when healthy; missing registered files or sync conflicts add a numbered sidebar indicator and a direct review action.

If the local database cannot open during startup -> review the error code, correlation reference, workspace location, and backup location -> choose a SiteDatum `.sqlite3` backup -> review record counts -> explicitly confirm recovery -> close and reopen SiteDatum. SiteDatum verifies and migrates a candidate first, preserves the unavailable database in the backup folder, and does not touch normal project documents.

## Appearance

Use the header switch for an immediate Light/Dark change without leaving current work. Settings -> Appearance provides Light, Dark, and Windows default. Windows default follows changes to this computer's Windows app theme while SiteDatum is open; choosing Light or Dark holds that explicit preference across restarts.

## Consequential actions
Choose a destructive or filesystem-changing action -> review a focused confirmation that names the affected record and consequence -> confirm or cancel with keyboard or pointer -> return focus to the initiating control. Reference removal states that the physical file is preserved; Move shows the source and destination; restore states that a safety backup is created first.

## Keyboard and zoom
Use Tab to reach the skip link and move directly to the workspace. Quick Capture and Search trap focus while open, close with Escape, and restore focus to the initiating control. Use Arrow keys plus Home/End to change Quick Capture record type. At increased Windows scaling or 200% browser zoom, narrow layouts collapse to the compact sidebar and forms stack without removing actions.

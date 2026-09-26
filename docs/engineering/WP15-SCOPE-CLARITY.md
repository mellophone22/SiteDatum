# WP15 — Notes, Contacts, and Activity Scope

## Decision

The Notes & Contacts workspace now states the scope supported by the current schema:

- Notes can be assigned to one project or remain workspace-wide.
- Contacts are workspace-wide. The application does not imply project-contact relationships that are not stored.
- Audit activity is workspace-wide because activity events do not currently carry project identity.

## Interaction

The selected project remains the default for a new note. The saved-note filter starts in the current context and can explicitly show workspace-wide notes or all notes. Scope is shown in the register rather than inferred from surrounding navigation.

## Architecture impact

No schema or command changes were made. Adding project-contact relationships or project-scoped activity later requires an explicit migration and coordinated updates to persistence, import/export, search, and UI behavior.

# WP16 — Settings and Recovery

## Information architecture

Settings is organized into four operational sections:

- Workspace: project-root validation and persistence.
- Sync: optional metadata synchronization and conflict resolution.
- Notifications: opt-in Windows reminders while SiteDatum is running.
- Data & Recovery: backup creation, health summary, and entry to audit/restore tools.

## Recovery signal

The application derives a small recovery-health summary from missing registered files and unresolved sync conflicts. A healthy Recovery navigation item is visually subdued. Actionable issues add a non-color count and change the Data & Recovery action label.

## Preserved behavior

Backup creation, restore preview, pre-restore safety backup, missing-file routing, activity search, cloud conflict resolution, and offline local operation remain intact. No persistence or filesystem behavior changed.

# WP16 — Settings and Recovery

## Information architecture

Settings is organized into four standard operational sections, plus a conditional legacy section:

- Workspace: project-root validation and persistence.
- Notifications: opt-in Windows reminders while SiteDatum is running.
- Data & Recovery: backup creation, health summary, and entry to audit/restore tools.
- Legacy Sync: metadata synchronization and conflict resolution shown only on a computer with a saved legacy credential or prior local sync state. Fresh installations do not expose this section.

## Recovery signal

The application derives a small recovery-health summary from missing registered files and unresolved sync conflicts. A healthy Recovery navigation item is visually subdued. Actionable issues add a non-color count and change the Data & Recovery action label.

## Preserved behavior

Backup creation, restore preview, pre-restore safety backup, missing-file routing, activity search, grandfathered cloud conflict resolution, and offline local operation remain intact. Sync containment adds only a local eligibility setting and does not alter project records or files.

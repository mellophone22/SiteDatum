# ADR-014: Complete Free data portability

**Status:** Accepted — 2026-10-08

## Decision

SiteDatum provides an accountless complete-data export from Recovery on both Free and Pro. The user selects an existing parent folder. SiteDatum writes a new, uniquely named export directory containing 14 entity-specific UTF-8 CSV files and `manifest.json` with format version, database schema version, export time, file names, and row counts.

The exporter reads explicit column lists directly from SQLite so fields omitted from presentation models are not lost. It includes active and archived project metadata, tasks, RFIs, submittals, relationship tables, attachment references, registered-file metadata, notes, contacts, work items, project templates, and activity events. Text cells are protected against spreadsheet formula interpretation.

Document bytes are not copied. Project documents remain ordinary Windows files; the CSVs preserve their registered paths and attachment references. Device settings, commercial credentials or state, database migration internals, and contained legacy-sync implementation tables are not exported.

## Reliability and overwrite policy

The exporter creates a private UUID-named partial directory inside the selected destination, writes and flushes every CSV and the manifest, then renames the completed directory into place. The final name is unique and existing paths are never overwritten. If any step fails, SiteDatum removes only the exact partial directory created for that attempt and leaves the database and existing exports unchanged.

## Consequences

- A customer can retrieve all core machine-readable metadata after downgrade or without creating an account.
- Native Excel interchange remains a Pro productivity feature; complete CSV portability is not a Pro gate.
- The format is intentionally transparent and versioned rather than a proprietary archive.
- Re-import of this full relational set is not implied by this decision; verified SQLite backup/restore remains the lossless in-product recovery path.

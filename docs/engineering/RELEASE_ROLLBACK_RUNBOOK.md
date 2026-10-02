# SiteDatum release and rollback runbook

## Purpose

This runbook covers a local-first Windows release that may contain SQLite migrations. It does not authorize deleting or replacing customer project documents. Project files remain normal Windows files and are outside the application-database rollback procedure.

## Roles and custody

- The release operator builds and verifies the candidate from a clean tagged commit.
- Microsoft Store retains production signing authority for the public MSIX package. Any self-signed development certificate is restricted to disposable operator-controlled test machines and remains outside the repository and CI logs.
- Only public verification material belongs in the application or repository.

## Release preparation

1. Confirm the release commit is immutable and the working tree is clean.
2. Confirm frontend quality, Rust quality, dependency audits, and secret detection pass in GitLab.
3. Run the complete Windows Rust suite locally, including migration-recovery tests.
4. Run the Supabase pgTAP suite against an isolated non-production project when the release changes licensing-service migrations.
5. Build the production Tauri executable and NSIS installer from the verified commit.
6. Install the package for a Windows test user and complete native smoke, backup/export, and scaling checks.
7. If the release adds a database migration, test both:
   - upgrade from the last public schema with representative records; and
   - deliberate migration failure, confirming transaction rollback and a valid pre-migration snapshot.
8. Build the MSIX using the Partner Center-assigned identity, then run the Windows App Certification Kit and native MSIX acceptance checks.
9. Submit the immutable package to Partner Center. Record hashes, version, package identity, build commit, certification result, and publication time. Microsoft signs and publishes the certified Store package.

## Automatic migration safety

On first startup of a newer schema, SiteDatum:

1. opens the existing local SQLite database with foreign keys and WAL enabled;
2. reads the highest `schema_migrations` version;
3. writes a validated snapshot beside the database under `backups/`;
4. applies each pending migration in its own transaction; and
5. starts the workspace only after every migration succeeds.

The safety snapshot is named `pre-migration-v{from}-to-v{to}-{timestamp}.sqlite3`. It contains application records only; it does not copy, move, overwrite, or delete project documents.

## Release monitoring and stop conditions

Stop the Store submission or halt its rollout if any of these occur:

- signature or hash mismatch;
- installer or startup failure on a supported Windows baseline;
- migration, integrity-check, backup, or restore failure;
- missing or unexpectedly changed customer records;
- an older executable writes to a newer schema;
- backup or essential export becomes unavailable; or
- billing/licensing behavior restricts editing of existing local records.

Preserve the failed release artifact, commit, logs, correlation IDs, and a copy of the affected database only with the customer's explicit permission. Never request project documents or place private customer data in GitLab, Slack, Stripe, Supabase, or telemetry.

## Rollback decision

### No schema change

1. Halt the affected Store submission or rollout.
2. Restore the last known-good Store package as the available production version through Partner Center.
3. Install the known-good version over the affected version.
4. Launch SiteDatum and verify project counts, representative records, backup, and essential export.

Do not replace the workspace database when the schema did not change.

### Schema advanced but the new application still opens

Prefer a forward fix. Halt the affected Store rollout, correct the application or add a new forward migration, execute the full release gates, and submit a higher package version. Do not run ad-hoc downgrade SQL against a customer database.

### Schema advanced and the application cannot operate safely

1. Halt the affected Store rollout in Partner Center.
2. Preserve the current `workspace.sqlite3` and every file in its `backups/` directory. Do not overwrite either.
3. Identify the snapshot whose name matches the prior and affected schema versions.
4. Validate the snapshot with the SiteDatum recovery preview or SQLite `PRAGMA integrity_check` on an operator-controlled copy.
5. Reinstall the last known-good signed application version.
6. Restore the validated pre-migration snapshot through SiteDatum's recovery flow. The restore flow creates its own pre-restore safety backup.
7. Verify project and register counts, representative records, foreign-key enforcement, backup, and essential export.
8. Retain the newer database until the customer confirms recovery. Never silently delete it.

Any work entered after the migration snapshot must be reconciled deliberately. Restoring the snapshot rewinds application records to the snapshot time; it does not alter external project documents.

### Older application reports a newer schema

Do not bypass the error and do not delete the database. Reinstall the newer SiteDatum version that supports the recorded schema. Use the schema-advanced recovery procedure only when an approved rollback explicitly requires restoring the matching pre-migration snapshot.

## Post-rollback verification

- SiteDatum launches without migration or integrity errors.
- Existing projects and records are visible and editable.
- Project file paths still refer to the same normal Windows files.
- Foreign keys are enabled and `PRAGMA integrity_check` returns `ok`.
- Backup and essential export remain available regardless of entitlement state.
- The rollback version, restored snapshot, verification result, and incident owner are recorded without customer content or secrets.


# SiteDatum release and rollback runbook

## Purpose

This runbook covers a local-first Windows release that may contain SQLite migrations. It does not authorize deleting or replacing customer project documents. Project files remain normal Windows files and are outside the application-database rollback procedure.

## Roles and custody

- The release operator builds and verifies the candidate from a clean immutable commit.
- Controlled Early Access currently uses an unsigned NSIS installer. Its version, source commit, byte size, SHA-256, `NotSigned` state, and publication date must match the canonical release page.
- Production licensing coordinates supplied to the build are public verification material only. Billing secrets, entitlement private keys, webhook secrets, and reconciliation credentials remain server-side and outside the repository and CI logs.
- A future Microsoft Store or Authenticode channel has separate signing-custody and certification gates. It must not be implied by the current unsigned channel.

## Release preparation

1. Confirm the release commit is immutable and the working tree is clean.
2. Confirm the GitHub Actions frontend, Rust, Windows native/provenance, and disposable Supabase jobs pass for the exact commit. Review repository secret-scanning results. GitLab history remains supporting evidence, not the current source-of-record gate.
3. Run the complete local C8 preflight and review every deferred item; zero failed checks are permitted.
4. Run the complete Windows Rust suite locally, including migration-recovery tests.
5. Run the Supabase pgTAP suite against an isolated non-production project when the release changes licensing-service migrations.
6. Build the fail-closed production-commerce Tauri executable and NSIS installer from the verified commit using only the approved public production licensing coordinates.
7. Generate a staged release manifest and independently verify it against the same installer. The source/evidence commits, synchronized version, filename, size, SHA-256, and Authenticode state must match.
8. Install the package for a disposable Windows test user and complete native startup, backup/export, offline, upgrade/reinstall-preservation, keyboard, and scaling checks.
9. If the release adds a database migration, test both:
   - upgrade from the last public schema with representative records; and
   - deliberate migration failure, confirming transaction rollback and a valid pre-migration snapshot.
10. Update the canonical HTTPS page with the exact immutable artifact metadata and required unsigned Early Access disclosure before enabling its download or Checkout path.
11. Download the public-origin copy, recalculate SHA-256, confirm its signature state, and repeat the bounded install/startup smoke. A mismatch is a release stop.

For a future trusted Store or Authenticode release, complete that channel's assigned identity, signing, certification, signature-verification, and updater gates in addition to—not instead of—the data and migration checks above.

## Automatic migration safety

On first startup of a newer schema, SiteDatum:

1. opens the existing local SQLite database with foreign keys and WAL enabled;
2. reads the highest `schema_migrations` version;
3. writes a validated snapshot beside the database under `backups/`;
4. applies each pending migration in its own transaction; and
5. starts the workspace only after every migration succeeds.

The safety snapshot is named `pre-migration-v{from}-to-v{to}-{timestamp}.sqlite3`. It contains application records only; it does not copy, move, overwrite, or delete project documents.

## Release monitoring and stop conditions

Disable the affected download and purchase entry points, or stop a future trusted-channel submission, if any of these occur:

- signature or hash mismatch;
- installer or startup failure on a supported Windows baseline;
- migration, integrity-check, backup, or restore failure;
- missing or unexpectedly changed customer records;
- an older executable writes to a newer schema;
- backup or essential export becomes unavailable; or
- billing/licensing behavior restricts editing of existing local records.

Preserve the failed release artifact, commit, logs, correlation IDs, and a copy of the affected database only with the customer's explicit permission. Never request project documents or place private customer data in GitHub, GitLab, Slack, Stripe, Supabase, or telemetry.

## Rollback decision

### No schema change

1. Disable the affected canonical download and purchase entry points without deleting the immutable artifact or its evidence.
2. Identify the last known-good installer and verify its source commit, version, SHA-256, signature state, and matching hosted CI run.
3. Restore that exact immutable installer as the offered Early Access version, including its matching disclosure and metadata. Do not replace bytes beneath an existing release identity.
4. Install the known-good version over the affected version on a disposable profile before offering it again.
5. Launch SiteDatum and verify project counts, representative records, backup, and essential export.

Do not replace the workspace database when the schema did not change.

### Schema advanced but the new application still opens

Prefer a forward fix. Disable the affected download and purchase entry points, correct the application or add a new forward migration, execute the full release gates, and publish a higher immutable package version. Do not run ad-hoc downgrade SQL against a customer database.

### Schema advanced and the application cannot operate safely

1. Disable the affected download and purchase entry points. If a trusted-channel rollout exists later, halt it through that channel as well.
2. Preserve the current `workspace.sqlite3` and every file in its `backups/` directory. Do not overwrite either.
3. Identify the snapshot whose name matches the prior and affected schema versions.
4. Validate the snapshot with the SiteDatum recovery preview or SQLite `PRAGMA integrity_check` on an operator-controlled copy.
5. Reinstall the last known-good verified application version. For unsigned Early Access, confirm the exact published digest and `NotSigned` status; for a future signed channel, require a valid signature.
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


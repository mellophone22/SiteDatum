# C1 Entitlement Domain

Status: Completed 2026-09-29

## Outcome

C1 adds a provider-neutral Rust domain module for SiteDatum's agreed Free and Pro policy. The module is deliberately dormant: it is not connected to Tauri commands, the database, application screens, project mutations, a licensing service, or a billing provider. Current application behavior is unchanged until the separately scoped C2 enforcement package.

## Policy encoded

- Free permits up to three active projects.
- Pro monthly and Pro annual share the same product capabilities.
- Existing records always remain editable, including after expiration or downgrade.
- Local backup/restore and complete machine-readable CSV data export remain available on Free.
- Templates, bulk operations, spreadsheet import/export, and professional reports require an effective Pro entitlement.
- Metadata sync is disabled in this commercial policy because it is deferred from the initial paid launch.
- Pro permits two active Windows devices.
- When entitlement verification is unavailable, a previously verified and otherwise unexpired Pro entitlement receives an inclusive 21-day grace period.
- Known expiration overrides grace. Missing evidence and local-clock rollback fail closed to Free without restricting existing-data editing or portability.
- Canceled and past-due subscriptions remain effective only through their trusted paid-through time.

## Boundary

The implementation lives in `src-tauri/src/entitlement.rs` and exposes pure data types and policy functions from the Rust library. It stores no customer or project data, performs no I/O, contains no provider SDK, sends no telemetry, and introduces no secret or public client identifier.

This is not an application architecture change. It establishes the centralized policy seam that C2 can call from consequential Rust mutation boundaries.

## Verification

The module has focused unit coverage for the Free project limit, Pro device allowance, capability access, inclusive grace boundary, known expiration, canceled/past-due paid-through behavior, missing evidence, clock rollback, and the non-destructive editing guarantee.

No visual review is required for C1 because it changes no rendered interface.

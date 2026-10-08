# ADR-015: Device self-service and subscription continuity

**Status:** Accepted and deployed to SiteDatum Licensing Test; production deployment pending — 2026-10-08

## Decision

SiteDatum gives a signed-in customer a compact **Manage computers** surface in Account & Subscription. It lists only that customer's active pseudonymous device registrations, identifies the current installation, and permits an explicit confirmed deactivation. A Pro subscription continues to support two active Windows computers.

The desktop sends a stable random device identity only as a SHA-256 fingerprint. The licensing service never receives or returns Windows machine names, project names, records, contacts, document contents, database contents, or file paths. The UI therefore labels registrations as **This computer** or **Windows computer**, with activation and last-verification dates rather than invented names.

## Trust boundary

An authenticated Edge Function resolves the bearer token to the Supabase Auth subject and calls service-role-only database functions. Listing is scoped by that subject. Deactivation reuses the existing ownership-scoped `licensing_deactivate_device` function. Neither RPC is executable by `anon` or `authenticated`; the browser/desktop cannot select an arbitrary customer.

The endpoint rate-limits management requests and exposes no fingerprint hashes. Device removal writes the existing allowlisted `device_deactivated` audit event. Deactivating the current installation deletes only its cached signed entitlement, leaving its account session, stable local identity, workspace database, project folders, backups, and exports untouched. It immediately applies the non-destructive Free policy.

## Continuity behavior

When a cached entitlement becomes due, Settings attempts one background refresh. An unavailable service does not discard otherwise valid cached evidence. Days 1–13 of the bounded 21-day offline grace remain quiet; days 14–17 show a low-pressure verification notice; days 18–21 show stronger recovery guidance. Paid-through time always caps grace.

If a third computer attempts activation, the desktop presents a specific recovery route: open **Manage computers**, retire an old activation, then refresh. A remote computer may retain already cached, signed Pro access only until its existing verification or paid-through boundary; deactivation does not remotely delete local data.

## Consequences

- Routine device replacement no longer requires founder support.
- Device management remains a narrow commercial boundary and does not introduce workspace collaboration or a cloud requirement for ordinary work.
- The migration and `licensing-devices` function are verified in SiteDatum Licensing Test. Production promotion remains a separate reviewed release action.
- The service cannot provide user-defined device labels without collecting additional customer-supplied metadata, so labels are deliberately omitted.

# C8 — Launch audit

**Status:** Preflight harness implemented; release remains blocked pending manual, legal, and C6 distribution gates — 2026-10-02

## Purpose

C8 gathers evidence that SiteDatum can be installed, upgraded, licensed, disconnected, recovered, and operated without risking customer work. It does not authorize live Stripe mode, real charges, production secrets, public distribution of an unsigned installer, or collection of project content.

Run the static boundary check with `npm run audit:c8`. Run the complete local frontend and Rust gate with `npm run audit:c8:full`.

## Acceptance matrix

| ID | Gate | Evidence | Current state |
|---|---|---|---|
| C8-01 | Candidate provenance | Clean commit, matching package/Tauri/Rust version, installer hash | Automated version check; clean tagged candidate remains required |
| C8-02 | Secret and environment boundary | GitLab secret detection plus repository live-token scan | Automated preflight and CI |
| C8-03 | Frontend quality | Site validator, lint, Vitest, production build | Automated full preflight |
| C8-04 | Native quality | Rust format and locked test suite | Automated full preflight |
| C8-05 | Clean-profile install | Install internal candidate for a disposable Windows user; complete first run | Manual, pending |
| C8-06 | Upgrade and migration | Upgrade representative prior-schema workspace; verify snapshot, records, and integrity | Domain tests pass; native scenario pending |
| C8-07 | Uninstall/reinstall recovery | Reinstall without consuming an extra device and without deleting workspace data | Manual sandbox scenario, pending |
| C8-08 | Offline behavior | Disconnect network; verify CRUD, search, backup/export, entitlement grace, and honest status | Domain coverage exists; native scenario pending |
| C8-09 | Payment lifecycle | Monthly and annual checkout, portal, failure, recovery, cancellation, expiration, and refund | Stripe/Supabase sandbox only; manual scenario pending |
| C8-10 | Two-device allowance | Activate two disposable device identities, reject third, deactivate and replace | Sandbox only; manual scenario pending |
| C8-11 | Non-destructive downgrade | Existing records remain visible/editable; backup and essential export stay available | Automated domain coverage plus manual confirmation pending |
| C8-12 | Accessibility and scaling | Keyboard-critical paths and exact 200% Windows scaling | Prior 200% pass recorded; repeat on release candidate |
| C8-13 | Support and incident response | Private billing path, redacted evidence, stop/rollback decisions | Founder operations runbook implemented; private support contact pending |
| C8-14 | Legal and public surfaces | Founder/legal-approved policies and publisher contacts | Drafts only; launch blocker |
| C8-15 | Trusted distribution | Microsoft-signed MSIX or separately approved trusted route | C6 deferred; launch blocker |

## Disposable-profile procedure

Use a dedicated local Windows test account or disposable virtual machine. Never point the test at a real project folder or copy a customer database into it.

1. Record candidate commit, version, installer SHA-256, and Authenticode status.
2. Install and launch SiteDatum. Confirm the product name and first-run experience.
3. Select a temporary test root and create only fictional records.
4. Restart and verify selected context, registers, ordinary Windows files, and local editing.
5. Exercise task, RFI, submittal, file, controls, backup, export, and recovery paths.
6. Disconnect networking and repeat representative CRUD, search, backup, and export operations.
7. If a prior candidate is part of the test, upgrade it and verify the pre-migration snapshot before continuing.
8. Uninstall the application without deleting the test workspace, reinstall, and verify recovery.
9. Inspect Home, Projects, Tasks, RFIs, Submittals, Files, Controls, Recovery, and Settings at exact 200% Windows scaling.
10. Remove the disposable account or VM only after preserving the content-free test record and hashes.

## Sandbox payment procedure

All billing work stays in the connected Stripe and Supabase sandboxes. Use fictional customer details and Stripe test payment methods. Confirm the Dashboard says sandbox/test mode before every mutation.

Evidence should record event IDs, subscription state transitions, HTTP delivery status, entitlement plan/state, paid-through behavior, and anonymized device counts. Do not paste secrets, payment details, passwords, project records, databases, or private customer information into the repository, GitLab, or Slack.

## Stop conditions

Stop the audit and preserve evidence if any test causes record loss, overwrites a file without confirmation, makes existing records uneditable after expiration, removes backup/export, accepts an unverified entitlement, contacts live Stripe mode, exposes a secret, or produces a migration/integrity failure. Follow `RELEASE_ROLLBACK_RUNBOOK.md`; do not improvise destructive database repair.

## Launch decision

C8 can report engineering readiness before every external gate is resolved, but public launch remains **NO-GO** until C8-14 and C8-15 are complete and every required manual release-candidate scenario passes. Deferred means unverified, not accepted.

Founder routine operations, incident severity, billing recovery, outage, security/privacy, communications, and launch hold points are defined in `FOUNDER-OPERATIONS-RUNBOOK.md`.

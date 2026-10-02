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
| C8-05 | Clean-profile install | Install internal candidate for a disposable Windows user; complete first run | Manual pass 2026-10-02 on dedicated `SiteDatumC8` profile |
| C8-06 | Upgrade and migration | Upgrade representative prior-schema workspace; verify snapshot, records, and integrity | In-place corrected-candidate reinstall preserved current records; representative prior-schema migration remains pending |
| C8-07 | Uninstall/reinstall recovery | Reinstall without consuming an extra device and without deleting workspace data | Local uninstall/reinstall recovery passed; licensed-device accounting remains pending |
| C8-08 | Offline behavior | Disconnect network; verify CRUD, search, backup/export, entitlement grace, and honest status | Manual offline CRUD, search, backup, CSV export, and reconnect pass 2026-10-02 |
| C8-09 | Payment lifecycle | Monthly and annual checkout, portal, failure, recovery, cancellation, expiration, and refund | Sandbox monthly checkout, entitlement, portal, scheduled cancellation, and failed-renewal projection passed; recovery and remaining scenarios pending |
| C8-10 | Two-device allowance | Activate two disposable device identities, reject third, deactivate and replace | Sandbox only; manual scenario pending |
| C8-11 | Non-destructive downgrade | Existing records remain visible/editable; backup and essential export stay available | Free-mode record preservation, backup/restore, and CSV export passed; verified Pro-to-Free transition remains pending |
| C8-12 | Accessibility and scaling | Keyboard-critical paths and exact 200% Windows scaling | Manual keyboard-critical paths and all core screens passed at exact 200% scaling 2026-10-02 |
| C8-13 | Support and incident response | Private billing path, redacted evidence, stop/rollback decisions | Founder operations runbook implemented; private support contact pending |
| C8-14 | Legal and public surfaces | Founder/legal-approved policies and publisher contacts | Drafts only; launch blocker |
| C8-15 | Trusted distribution | Microsoft-signed MSIX or separately approved trusted route | C6 deferred; launch blocker |

## Disposable-profile procedure

Use a dedicated local Windows test account or disposable virtual machine. Never point the test at a real project folder or copy a customer database into it.

From inside that disposable profile, first inspect the candidate without changing state:

```powershell
.\scripts\c8-disposable-profile.ps1
```

The runner prints the profile, installer hash, signature state, proposed temporary root, and whether an existing installation was found. It refuses execution unless the profile name is clearly disposable, the test root stays under that profile's temporary directory, no existing per-user install is present, and the operator explicitly supplies both `-Execute` and `-DisposableProfileAcknowledged`.

Run the eight-second clean-profile install/startup smoke with:

```powershell
.\scripts\c8-disposable-profile.ps1 -Execute -SmokeOnly -DisposableProfileAcknowledged
```

Omit `-SmokeOnly` to leave the application open for the manual checklist. The runner writes a content-free evidence JSON file under the disposable temporary root. It never removes the account, uninstalls SiteDatum, or deletes a test workspace automatically.

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

## Manual candidate evidence — 2026-10-02

The unsigned internal NSIS candidate was exercised in the dedicated local `SiteDatumC8` Windows profile. The corrected candidate installer SHA-256 was `354697BA9F3072B282D8CC5669A10AB0F4F0D780156C4580E81A73AAF9D8CD59`; unsigned status remains expected only because trusted distribution is still deferred under C8-15.

- Clean install, first-run root selection, fictional project creation, direct restart into the selected context, and normal per-user launch passed.
- A task, RFI, submittal, registered ordinary Windows file, and project-control milestone persisted. An RFI optional-date serialization defect found during the run was corrected, regression-tested, repackaged, and verified in the same disposable profile.
- The corrected installer preserved the current workspace in place. A representative older-schema migration and its pre-migration snapshot still require a separate fixture-based run.
- Local backup preview and restore passed. A post-backup task was removed by restore while the backed-up task, RFI, submittal, milestone, registered file, and ordinary project files remained intact.
- With networking disconnected, local task creation, workspace search, local backup, and CSV export passed. The application continued to present itself honestly as a local workspace and the new records remained available after reconnecting.
- Windows uninstall left the fictional project folder and ordinary files intact. Reinstall reopened the existing project without repeating first-run setup and restored access to all tested metadata. Licensed-device reuse remains part of C8-10 rather than this Free-mode run.
- In Free mode, manual RFI creation remained available while professional PDF output was refused as a Pro feature; the refusal did not discard the RFI. Backup/restore and essential CSV export remained available. A verified Pro-to-Free entitlement transition is still required before closing C8-11.
- At exact 200% Windows scaling, Home, Attention, Projects, Tasks, RFIs, Submittals, Files, Project Controls, Recovery, and Settings remained usable. Project search opened from the keyboard, keyboard selection opened the RFI, Escape dismissed the context, and visible focus navigation passed.
- In the isolated Stripe/Supabase sandbox, monthly Checkout activated Pro, the signed entitlement unlocked professional RFI PDF output, and the hosted billing portal loaded the correct sandbox subscription. Scheduling cancellation preserved Pro through the paid-through date. The run exposed Stripe's `cancel_at` representation for portal-scheduled cancellation; the adapter was corrected, regression-tested, deployed to the sandbox webhook and reconciliation functions, and verified with a new webhook delivery projecting the subscription as canceled without shortening paid access. Annual checkout, failed-payment recovery, terminal expiration, and refund remain pending for C8-09.
- A separate fictional monthly test-clock fixture reached a failed renewal and projected `past_due`. That run exposed an earlier `active` subscription-update event advancing `current_period_end` before collection failed. The database projection now requires `invoice.paid` (or payment-aware reconciliation with a paid latest invoice) before extending an existing paid-through boundary. The forward migration and reconciliation update passed 62 local database assertions and a rollback-only hosted ordering check; the fixture now retains its last successfully paid boundary. Recovery, terminal expiration, and refund remain pending for this fixture.

## Sandbox payment procedure

All billing work stays in the connected Stripe and Supabase sandboxes. Use fictional customer details and Stripe test payment methods. Confirm the Dashboard says sandbox/test mode before every mutation.

Evidence should record event IDs, subscription state transitions, HTTP delivery status, entitlement plan/state, paid-through behavior, and anonymized device counts. Do not paste secrets, payment details, passwords, project records, databases, or private customer information into the repository, GitLab, or Slack.

## Stop conditions

Stop the audit and preserve evidence if any test causes record loss, overwrites a file without confirmation, makes existing records uneditable after expiration, removes backup/export, accepts an unverified entitlement, contacts live Stripe mode, exposes a secret, or produces a migration/integrity failure. Follow `RELEASE_ROLLBACK_RUNBOOK.md`; do not improvise destructive database repair.

## Launch decision

C8 can report engineering readiness before every external gate is resolved, but public launch remains **NO-GO** until C8-14 and C8-15 are complete and every required manual release-candidate scenario passes. Deferred means unverified, not accepted.

Founder routine operations, incident severity, billing recovery, outage, security/privacy, communications, and launch hold points are defined in `FOUNDER-OPERATIONS-RUNBOOK.md`.

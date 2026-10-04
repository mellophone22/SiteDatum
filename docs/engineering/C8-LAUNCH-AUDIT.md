# C8 — Launch audit

**Status:** Controlled unsigned Early Access distribution accepted; trusted signed general availability remains deferred — 2026-10-03

## Purpose

C8 gathers evidence that SiteDatum can be installed, upgraded, licensed, disconnected, recovered, and operated without risking customer work. It does not authorize live Stripe mode, real charges, production secrets, collection of project content, or publication of an unsigned installer outside the controlled Early Access rules in ADR-011.

Run the static boundary check with `npm run audit:c8`. Run the complete local frontend and Rust gate with `npm run audit:c8:full`.

## Acceptance matrix

| ID | Gate | Evidence | Current state |
|---|---|---|---|
| C8-01 | Candidate provenance | Clean commit, matching package/Tauri/Rust version, installer hash | Passed: 1.4.1 candidate recorded from clean commit `a9ab136`; copied hash and disposable-profile acceptance passed; prerelease provenance tag `v1.4.1-early-access.1` identifies the exact source without representing a signed GA release |
| C8-02 | Secret and environment boundary | GitLab secret detection plus repository live-token scan | Automated preflight and CI |
| C8-03 | Frontend quality | Site validator, lint, Vitest, production build | Automated full preflight |
| C8-04 | Native quality | Rust format and locked test suite | Automated full preflight |
| C8-05 | Clean-profile install | Install internal candidate for a disposable Windows user; complete first run | Manual pass 2026-10-02 on dedicated `SiteDatumC8` profile |
| C8-06 | Upgrade and migration | Upgrade representative prior-schema workspace; verify snapshot, records, and integrity | Automated representative schema-v9 to schema-v10 upgrade passed 2026-10-03; corrected-candidate reinstall separately preserved current records |
| C8-07 | Uninstall/reinstall recovery | Reinstall without consuming an extra device and without deleting workspace data | Exact 1.4.1 candidate uninstall/reinstall preserved the fictional workspace and verified Pro entitlement; hosted device-accounting rules passed under C8-10 |
| C8-08 | Offline behavior | Disconnect network; verify CRUD, search, backup/export, entitlement grace, and honest status | Manual offline CRUD, search, backup, CSV export, and reconnect pass 2026-10-02 |
| C8-09 | Payment lifecycle | Monthly and annual checkout, portal, failure, recovery, cancellation, expiration, and refund | Passed in the isolated Stripe/Supabase sandbox |
| C8-10 | Two-device allowance | Activate two disposable device identities, reject third, deactivate and replace | Hosted Supabase sandbox transactional pass 2026-10-03; no test identity or licensing rows retained |
| C8-11 | Non-destructive downgrade | Existing records remain visible/editable; backup and essential export stay available | Passed 2026-10-03: verified Pro changed authoritatively to Free/Expired; existing-record editing, backup, CSV export, and Pro-feature locking passed |
| C8-12 | Accessibility and scaling | Keyboard-critical paths and exact 200% Windows scaling | Manual keyboard-critical paths and all core screens passed at exact 200% scaling 2026-10-02 |
| C8-13 | Support and incident response | Private billing path, redacted evidence, stop/rollback decisions | Passed 2026-10-03: private inbox delivery verified; redacted public intake and content-free incident workflow implemented |
| C8-14 | Legal and public surfaces | Founder-approved policies and publisher contacts | Passed 2026-10-03: policies effective; founder decision to proceed without qualified legal review recorded as accepted risk |
| C8-15 | Controlled Early Access distribution | Pre-purchase and pre-download unsigned disclosure; immutable artifact metadata; public-origin install and refund/support evidence | Passed 2026-10-03: canonical Hostinger publication, byte-for-byte public-origin verification, disposable-profile installation, support/refund journey, and Managed Payments disclosure passed |

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

The unsigned internal NSIS candidate was exercised in the dedicated local `SiteDatumC8` Windows profile. The corrected candidate installer SHA-256 was `354697BA9F3072B282D8CC5669A10AB0F4F0D780156C4580E81A73AAF9D8CD59`. That evidence proves the tested internal candidate only; it does not make the file the final public Early Access artifact or authorize a download link.

- Clean install, first-run root selection, fictional project creation, direct restart into the selected context, and normal per-user launch passed.
- A task, RFI, submittal, registered ordinary Windows file, and project-control milestone persisted. An RFI optional-date serialization defect found during the run was corrected, regression-tested, repackaged, and verified in the same disposable profile.
- The corrected installer preserved the current workspace in place. A separate automated schema-v9 fixture then exercised the real startup upgrade path to schema v10 with representative project, task, RFI, submittal, relationship, attachment, registered-file, note, contact, sync-marker, and activity records. The migrated database and version-9 safety snapshot both passed `PRAGMA integrity_check`; foreign keys remained enabled, the new operations table was present only after migration, reopening created no duplicate snapshot, and the referenced ordinary project file remained byte-for-byte unchanged. This closes C8-06.
- Local backup preview and restore passed. A post-backup task was removed by restore while the backed-up task, RFI, submittal, milestone, registered file, and ordinary project files remained intact.
- With networking disconnected, local task creation, workspace search, local backup, and CSV export passed. The application continued to present itself honestly as a local workspace and the new records remained available after reconnecting.
- Windows uninstall left the fictional project folder and ordinary files intact. Reinstall reopened the existing project without repeating first-run setup and restored access to all tested metadata. Licensed-device reuse remains part of C8-10 rather than this Free-mode run.
- In Free mode, manual RFI creation remained available while professional PDF output was refused as a Pro feature; the refusal did not discard the RFI. Backup/restore and essential CSV export remained available.
- At exact 200% Windows scaling, Home, Attention, Projects, Tasks, RFIs, Submittals, Files, Project Controls, Recovery, and Settings remained usable. Project search opened from the keyboard, keyboard selection opened the RFI, Escape dismissed the context, and visible focus navigation passed.
- In the isolated Stripe/Supabase sandbox, monthly Checkout activated Pro, the signed entitlement unlocked professional RFI PDF output, and the hosted billing portal loaded the correct sandbox subscription. Scheduling cancellation preserved Pro through the paid-through date. The run exposed Stripe's `cancel_at` representation for portal-scheduled cancellation; the adapter was corrected, regression-tested, deployed to the sandbox webhook and reconciliation functions, and verified with a new webhook delivery projecting the subscription as canceled without shortening paid access. The previously recorded C4 annual Checkout and renewal evidence completes the annual path.
- A separate fictional monthly test-clock fixture reached a failed renewal and projected `past_due`. That run exposed an earlier `active` subscription-update event advancing `current_period_end` before collection failed. The database projection now requires `invoice.paid` (or payment-aware reconciliation with a paid latest invoice) before extending an existing paid-through boundary. The forward migration and reconciliation update passed 62 local database assertions and a rollback-only hosted ordering check. After replacing the failed test card, Stripe's scheduled retry paid the invoice on its second attempt, restored `active`, and advanced paid-through by one month only when the applied `invoice.paid` event arrived. Scheduling cancellation and advancing past that paid period produced an applied `customer.subscription.deleted` event, projected `expired`, and preserved the final paid-through boundary. A partial refund followed by refunding the full remaining renewal payment left that projection and boundary unchanged and produced no refund entries in the licensing event ledger. C8-09 is complete.
- The installed 1.4.1 candidate then completed an authoritative Pro-to-Free downgrade against the actual sandbox entitlement. The first refresh safely retained cached Pro and surfaced a licensing-service error because the service rejected `expired` before it could issue a signed downgrade state. The service was narrowed so only an already-active matching device can receive a signed `expired` claim while the trusted paid-through time is still in the future; expired accounts still cannot activate a new device or receive Pro. The corrected database rule and entitlement contract passed 66 local database assertions, 77 Vitest tests, and lint before sandbox deployment. After refresh, Settings showed Free and Expired while retaining the connected account and paid-through evidence. Post-transition editing of an existing task, local backup, and essential CSV export passed; the custom RFI PDF template control was disabled as a Pro-only capability. No project content was sent to the licensing or billing services. C8-11 is complete.

## Provisional 1.4.1 Early Access candidate — 2026-10-03

The first candidate built under ADR-011 is recorded in `C8-1.4.1-EARLY-ACCESS-CANDIDATE.md`. It was built from commit `a9ab136d43de9777dbf970c3b6ddef091baebb90`, passed the matching GitLab pipeline, passed the complete local gate, and produced SHA-256 `CAB68C74EDBE1F8C79780C709701ECA52FE1C711240D3CDE2D186E6062CA28A7`. Authenticode reports `NotSigned`, as the Early Access disclosure requires, and Microsoft Defender found no threats.

The exact copied artifact then passed manual acceptance in the dedicated `SiteDatumC8` profile. The guarded runner confirmed the expected hash and `NotSigned` state and correctly refused unattended execution because version 1.4.0 was already installed. A deliberate direct install upgraded that existing profile to 1.4.1. No SmartScreen prompt appeared on this test device; that observation does not predict warning or policy behavior on another computer. SiteDatum reopened directly into the existing fictional workspace, reported version 1.4.1, and retained both tasks after filters were cleared, the RFI, the submittal, and the registered file.

The neutral SiteDatum RFI PDF exported successfully without the removed company-specific template. While the sandbox Pro entitlement remained verified, a customer-owned one-page PDF was selected as the temporary custom template, validated without copying or modifying the source, and produced a readable custom-layout export. The setting was restored to the SiteDatum layout after the test. Uninstall left the fictional project folder and ordinary files intact; reinstalling the exact 1.4.1 candidate skipped first-run setup, reopened the existing workspace with all checked records, and retained the expected verified Pro entitlement.

The exact candidate is published at `https://sitedatum.site/downloads/SiteDatum_1.4.1_x64-setup.exe`. The canonical Early Access page presents the version, publication date, source commit, byte size, SHA-256 digest, `NotSigned` state, security warning, verification command, support route, and refund route before download. A cache-bypassed public-origin download returned 4,681,099 bytes and SHA-256 `CAB68C74EDBE1F8C79780C709701ECA52FE1C711240D3CDE2D186E6062CA28A7`; Windows reported `NotSigned`, matching the accepted candidate exactly.

The public copy was then installed from the shared C8 test location in the disposable `SiteDatumC8` profile without disabling security controls. No warning appeared on this test device, SiteDatum opened successfully, and the existing fictional workspace and records remained intact. The public home, Early Access, support, refund, and release-manifest paths all returned HTTP 200.

The initial Checkout disclosure implementation exposed a compatibility defect: Stripe Managed Payments rejects the `custom_text` parameter and the sandbox function returned HTTP 503 before session creation. The unsupported parameter was removed while preserving `managed_payments[enabled]=true`, covered by a focused regression test, and deployed as sandbox function version 6. Both sandbox products now identify themselves as `SiteDatum Pro — Unsigned Windows Early Access` and link to the canonical warning and refund terms. An independent sandbox API request created an open Managed Payments subscription Checkout Session, and the installed application opened Checkout showing the updated unsigned-product name and description. No payment was completed.

## Sandbox payment procedure

All billing work stays in the connected Stripe and Supabase sandboxes. Use fictional customer details and Stripe test payment methods. Confirm the Dashboard says sandbox/test mode before every mutation.

Evidence should record event IDs, subscription state transitions, HTTP delivery status, entitlement plan/state, paid-through behavior, and anonymized device counts. Do not paste secrets, payment details, passwords, project records, databases, or private customer information into the repository, GitLab, or Slack.

## C8-10 hosted device-allowance evidence — 2026-10-03

The linked `SiteDatum Licensing Test` Supabase sandbox ran `supabase/tests/c8_device_allowance.sql` through the hosted database boundary. The acceptance script uses one fictional Auth subject, a provider-neutral fictional Pro subscription, and deterministic disposable fingerprint hashes inside a single transaction.

- The first device activated and a repeat request reused the same device without consuming another slot.
- A distinct second device activated.
- A third active device was rejected with `LICENSING_DEVICE_LIMIT_REACHED`.
- Deactivating the first device removed its entitlement association.
- The previously rejected fingerprint then activated as the replacement, retained Pro entitlement, and restored the active count to exactly two.
- Every assertion is fail-closed through a raised database exception. The transaction was rolled back, and a separate read-only check confirmed zero matching rows in both `auth.users` and `licensing.customers`.

This closes the authoritative hosted allowance/replacement rule. A future native multi-profile exercise may supplement the evidence for Windows Credential Manager behavior, but it is not required to repeat the already verified server limit.

## Stop conditions

Stop the audit and preserve evidence if any test causes record loss, overwrites a file without confirmation, makes existing records uneditable after expiration, removes backup/export, accepts an unverified entitlement, contacts live Stripe mode, exposes a secret, or produces a migration/integrity failure. Follow `RELEASE_ROLLBACK_RUNBOOK.md`; do not improvise destructive database repair.

## Launch decision

C8 engineering acceptance for the controlled unsigned Early Access route is complete. The exact 1.4.1 public installer and its disclosure/support/refund journey are **GO** for the approved Early Access channel. This decision does not authorize live Stripe mode or a real charge; billing remains sandbox-only until a separate live-commerce decision and configuration are completed. Trusted general availability remains deferred until a signed route is separately completed. Deferred means unverified, not accepted.

Founder routine operations, incident severity, billing recovery, outage, security/privacy, communications, and launch hold points are defined in `FOUNDER-OPERATIONS-RUNBOOK.md`.

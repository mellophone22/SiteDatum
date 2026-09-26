# AnyDesk UX 1.1 — Final Verification

Date: 2026-09-25  
Branch: `codex/ux-1.1-project-centric-workspace`  
Candidate base HEAD: `bcaceee6afb0c6fe310c7d3717705e8fb7b0f087`  
Release commit: recorded by the `v1.1.0` tag after final validation.

## Release recommendation

**Hold for final manual sign-off.** The code, automated tests, optimized application binary, NSIS installer build, production startup, and offline-startup smoke pass. A prior `1.0.1` package also passed per-user installer execution; the final `1.1.0` package was built and checksummed but was not silently installed over that existing installation. No automated release blocker was found. Release should remain on hold until a human completes the native end-to-end workflow on a clean/disposable profile and checks every core screen at exact 200% Windows/WebView zoom.

This hold is a verification limitation, not a known product failure.

## Automated gates

| Gate | Result | Evidence |
| --- | --- | --- |
| Clean dependency install | Pass | `npm ci`: 161 packages installed, 0 vulnerabilities. The first attempt encountered a Vite native-binary lock; stopping the exact port-1420 Node process allowed the clean install. |
| Frontend lint | Pass | `npm run lint`: zero errors or warnings. |
| Frontend tests | Pass | `npm test -- --run`: 19 files, 48 tests. |
| TypeScript/Vite build | Pass | `npm run build`: 59 modules transformed; production assets emitted. |
| Rust formatting | Pass | `cargo fmt --all -- --check`. |
| Rust tests | Pass | `cargo test`: 27 unit tests and doc tests passed. |
| Diff validation | Pass | `git diff --check`; only expected LF-to-CRLF notices were reported. |
| Tauri optimized application | Pass | `src-tauri/target/release/AnyDesk.exe`, 11,671,552 bytes. |
| NSIS installer | Pass | `releases/1.1.0/AnyDesk_1.1.0_x64-setup.exe`, 4,759,028 bytes. |
| Installer SHA-256 | Recorded | `3A0448D68C9A4F511026934C05588FA41E29770AA1DD0B0572483BC6E285B020`. |
| Release executable | Pass | `releases/1.1.0/AnyDesk.exe`, 11,671,552 bytes. |
| Release executable SHA-256 | Recorded | `251CE1175C9D452A6951422EF7EF89C792EC21344CBC413BAD09BA9935F1CC63`. |
| Production startup smoke | Pass | Hidden launch remained alive after four seconds with no startup panic; the exact process was then stopped. |
| Offline startup smoke | Pass | Production binary remained alive with HTTP/HTTPS routed to an unreachable local proxy; cloud access was not required for startup. |

The initial sandboxed NSIS attempt built the optimized binary but could not extract verified NSIS tooling because of Windows access control. Re-running the same command with approved filesystem elevation completed `makensis` successfully.

## Architecture and state verification

Pure frontend tests cover:

- shared workspace reducer, persistence parsing, and stale-project reconciliation;
- project-context inheritance;
- detail-panel Escape behavior;
- task editor payloads and progress summaries;
- RFI and submittal lifecycle helpers;
- command-palette grouping, recent items, and typed results;
- project-control grouping;
- phase-aware module prioritization;
- project-scoped number suggestions;
- note-scope filtering;
- recovery-health summaries;
- first-run entry conditions;
- Quick Capture tab navigation;
- Home/project-attention summaries and saved task views.

The selected project remains explicit in the shell, create flows inherit only when a project is selected, and All Projects does not silently assign required project-owned records.

## Persistence, migration, and data integrity

Result: **Pass for this candidate.** UX 1.1 adds no schema migration. Task detail editing uses the existing `description` and `category` columns. Rust remains the only SQLite/filesystem owner.

Evidence:

- migrations 1–10 remain forward-only and transactional through `schema_migrations`;
- every database connection enables foreign keys;
- recovery tests reopen a restored database, reapply current migrations, and confirm foreign keys remain enabled;
- task update tests verify atomic persistence, activity history, and rejection of project changes that would invalidate RFI/submittal relationships;
- cloud snapshot tests verify metadata round-trip and project-path rebasing;
- realistic task-volume tests list records without data loss.

## Domain workflow evidence

| Area | Verification |
| --- | --- |
| Tasks | Create, update, Waiting requirements, follow-up dates, completion/attention grouping, activity history, and realistic volume are covered by Rust and frontend tests. |
| RFIs | Lifecycle requirements, persistence, attention, task relationships, attachment references, PDF generation, and refusal to overwrite are covered by Rust tests. |
| Submittals | Lifecycle/disposition requirements, persistence, revision parent relationship, related tasks, and attachments are covered by Rust tests. |
| Project Controls | Register validation, bulk updates, templates, CSV/XLSX import/export, and reporting paths remain exercised by Rust tests. |
| Backup/restore | Backup inventory/preview UI remains present; restore tests verify safety backup behavior, migration reopening, counts, and database guards. |
| Optional sync | Sign-in remains optional; credential storage test targets the Windows native keyring; sync snapshot and conflict code are unchanged by schema migration. |

## Filesystem safety

Result: **Pass in automated coverage; native UI scenario still requires manual sign-off.**

- Register, Copy, Move, and collision protection are covered by `file_record::tests::copy_move_register_and_collision_are_explicit`.
- Project and report/PDF generation refuse overwrite.
- The shared confirmation dialog shows Move source and destination before invoking Rust.
- Removing file or attachment references explicitly states that the physical file is unchanged.
- React has no unrestricted filesystem or SQLite access.

## Accessibility and UX

Result: **Pass for code and browser semantic review; exact native zoom sweep pending.**

Verified:

- visible focus styles for buttons, fields, links, and disclosure summaries;
- skip link to the main workspace;
- `aria-current` on active navigation;
- named dialogs with modal semantics;
- confirmation and search/Quick Capture focus trapping and restoration;
- Escape behavior;
- valid search listbox/option ownership;
- Quick Capture Arrow/Home/End tab navigation;
- non-color text/semantic selected, archived, missing, warning, and recovery states;
- announced loading, success, and error states;
- reduced-motion overrides;
- responsive CSS that collapses navigation, stacks forms, bounds dialogs, and retains horizontal table scrolling at the effective narrow widths associated with 200% zoom.

Browser previews were reviewed throughout WP02–WP19. The browser-only surface cannot execute Tauri commands, so its visible native-invoke error is expected and is not present in the production WebView.

## First run and offline behavior

First-run entry is unit tested: it appears only when both the project root is absent and the project count is zero. The flow reuses Rust-owned root validation, folder preview, and collision-safe project creation; Set up later routes to Settings; cloud sync is outside the gate.

The production binary starts with network requests forced to an unreachable proxy. Full offline CRUD through the native UI remains part of manual sign-off.

## Performance

No new unbounded polling or render loop was introduced. Search grouping, filters, summaries, phase mapping, and reducers are pure/tested. Rust includes realistic task-volume coverage. The requested full mixed dataset (50 projects, 500 files, 100 RFIs, 100 submittals, and 1,000 activity events) was not generated in the user's real profile and remains a manual/disposable-profile test.

## Manual end-to-end scenario

Status: **Partially complete — native interaction remains pending.**

The prior `1.0.1` NSIS package was installed successfully for the current Windows user. The final `1.1.0` executable passed connected and forced-offline startup smokes, but its installer was not silently applied over the existing installation. This Codex environment can inspect browser previews and launch the release executable, but native-app UI control is disabled. It therefore cannot truthfully complete the handoff's 24-step native scenario or visually verify the installer UI. Automated domain tests cover the underlying persistence, lifecycle, filesystem, export, backup, recovery, and sync invariants, but they are not a substitute for final native interaction sign-off.

Before release, complete on a disposable Windows profile:

1. On a clean/disposable profile, interactively install `AnyDesk_1.1.0_x64-setup.exe` and launch it.
2. Run first setup, create a project, restart, and confirm context persistence.
3. Execute the handoff's Task → Waiting → RFI → Submittal → Register/Copy/Move → Project Controls export → Backup scenario.
4. Disconnect networking and repeat representative local CRUD/search/backup actions.
5. Check Home, Projects, Tasks, RFIs, Submittals, Files, Project Controls, and Settings at exact 200% zoom.
6. Confirm keyboard-only Search, Quick Capture, Task detail, Settings, and confirmation cancellation.

## Known limitations and deferred items

- The release commit and `v1.1.0` tag identify the final candidate in Git.
- The generated installer is unsigned; Windows SmartScreen may warn, as documented in the README.
- Activity events and contacts remain workspace-wide because their schema does not carry project relationships.
- Project-scoped activity and project-contact relationships were deliberately not fabricated.
- Exact native 200% zoom, clean-profile interactive installer execution, mixed large-data performance, and the full manual release scenario await human sign-off.
- Cloud sync is optional and was not exercised against a live Supabase account during this local release pass.

## No-go audit

No evidence was found of data loss, migration failure, silent filesystem overwrite, incorrect project inheritance, broken automated lifecycle behavior, backup/restore regression, production compilation failure, or test failure. The release hold is solely for the required manual checks listed above.

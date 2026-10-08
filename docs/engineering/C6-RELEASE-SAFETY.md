# C6 — Release safety foundation

Status: completed for controlled unsigned Early Access; trusted signed general availability deferred

## Delivered boundary

The release-safety boundary includes these controls without changing licensing policy, subscription behavior, or the rendered interface:

- the Tauri webview now uses an explicit restrictive Content Security Policy; and
- GitHub Actions now runs the frontend, Rust, and disposable Supabase quality gates; GitHub repository-native secret scanning protects the public source boundary.

Migration recovery now creates and validates an app-local SQLite snapshot before an existing older database is upgraded. Each migration remains transactional, and an older executable refuses a database whose recorded schema is newer than it supports. The release and rollback procedures are recorded in `RELEASE_ROLLBACK_RUNBOOK.md`; the architecture boundary is recorded in `ADR-007-MIGRATION-RECOVERY.md`.

The policy permits only packaged application resources, Tauri IPC, and Tauri's asset protocol. It denies embedded frames, form submission, plugins, inline styles, inline scripts, and arbitrary network connections. Stripe Checkout and the customer portal continue to open in the system browser through the separately allowlisted Tauri opener capability. Supabase licensing requests remain Rust-owned and are not webview connections.

## Continuous integration

The GitHub Actions frontend job uses the documented Node.js 24 baseline and performs a clean locked install, high-severity dependency audit, ESLint, the Vitest suite, release-provenance unit tests, commercial-site validation, and the TypeScript/Vite production build. The Rust job uses the documented Rust 1.97 baseline, installs `rustfmt` plus only the Linux libraries needed to compile Tauri on an ephemeral Ubuntu runner, audits the Rust lockfile, checks formatting, and runs the locked portable Rust test suite. A Windows-hosted job runs the locked native Rust suite, builds the frontend and a CI-only unsigned NSIS package, smoke-tests the release executable, and verifies a staged manifest against the package's actual bytes and Authenticode status. The CI-only installer is not uploaded or published and contains no production licensing coordinates. Superseded workflow runs are interruptible.

The Supabase pgTAP suite runs on an ephemeral GitHub-hosted Ubuntu runner using the runner's Docker daemon. The job receives no production credentials, linked hosted database, billing secret, signing key, or customer data. The repository-pinned Supabase CLI starts disposable local Postgres, applies repository migrations, runs both licensing and Stripe adapter test files, and removes the local volumes afterward. A persistent self-managed privileged runner remains rejected. See `ADR-012-GITHUB-ACTIONS-CI.md`; `ADR-008-SUPABASE-CI-RUNNER.md` remains the historical GitLab decision for earlier evidence.

## Windows distribution boundary

`ADR-011-CONTROLLED-UNSIGNED-EARLY-ACCESS.md` authorizes a narrowly scoped direct-download Early Access channel for the verified NSIS package. The channel must disclose before purchase and download that the installer is unsigned, that Windows may warn or block it, and that managed devices may not permit installation. It must never be described as trusted, signed, Microsoft-certified, or generally available. Support must not instruct a customer to disable or weaken Windows or organizational security controls.

Every published Early Access installer must be byte-for-byte identical to a candidate that passed the complete local C8 gate and the matching hosted GitHub Actions workflow. The canonical HTTPS release page must publish its version, source commit, SHA-256 digest, Authenticode status, and publication date. A rebuild or repackaging is a new candidate. Automatic application updating remains disabled for this channel; upgrades are manually obtained as separately verified installers.

`ADR-010-MICROSOFT-STORE-MSIX-DISTRIBUTION.md` remains the preferred future trusted general-availability route. Microsoft would sign a certified MSIX and provide Store updates without a recurring external code-signing charge. That later route does not change SiteDatum's local-first data boundary.

MSIX packaging will use Microsoft's `winapp` CLI after Partner Center assigns SiteDatum's exact case-sensitive package identity and publisher values. Those values must be copied from the reserved Store product; the repository will not contain a fabricated production identity. Local package tests may use a self-signed development certificate only on disposable operator-controlled machines.

The founder does not currently qualify for the intended Microsoft Store Company-account path and will not use an unsuitable Individual-account classification. No Store identity, Azure signing resource, production certificate, or signing expense has been created. This does not prevent the controlled unsigned Early Access release, whose immutable artifact metadata, Checkout disclosure, refund route, and public-origin download/install acceptance were completed on 2026-10-03.

## Dependency advisory policy

The frontend quality job runs `npm audit --audit-level=high` against the committed lockfile after a clean install. This checks runtime and build dependencies and fails the pipeline for high or critical npm advisories. Lower-severity findings remain visible for maintenance review rather than silently disappearing.

The Rust quality job installs the explicitly pinned `cargo-audit 0.22.2` tool with its own locked dependency graph, caches the executable, and checks `src-tauri/Cargo.lock` against the current RustSec advisory database. RustSec vulnerability advisories fail the job. Informational warnings such as unmaintained, unsound, or yanked transitive crates remain visible and require review, but do not automatically fail a release unless they are also classified as vulnerabilities or a direct impact is established.

GitHub's repository-native secret scanning and push protection remain separate from the Actions workflow. These advisory checks do not upload SiteDatum project records or customer data; they submit dependency names and versions to the relevant public advisory services.

## Verification

Completed locally on Windows:

- Tauri configuration JSON parsing and release configuration validation: pass;
- frontend ESLint: pass;
- Vitest: 24 files and 66 tests passed;
- TypeScript/Vite production build: pass;
- Rust formatting: pass;
- full Windows Rust suite: 40 tests passed;
- the frontend job in its declared `node:24-bookworm` container: pass;
- the portable Rust job in its declared `rust:1.97-bookworm` container: 38 tests passed;
- production Tauri build and NSIS bundle generation: pass; and
- five-second production-binary startup smoke: pass, with only the launched process stopped afterward.

Native installer acceptance on 2026-10-02:

- the locally built `SiteDatum_1.4.0_x64-setup.exe` completed a silent current-user installation with exit code `0`;
- the installed executable was present at `%LOCALAPPDATA%\SiteDatum\SiteDatum.exe` with file version `1.4.0`; and
- the installed executable opened a native window titled `SiteDatum` and remained running after a five-second startup observation.

Manual native Windows display-scaling acceptance was completed on 2026-10-02 at 200% scaling. The installed application remained usable without reported clipping, overlap, inaccessible controls, or unusable dialogs. This closes the native Windows acceptance and 200% scaling gate.

Dependency advisory baseline on 2026-10-01:

- complete npm dependency tree: zero known vulnerabilities;
- RustSec vulnerability scan across 581 locked crates: zero vulnerabilities; and
- RustSec maintenance warnings: `proc-macro-error` is unmaintained, `glib 0.18.5` has an unsound iterator advisory, and `yoke-derive 0.8.3` is yanked. These are transitive dependencies and remain recorded for upgrade tracking.

No application visual comparison is required because the rendered desktop UI is unchanged. GitHub Actions must still complete successfully for the pushed commit before this slice is considered hosted-verified.

## Remaining C6 work

Completed for unsigned Early Access:

- the exact 1.4.1 candidate is identified by source commit, size, SHA-256, and `NotSigned` Authenticode state;
- the canonical public page carries the real artifact metadata and HTTPS destination;
- the actual pre-purchase and pre-download journey presents the required unsigned Early Access disclosure;
- the public-origin download, install, upgrade/reinstall preservation, and refund/support journey passed; and
- the release controls continue to fail closed whenever an artifact, digest, disclosure, or matching CI record is missing.

For later trusted general availability:

- establish a qualified publisher/signing route;
- add the assigned package identity and packaging workflow for that route;
- complete its certification and native acceptance checks; and
- publish only the immutable signed artifact approved by that channel.

# C6 — Release safety foundation

Status: engineering safety foundation implemented; controlled unsigned Early Access route approved but not yet opened

## Delivered boundary

The release-safety boundary includes these controls without changing licensing policy, subscription behavior, or the rendered interface:

- the Tauri webview now uses an explicit restrictive Content Security Policy; and
- GitLab CI now runs the existing frontend and Rust quality gates before the existing secret-detection stage.

Migration recovery now creates and validates an app-local SQLite snapshot before an existing older database is upgraded. Each migration remains transactional, and an older executable refuses a database whose recorded schema is newer than it supports. The release and rollback procedures are recorded in `RELEASE_ROLLBACK_RUNBOOK.md`; the architecture boundary is recorded in `ADR-007-MIGRATION-RECOVERY.md`.

The policy permits only packaged application resources, Tauri IPC, and Tauri's asset protocol. It denies embedded frames, form submission, plugins, inline styles, inline scripts, and arbitrary network connections. Stripe Checkout and the customer portal continue to open in the system browser through the separately allowlisted Tauri opener capability. Supabase licensing requests remain Rust-owned and are not webview connections.

## Continuous integration

The frontend job uses the documented Node.js 24 baseline and performs a clean locked install, ESLint, the Vitest suite, and the TypeScript/Vite production build. The Rust job uses the documented Rust 1.97 baseline, installs `rustfmt` plus only the Linux libraries needed to compile Tauri on a GitLab Docker runner, checks formatting, and runs the locked portable Rust test suite. The existing project-root acceptance test is explicitly Windows-only because it validates a real Windows temporary path; it remains part of the full local Windows suite. Both jobs use lockfile-keyed caches and are interruptible. GitLab secret detection remains enabled as a separate stage.

The Supabase pgTAP suite runs on GitLab.com's ephemeral `saas-linux-small-amd64` hosted runner. Its TLS-protected Docker-in-Docker service is isolated to the newly provisioned job VM and receives no production credentials, linked hosted database, billing secret, signing key, or customer data. The repository-pinned Supabase CLI starts disposable local Postgres, applies repository migrations, runs both licensing and Stripe adapter test files, and removes the local volumes afterward. A persistent self-managed privileged runner is explicitly rejected in `ADR-008-SUPABASE-CI-RUNNER.md`.

## Windows distribution boundary

`ADR-011-CONTROLLED-UNSIGNED-EARLY-ACCESS.md` authorizes a narrowly scoped direct-download Early Access channel for the verified NSIS package. The channel must disclose before purchase and download that the installer is unsigned, that Windows may warn or block it, and that managed devices may not permit installation. It must never be described as trusted, signed, Microsoft-certified, or generally available. Support must not instruct a customer to disable or weaken Windows or organizational security controls.

Every published Early Access installer must be byte-for-byte identical to a candidate that passed the complete local C8 gate and the matching GitLab pipeline. The canonical HTTPS release page must publish its version, source commit, SHA-256 digest, Authenticode status, and publication date. A rebuild or repackaging is a new candidate. Automatic application updating remains disabled for this channel; upgrades are manually obtained as separately verified installers.

`ADR-010-MICROSOFT-STORE-MSIX-DISTRIBUTION.md` remains the preferred future trusted general-availability route. Microsoft would sign a certified MSIX and provide Store updates without a recurring external code-signing charge. That later route does not change SiteDatum's local-first data boundary.

MSIX packaging will use Microsoft's `winapp` CLI after Partner Center assigns SiteDatum's exact case-sensitive package identity and publisher values. Those values must be copied from the reserved Store product; the repository will not contain a fabricated production identity. Local package tests may use a self-signed development certificate only on disposable operator-controlled machines.

The founder does not currently qualify for the intended Microsoft Store Company-account path and will not use an unsuitable Individual-account classification. No Store identity, Azure signing resource, production certificate, or signing expense has been created. This no longer prevents a controlled unsigned Early Access release, but purchase and download remain closed until the real immutable artifact metadata, checkout disclosure, refund route, and public-origin download/install acceptance are complete.

## Dependency advisory policy

The frontend quality job runs `npm audit --audit-level=high` against the committed lockfile after a clean install. This checks runtime and build dependencies and fails the pipeline for high or critical npm advisories. Lower-severity findings remain visible for maintenance review rather than silently disappearing.

The Rust quality job installs the explicitly pinned `cargo-audit 0.22.2` tool with its own locked dependency graph, caches the executable, and checks `src-tauri/Cargo.lock` against the current RustSec advisory database. RustSec vulnerability advisories fail the job. Informational warnings such as unmaintained, unsound, or yanked transitive crates remain visible and require review, but do not automatically fail a release unless they are also classified as vulnerabilities or a direct impact is established.

GitLab's existing secret-detection stage remains separate. These advisory checks do not upload SiteDatum project records or customer data; they submit dependency names and versions to the relevant public advisory services.

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

No visual comparison is required because the rendered UI is unchanged. GitLab must still run the new pipeline successfully after the changes are pushed before this slice is considered hosted-verified.

## Remaining C6 work

For unsigned Early Access:

- produce the final immutable candidate from a clean tagged commit and record its SHA-256 and `NotSigned` Authenticode state;
- add the real artifact metadata and destination to the canonical public release page;
- place the required disclosure in the actual pre-purchase and pre-download journey;
- test the public-origin download, Windows warning or block behavior, install, upgrade, rollback, and refund/support path; and
- keep the download closed whenever the artifact, digest, disclosure, or matching CI evidence is missing.

For later trusted general availability:

- establish a qualified publisher/signing route;
- add the assigned package identity and packaging workflow for that route;
- complete its certification and native acceptance checks; and
- publish only the immutable signed artifact approved by that channel.

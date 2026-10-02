# C6 — Release safety foundation

Status: restrictive CSP, core CI, and dependency advisory gates implemented

## Delivered boundary

This slice adds two release-safety controls without changing SiteDatum data, licensing policy, subscription behavior, or rendered interface:

- the Tauri webview now uses an explicit restrictive Content Security Policy; and
- GitLab CI now runs the existing frontend and Rust quality gates before the existing secret-detection stage.

The policy permits only packaged application resources, Tauri IPC, and Tauri's asset protocol. It denies embedded frames, form submission, plugins, inline styles, inline scripts, and arbitrary network connections. Stripe Checkout and the customer portal continue to open in the system browser through the separately allowlisted Tauri opener capability. Supabase licensing requests remain Rust-owned and are not webview connections.

## Continuous integration

The frontend job uses the documented Node.js 24 baseline and performs a clean locked install, ESLint, the Vitest suite, and the TypeScript/Vite production build. The Rust job uses the documented Rust 1.97 baseline, installs `rustfmt` plus only the Linux libraries needed to compile Tauri on a GitLab Docker runner, checks formatting, and runs the locked portable Rust test suite. The existing project-root acceptance test is explicitly Windows-only because it validates a real Windows temporary path; it remains part of the full local Windows suite. Both jobs use lockfile-keyed caches and are interruptible. GitLab secret detection remains enabled as a separate stage.

The hosted Supabase pgTAP suite is intentionally not placed in this first CI slice. It requires a privileged Docker runner and isolated test-service configuration; adding it without confirming runner capabilities would leave ordinary pipelines pending or unsafe. Database tests remain a required local release gate until that runner boundary is implemented.

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

- configure Windows Authenticode and Tauri updater signing with founder-controlled private-key custody;
- publish signed update manifests and immutable release artifacts;
- add migration-time recovery coverage and finalize the release/rollback runbook; and
- decide whether to provision a privileged GitLab runner for the local Supabase pgTAP suite.

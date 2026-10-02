# C6 — Release safety foundation

Status: first vertical slice implemented locally; GitLab pipeline execution pending

## Delivered boundary

This slice adds two release-safety controls without changing SiteDatum data, licensing policy, subscription behavior, or rendered interface:

- the Tauri webview now uses an explicit restrictive Content Security Policy; and
- GitLab CI now runs the existing frontend and Rust quality gates before the existing secret-detection stage.

The policy permits only packaged application resources, Tauri IPC, and Tauri's asset protocol. It denies embedded frames, form submission, plugins, inline styles, inline scripts, and arbitrary network connections. Stripe Checkout and the customer portal continue to open in the system browser through the separately allowlisted Tauri opener capability. Supabase licensing requests remain Rust-owned and are not webview connections.

## Continuous integration

The frontend job uses the documented Node.js 24 baseline and performs a clean locked install, ESLint, the Vitest suite, and the TypeScript/Vite production build. The Rust job uses the documented Rust 1.97 baseline, installs `rustfmt` plus only the Linux libraries needed to compile Tauri on a GitLab Docker runner, checks formatting, and runs the locked portable Rust test suite. The existing project-root acceptance test is explicitly Windows-only because it validates a real Windows temporary path; it remains part of the full local Windows suite. Both jobs use lockfile-keyed caches and are interruptible. GitLab secret detection remains enabled as a separate stage.

The hosted Supabase pgTAP suite is intentionally not placed in this first CI slice. It requires a privileged Docker runner and isolated test-service configuration; adding it without confirming runner capabilities would leave ordinary pipelines pending or unsafe. Database tests remain a required local release gate until that runner boundary is implemented.

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

No visual comparison is required because the rendered UI is unchanged. GitLab must still run the new pipeline successfully after the changes are pushed before this slice is considered hosted-verified.

## Remaining C6 work

- configure Windows Authenticode and Tauri updater signing with founder-controlled private-key custody;
- publish signed update manifests and immutable release artifacts;
- add migration-time recovery coverage and finalize the release/rollback runbook;
- add dependency/security audit gates with an explicit maintenance policy;
- decide whether to provision a privileged GitLab runner for the local Supabase pgTAP suite; and
- complete the native Windows acceptance and 200% zoom review recorded in the release documentation.

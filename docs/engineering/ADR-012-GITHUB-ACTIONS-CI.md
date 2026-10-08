# ADR-012 — GitHub Actions continuous integration

Status: Accepted
Date: 2026-10-05

## Context

SiteDatum's public source of record moved to `github.com/mellophone22/SiteDatum`. The existing GitLab pipeline established three required hosted gates: frontend quality, Rust quality, and disposable Supabase database tests. Historical GitLab pipeline evidence remains valid for the commits that produced it, but current changes need equivalent verification on GitHub.

## Decision

1. `.github/workflows/ci.yml` runs on `main`, `codex/**` branches, pull requests, and manual dispatch.
2. The frontend job uses Node.js 24 and runs the retired-template checks, locked dependency installation, high-severity npm audit, lint, Vitest, release-provenance unit tests, commercial-site validation, and the production frontend build.
3. The Rust job uses Rust 1.97, installs the Linux libraries required to compile Tauri, pins `cargo-audit` 0.22.2, audits the lockfile, checks formatting, and runs locked Rust tests.
4. The licensing-database job uses GitHub's ephemeral Ubuntu runner and its Docker daemon to start the repository-pinned local Supabase stack. It receives no hosted Supabase coordinates, provider credentials, customer records, or production secrets.
5. Workflow permissions are read-only, concurrent superseded runs are cancelled, and no release, deployment, billing, signing, or provider mutation occurs in CI.
6. GitHub's repository-native secret scanning and push protection are the preferred secret boundary for the public repository; secrets are never supplied merely to make a hosted quality job pass.
7. A separate `windows-latest` job runs the locked Rust suite on the shipped operating system, builds the frontend and a CI-only unsigned NSIS package, performs a bounded startup smoke, generates a staged release manifest, and independently verifies the source/evidence commits, synchronized version, filename, size, SHA-256 digest, and actual Authenticode state. The job proves the checkout is clean immediately before packaging. Because Tauri's Windows bundler may rewrite `Cargo.toml` while applying bundle metadata, manifest generation runs in a fresh detached worktree at the exact workflow commit with a copied installer; the verifier therefore never waives the clean-source requirement or attests from the bundler-mutated checkout. The manifest must remain unpublished and the job does not upload or release the installer.

## Consequences

- GitHub Actions becomes the current hosted quality gate while historical GitLab evidence remains unchanged.
- Standard GitHub-hosted runners are used for this public repository, avoiding a persistent privileged runner and additional infrastructure cost.
- The Supabase job still requires Docker and therefore remains isolated to an ephemeral hosted runner.
- Release publication continues to require the complete local release gate and a successful hosted run for the exact candidate commit.
- The Windows CI package proves packaging and provenance mechanics but is not a production-commerce candidate: it is built without the `commercial-production` feature or production licensing coordinates and is discarded with the ephemeral runner.

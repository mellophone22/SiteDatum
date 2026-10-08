# ADR-018 - Windows CI and release provenance

**Status:** Accepted; local validation complete, hosted verification pending push
**Date:** 2026-10-08

## Context

SiteDatum ships as a Windows desktop application, but the primary hosted pipeline previously compiled and tested Rust only on Linux. Local Windows acceptance remained valuable but did not prove that each branch could compile, package, and start on a clean hosted Windows machine. Release-manifest tooling generated immutable artifact metadata, but its tests were not part of hosted CI and no independent verifier compared a manifest with the actual installer bytes.

## Decision

GitHub Actions adds a Windows-native quality job on the same push, pull-request, and manual triggers as the existing pipeline. The job:

1. installs the pinned Node.js 24 and Rust 1.97 toolchains;
2. performs a locked dependency install;
3. runs the release-provenance unit suite;
4. runs the complete locked Rust suite on Windows;
5. builds the production frontend and a CI-only unsigned NSIS package;
6. starts the release executable, requires it to remain alive through a five-second observation, and stops only that process;
7. generates a staged manifest from the clean checked-out commit;
8. independently recalculates the installer size and SHA-256, resolves the source and evidence commits, checks synchronized versions and the exact filename, and reads the actual Windows Authenticode state; and
9. fails if the CI manifest contains a publication date or download URL.

The frontend quality jobs on GitHub and GitLab also run the release-provenance unit suite so malformed schema, commit, digest, signature, or publication-state changes fail before packaging.

## Security and release boundary

- The Windows CI job does not enable `commercial-production` and receives no production licensing coordinates, billing credentials, signing keys, customer data, or project records.
- Its NSIS package is a disposable packaging proof. It is neither uploaded nor published and must not be offered to customers.
- The workflow retains read-only repository permissions and creates no release, tag, deployment, attestation, or hosted-service mutation.
- The controlled unsigned Early Access release process still requires a separately identified clean production-commerce candidate, complete local acceptance, matching hosted verification for its exact commit, and immutable public artifact metadata.
- A future signed channel must verify `Valid` Authenticode state and follow its separately approved signing custody decision; this job does not simulate trust by relabeling an unsigned file.

## Consequences

Every proposed change now exercises native Windows compilation, tests, packaging, startup, and provenance mechanics on an ephemeral hosted runner. Hosted Windows execution adds CI time but no persistent runner or new service. The runner discards the installer and staged manifest at job completion, preventing ordinary branch CI from becoming an accidental distribution channel.

Local completion does not claim that GitHub has run the new job. The first pushed commit must complete all GitHub jobs successfully before this package becomes hosted-verified.

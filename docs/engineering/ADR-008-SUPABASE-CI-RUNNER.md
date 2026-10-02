# ADR-008 — Supabase database CI runner boundary

## Status

Accepted on 2026-10-02.

## Context

The licensing and Stripe adapter pgTAP suites require the Supabase CLI and a Docker-compatible runtime. Running Docker-in-Docker requires privileged execution. A persistent self-managed privileged runner would let repository jobs reach a long-lived runner host with effectively disabled container isolation, which is inappropriate for this public repository.

Supabase documents local database testing in CI through its CLI and Docker stack. GitLab documents Docker-in-Docker with TLS on GitLab.com hosted runners and states that each hosted job receives a newly provisioned VM dedicated to that job.

## Decision

1. Database tests run on GitLab.com's ephemeral `saas-linux-small-amd64` hosted runner, not a self-managed runner.
2. The job uses Docker-in-Docker with TLS and an explicitly pinned Docker service image.
3. The repository-pinned Supabase CLI is installed by `npm ci`; CI does not fetch an unpinned CLI release.
4. The job starts only the local Postgres development service, applies repository migrations, and runs both pgTAP files.
5. The job receives no Supabase access token, hosted project reference, database password, Stripe secret, signing key, or customer data.
6. The disposable local stack is stopped and its volumes are removed after the job. No production or shared database is contacted.
7. A persistent self-managed privileged runner remains prohibited unless a future decision documents dedicated single-use infrastructure, isolation, ownership, patching, and teardown.

## Consequences

- Every pipeline verifies the licensing and Stripe projection schema alongside frontend and Rust quality gates.
- Database tests consume additional GitLab-hosted compute and Docker image bandwidth.
- A GitLab hosted-runner outage may delay verification, but it cannot weaken the required database gate.
- Hosted deployment, production migration, and secret-bearing integration tests remain separate explicitly authorized work.


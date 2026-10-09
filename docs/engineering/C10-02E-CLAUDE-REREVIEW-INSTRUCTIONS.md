# C10-02E Claude re-review instructions

**Remediation source commit:**
`2009675b683a082ebf62e2b01301a240c7b7321f`

**Source-only archive:**
`review-packages/SiteDatum-C10-02E-remediation-source-2009675.zip`

**Archive SHA-256:**
`CDE9F8490309C6A314790CED745DB05A5786527F54AE67C0DE2255AB1957BCED`

The archive was produced with `git archive` from the exact remediation source
commit. It contains the complete tracked application source, Rust migrations,
proofs, test sources, workflows, website source, and engineering documents. It
intentionally excludes historical release binaries, promotional media, and
earlier review archives because none is part of the remediation source under
review.

## Prompt to give Claude

> Perform a critical independent re-review of SiteDatum C10-02E using the
> immutable GitHub commit
> `2009675b683a082ebf62e2b01301a240c7b7321f` on branch
> `codex/c10-02e-claude-review`. If you use the attached archive, first verify
> its SHA-256 is
> `CDE9F8490309C6A314790CED745DB05A5786527F54AE67C0DE2255AB1957BCED`.
> Read `docs/engineering/C10-02E-INDEPENDENT-REVIEW-REMEDIATION.md`, but verify
> every claim against code and tests rather than trusting the document.
>
> Re-test the two blocking findings from your 2026-10-08 report. For H-1,
> determine whether authenticated HPKE, independently retained source identity
> and enrollment context, secret-bound comparison, and substitution/replay
> negatives close the device-transfer attack. For H-2, trace every legacy Sync
> command and determine whether the centralized `MetadataSync` boundary denies
> Precommercial, Free, and Pro access, credentials cannot authorize Sync, the
> availability getter is read-only, disconnect is cleanup-only, and retired
> live coordinates are absent from production source.
>
> Also verify the claimed resolutions for M-1, M-2, M-8, M-9, F-1, F-11,
> F-12, F-13, and F-14. Attempt practical bypasses, inspect direct callers and
> production reachability, and distinguish implemented controls from items
> explicitly deferred to C10-03 through C10-07. Treat passing tests as evidence,
> not proof. Flag any contradiction between source, tests, and documentation.
>
> Return: (1) findings ordered by severity with exact file/line evidence and an
> attack or failure scenario; (2) a finding-by-finding disposition of the prior
> report; (3) residual risks and later-gate conditions; and (4) one explicit
> decision: `APPROVED FOR C10-03`, `CONDITIONALLY APPROVED FOR C10-03`, or
> `NOT APPROVED FOR C10-03`. Do not describe this AI review as a professional
> security certification, penetration test, or formal cryptographic audit.

## Gate handling

Sync remains disabled while this review is pending. An approval permits work on
C10-03; it does not enable Sync, approve production deployment, or waive the
later consent, retention/deletion, recovery, beta, and operational gates.

# C10-04D D1 — read-only census and authority contract proof

Date: 2026-10-11. Scope: fictional, test-only proof; no production retirement mechanism.

## What is implemented

The native proof module is registered under `#[cfg(test)]`, with its own inner test-only boundary. It enumerates the twelve ciphertext columns from existing local migrations, including four nullable columns and all five resolved/unresolved conflict envelopes. A caller-created query-only SQLite fixture and read transaction scan those cells without returning plaintext, keys or a pruning capability. Exact scope/version and existing record/checkpoint authentication are checked. Unknown BLOB columns, renamed columns or changed nullability refuse.

The proof bounds cells at 1 MiB and rows at 10,000; these are harness limits, not approved production budgets. Oversized BLOBs are not copied into a Rust buffer. Checkpoint validation uses a caller-supplied known fictional manifest: historical manifest discovery is not proved.

The standalone sealed contract defines distinct `ReadKeys` and `WriteKeys` wrappers with no type implementing both. A composite-reader contract cannot be passed to a writer and a writer cannot be passed to a reader; a positive control compiles. The historical adapter used with existing `WorkspaceKeys` returns `None` for write authority. The active writer selects only the active version.

Source: `src-tauri/src/sync_v2_retirement_census_proof.rs`; contract/compile fixtures: `proofs/sync-v2-retirement-d1/`. Existing production `WorkspaceKeys`, ring capacity, hosted SQL and migrations are unchanged.

## Executed tests

Twelve focused tests passed:

1. All twelve columns and resolved/unresolved conflict rows counted; query-only, unchanged SQLite change counter and original pull-history bytes.
2. All four nullable columns counted correctly.
3. Every column rejects malformed encoding without ring mutation.
4. Every column rejects correctly framed tampered ciphertext.
5. Wrong scope/key and oversized input refuse.
6. Unknown BLOB, renamed-column and nullability schema drift refuse.
7. Missing exact key version and incorrect checkpoint manifest refuse.
8. Query-only requirement and row-budget exhaustion refuse.
9. A newly acquired historical dependency increases the diagnostic census; no key is removed and no authority is issued.
10. Historical reads succeed, write authority is absent, and the existing queue refuses the historical resolver without SQL mutation.
11. Both role-crossing compile fixtures fail with E0277 for the intended trait; the positive control compiles.
12. Existing hosted push source retains advisory-lock-before-idempotency-before-row-lock ordering.

Commands: `cargo test --locked sync_v2_retirement_census_proof --quiet`, `cargo test --locked --quiet`, `cargo check --locked`, `cargo fmt --check`; frontend `npm test -- --run`, `npm run lint`, `npm run build`. Results: 12 focused passed; full Rust 214 passed, zero failed, five ignored; frontend 113 passed across 35 files; check, formatting, lint and build passed. Compile fixtures emit metadata only under unique fictional temporary directories; no credentials or customer files are accessed.

An initial fixture assertion used the wrong SQL function-declaration spelling; it was corrected. The first full regression exposed proof-code placement outside the existing source-containment test's recognized inner test boundary. The proof was moved inside that boundary; the containment test itself was not weakened or edited. Final focused and full regression passed.

## Not proved

No production read/write API migration, vault, archive format, manifest discovery, historical backup census, hosted floor, trusted roster, distributed barrier, actual contention, Windows storage budget or total-loss drill. The fixture fills all ciphertext stores together to exercise coverage and is not a valid production rotation state. A valid producer-signed checkpoint plus a known manifest is not independent freshness evidence.

D1 is complete as this bounded read-only proof only. Preservation completeness is a proposed distributed safety property, not established by the local diagnostic test. D2-D5 remain specification/measurement/model work; D6 and D7 stay blocked. No key is pruned, no schema or service changed, and Sync remains inactive.

# Security Hardening Review: SiteDatum C10-04D

We are designing preservation-first active-ring compaction, not implementing key destruction. The founder's five policies remain settled. Claude's 2026-10-11 review approves a design direction only; it does not approve D6 mutation or D7 activation.

## Evidence Basis

The original production source base is `097e8daa4459c03655dc5046372aefca0c138d97`; the reviewed packet commit is `8be1f5e8b415b62f9b696adb66a96d7d5e758b7c`. The [context](context.md) preserves the original LF/Git-blob integrity inventory. This follow-up adds only fictional test proofs and documentation corrections; existing production interfaces and hosted SQL remain unchanged. The [remediation ledger](review-remediation.md) separates completed corrections from outstanding mechanism proofs. Supplemental AI review is not security certification.

## Constraints

Every active device participates; offline age never excludes one. Historical backups and recovery files remain untouched. Each event requires explicit consent. Membership needs independently anchored owner-approved history. Ordinary local work remains accountless and available. No schema migration, vault, new recovery format, key removal, deployment, release, main merge or Sync activation is included.

## Opportunity Portfolio

| Opportunity | Evidence | Options | Recommendation | Proposal |
| --- | --- | --- | --- | --- |
| Preservation-first active-ring retirement | Bounded ring, exact-byte history, single-device receipts, reviewed policies and D1 fictional proofs | 1. Preserve no-removal baseline. 2. Hot write ring plus read-only historical preservation and epoch barrier. | Keep Option 1 operational. Continue Option 2 specifications after D1; D3's full-retained-set recovery drill is a hard predecessor of D6. | [Full proposal](proposals/safe-key-retirement.md) |

## Recommendation Summary

I recommend preservation completeness as the safety property: all retained historical versions remain readable, including dependencies acquired after preparation. The census diagnoses current dependencies; it does not authorize pruning or prove that a distributed dependency set cannot change. The barrier must prove membership and readiness, not a stable census.

[D1 evidence](d1-proof.md) records twelve passing fictional tests for the twelve-column census, resolved/nullable cases, tampering, bounded refusal, schema drift, exact-version reads, compile-time role separation and unchanged queue writes on refusal. These are test-only contract proofs, not production authority migration or an operational retirement mechanism. Full regression passed 214 Rust tests (five ignored) and 113 frontend tests; lint, build, production cargo check and formatting passed.

## Next Decisions

Next is D2 membership-trust specification: owner-root bootstrap, recovery and rollback-resistant roster pins. No schema may be added on the strength of this packet. D3 remains specification/measurement pending a reviewed format and full-retained-set total-loss drills; D4 must absorb existing hosted-lock re-keying and preserve lock-before-idempotency order; D5 must model explicit abort cleanup. D6 and D7 remain blocked.

The [updated review request](review-request.md) asks Claude to review the bounded D1 proof and corrections, not approve retirement. The five founder policies need no repeat approval. [Structured analysis](hardening.json) accompanies the proposal. No real vault, protected-storage budget, distributed concurrency or total-loss preservation test has been executed here.

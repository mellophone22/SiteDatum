# Implementation Plan: Separate offline owner root — conditional handoff

## Selected Design And Constraints

We have a founder-selected direction, not an accepted protocol or permission to implement it. This handoff is inert until independent review resolves specification blockers and an authorized proof scope is agreed. One owner, two active Windows computers, accountless local work, prior preservation policies and no-removal fallback remain mandatory.

## Source Revision And Drift Check

Original runtime source: `91973f57d9d4f9033dcf3455299046784ddf9742`.
D2 draft: `ce0bd4c92a9dea25b38582eecc947c54266189b8`, documentation-only.
Collection digest: `7c71d7df19e8ae93b51fce435efecaab4172c67644e3c4b0c6dfcd3e8092204a` from [context](../context.md).
Refresh and compare these trust boundaries before coding; material drift returns to design review.

## Affected Components

Future scope includes native enrollment/transfer, independent roster verification, protected pins, owner-artifact custody and trust-state diagnostics. Existing workspace-only receipts are not a membership barrier. D4 must preserve owner-wide membership while fencing workspaces; D5 must coordinate signed intents, publication and pin durability. None is changed here.

## Ordered Work Packages

| Package | Prerequisite | Deliverable | Acceptance gate |
| --- | --- | --- | --- |
| Independent review | Direction selected | Critical report at pinned packet | Resolve contradictions/missing guarantees; no code |
| Semantic clarification | Review findings | Root/owner binding, bootstrap, exact transitions, revocation, forks and total loss | Review revised design |
| Canonical encoding | Accepted semantics | Exact domains/framing, bounds and verification order using mature primitives | Crypto review; no format assigned here |
| Fictional verifier proof | Accepted specification and explicit proof scope | Test-only chain/omission/restore/fork/capability fixtures | Adversarial and positive tests; source-containment review |
| Artifact and pin design | D3 format review/measurement | Custody, recovery and latest-anchor reconciliation | No issuer or production migration before reviewed total-loss evidence |
| Publication/epoch integration | D4/D5 proofs | Ordering, lock hierarchy, cancellation, lost response and fork handling | Real contention/fault injection separately |
| D6 mutation / D7 activation | All earlier gates and later authorization | None now | Remain blocked |

We cannot collapse signed intent, committed publication and lost response into one success flag. Epoch reuse and forks must be distinguished from exact retries. Clarification precedes encoding, which precedes any parser proof.

## Compatibility And Migration

No first-device root silently grandfathered. Existing identities require explicit authorization into an independently pinned complete chain. Preserve two-device limits and old recovery files. Unsupported readers refuse without resetting pins or deleting keys. Minimum client version remains unassigned.

## Tactical Protections During Migration

Retain session/device/target/source proof and exact-transfer checks, current capacity refusal, absent production key removal and D1 role/source-containment tests. Provider lists, email reset, SQLite receipts and old root private material alone never establish current retirement readiness.

## Tests And Security Validation

Proposed only: bootstrap substitution; wrong owner/root/purpose; missing/reordered chain; omitted device; same-epoch fork; stale root artifact; succession without old-root authority; revoked-key replay; restoring all databases with independent pins intact; total independent-anchor loss; lost-response retry and cancelled intent; receipt/roster-ack promotion. Include positive complete-chain/reconciliation controls and compile-fail positive controls. No new protocol tests executed here.

## Performance And Resource Benchmarks

No measurements claimed. Compare baseline and fictional verifier at 1/8/100/1000 events: cold/cached verification, peak allocation, parse bounds and exhaustion. Windows custody/pin measurement belongs to D3. Determine budgets after measurement/review; never silently truncate history or assume an unbounded credential entry.

## Rollout And Rollback

No rollout now. Future rollback stops authority-bearing Sync writes while preserving highest pins, keys and local work. Old binaries/databases cannot clear trust errors. Total-anchor loss stays FreshnessUnestablished without a separately reviewed surviving-current-anchor path.

## Acceptance Criteria

Selection alone satisfies no technical criterion. Require accepted D2 design, reviewed encoding, passing bounded fictional proof, complete anchoring under declared assumptions, explicit fork/recovery outcomes and reviewed D3/D4/D5 integration. Indistinguishable total environmental rollback remains a residual. D6/D7 need their own gates and later authorization.

## Open Decisions

Exact artifact/ceremony, independent latest-anchor update/recovery, signing-intent cancellation/publication ordering, resource budgets and reader floor remain unresolved. Do not repeat prior policies or direction selection; material custody changes need a new direction decision.

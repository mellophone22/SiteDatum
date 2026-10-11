# Independent re-review request — D2 semantic corrections

Repository: https://github.com/mellophone22/SiteDatum
Branch: `codex/c10-02e-claude-review`
Runtime source base: `91973f57d9d4f9033dcf3455299046784ddf9742`
Prior reviewed packet: `0bf2e4ae04b0445be751002bb6bc33aed4f19d95`

Review the exact correction commit supplied with this request, not moving HEAD. Read [remediation](review-remediation.md), [proposal](proposals/owner-roster-trust.md), [context](context.md), [portfolio](hardening.md), [JSON](hardening.json), [validation](validation.md), [founder decision](founder-decision.md), [prior policy context](prior-context.md) and [conditional handoff](implementation/offline-owner-root.md).

Act as a critical independent reviewer in read-only mode. Do not alter repository/services/schema, issue roots, activate Sync, expose secrets, use customer data or purchase resources. Separate executed checks from proposed tests. No approval is inferred from this request.

Give separate verdicts on:
- DG-1: fresh event/workspace ack, signer's independent pinned tip, exact participant set and late-change invalidation. Challenge whether this blocks only cross-event replay or really supports the stated freshness assumptions. Pins are lower bounds, not global-latest proofs.
- DG-2: both membership mismatch directions and key mismatches, authorization-boundary prerequisites, migration/partial hosted-only enrollment. A returned list is not completeness evidence against a malicious provider.
- DG-3/DG-4: the new multi-parent ForkResolution proposal, union revocations, full known branch inventory and every required witness. Is it safe, useful and honestly limited? No claimed prevention of stale duplicated-artifact signing. Freeze remains fallback; no root-loss shortcut.
- DG-5: u64 semantic epoch range, issuance/cancellation reservation, exact lost-response retries, maximum refusal and parse/storage limits. No serialization assigned.
- SF-1/SF-2/DG-6/PD-1: 21 matching lines in five files, exact cap citations, diagrams and combined-freeze disclosure.
- Overall semantics readiness versus unresolved canonical encoding, D3 artifacts/Windows drills, D4 fences and D5 journals. D6/D7 remain blocked.

Use at most two simultaneously active devices for product fixtures. The revoke/replace sequence preserves that limit and distinguishes the hosted revocation gate from the analytical stale-view issue. Do not silently weaken the cap. Challenge same-event replay, complete but stale chains, lost responses, unknown forks, conflicting revocations, omitted witnesses, root succession/compromise, total-anchor loss and exhaustion.

Return corrections and proposed acceptance tests with source facts separate from design reasoning. These are specification changes, not fixes to an activated protocol. ForkResolution is proposed for review and later explicit acceptance, not already founder-approved. No blanket implementation approval or merge/release follows.

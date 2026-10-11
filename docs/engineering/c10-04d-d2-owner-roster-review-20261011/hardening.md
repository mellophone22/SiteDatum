# Security Hardening Review: SiteDatum C10-04D D2

We have completed a D2 **specification draft**, not a root, roster implementation or accepted mechanism. The founder's five policies remain settled; the additional customer-held owner-signing direction is now selected for independent review.

## Evidence Basis

I inspected native device possession and transfer signatures, hosted initial/approved enrollment, SQLite anchors and single-device receipt shapes at `91973f57d9d4f9033dcf3455299046784ddf9742`. [Context](context.md) pins ten Git-blob hashes. The supplied independent AI review motivates the root/rollback questions; it is not certification. No new D2 behavioral test has run.

## Constraints

One owner, local-first Windows, accountless ordinary work. Existing two-device limit is preserved. No schema, production source, key material, service, billing, release or Sync activation changes. D6/D7 stay blocked.

## Opportunity Portfolio

| Opportunity | Evidence | Options | Recommendation | Proposal |
| --- | --- | --- | --- | --- |
| Distinct owner authority and rollback-aware roster | Device possession/transfer; provider enrollment list; SQLite-bound anchors; MP-2 design review | 1. No-removal baseline. 2. Separate offline owner root with complete signed history and independent pins. | Review founder-selected Option 2; keep Option 1 operational. | [Owner/roster trust](proposals/owner-roster-trust.md) |

## Recommendation Summary

We would separate the key that proves “this computer” from the authority that approves “these are all my computers.” Independent latest pins help ordinary database restore, but cannot manufacture freshness after every anchor is lost or rolled back. The proposed response is FreshnessUnestablished with no retirement permission and local work preserved.

The founder selected the separate offline artifact design direction for review. I still require independent assessment of its recovery and update burden before implementation. Losing it would freeze membership-changing actions, not delete project data. We do not silently choose first-device authority or email-based replacement.

## Next Decisions

Review the [full semantic specification](proposals/owner-roster-trust.md) using [this request](review-request.md), with the [founder decision](founder-decision.md) recorded. [Structured analysis](hardening.json) records the tradeoffs. D2 technical acceptance, canonical format, Windows pin/recovery proof and D4/D5 ordering remain open. The [conditional handoff](implementation/offline-owner-root.md) orders future work without authorizing code. D3 specification may proceed separately; no schema or mechanism implementation follows automatically.

## Independent-review correction round

The D2 report at packet 0bf2e4ae supports the direction but requires semantic clarification before canonical encoding. We propose fresh event-bound acknowledgements carrying each signer's independent pin, fail-closed reconciliation in both membership-disagreement directions, a separately reviewed multi-parent fork join, honest duplicate-custody limits, and a u64 epoch bound with exact-retry semantics. [The correction ledger](review-remediation.md) separates proposed acceptance tests from checks actually performed. The fork join is not accepted or implemented; inability to justify it leaves authority frozen. Source inventory and diagrams are corrected, and capacity disclosure now names the combined freeze. No production vulnerability is claimed demonstrated by these analytical gaps.

## Re-review refinement package

The report at 1dd73683 supports semantics readiness only after N-1/N-2/N-3 refinements, with N-4/N-5 improvements. [Refinement ledger](re-review-remediation.md) records all five. We keep the existing MAX_SAFE_INTEGER counter convention, specify single-event owner/workspace admission with independently reserved signer slots, publish the narrow fork taxonomy and provider divergence residual, and isolate freshness from hosted revocation in the sharper fixture. No implementation acceptance follows. Fork adjudication remains proposed; conflicting revocations or an unavailable historical witness retain freeze.

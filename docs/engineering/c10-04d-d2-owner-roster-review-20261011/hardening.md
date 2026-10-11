# Security Hardening Review: SiteDatum C10-04D D2

We have completed a D2 **specification draft**, not a root, roster implementation or accepted mechanism. The founder's five policies remain settled; customer-held owner-signing custody is an additional decision.

## Evidence Basis

I inspected native device possession and transfer signatures, hosted initial/approved enrollment, SQLite anchors and single-device receipt shapes at `91973f57d9d4f9033dcf3455299046784ddf9742`. [Context](context.md) pins ten Git-blob hashes. The supplied independent AI review motivates the root/rollback questions; it is not certification. No new D2 behavioral test has run.

## Constraints

One owner, local-first Windows, accountless ordinary work. Existing two-device limit is preserved. No schema, production source, key material, service, billing, release or Sync activation changes. D6/D7 stay blocked.

## Opportunity Portfolio

| Opportunity | Evidence | Options | Recommendation | Proposal |
| --- | --- | --- | --- | --- |
| Distinct owner authority and rollback-aware roster | Device possession/transfer; provider enrollment list; SQLite-bound anchors; MP-2 design review | 1. No-removal baseline. 2. Separate offline owner root with complete signed history and independent pins. | Review Option 2 and obtain explicit artifact-custody selection; keep Option 1 operational. | [Owner/roster trust](proposals/owner-roster-trust.md) |

## Recommendation Summary

We would separate the key that proves “this computer” from the authority that approves “these are all my computers.” Independent latest pins help ordinary database restore, but cannot manufacture freshness after every anchor is lost or rolled back. The proposed response is FreshnessUnestablished with no retirement permission and local work preserved.

I recommend the separate offline artifact only if the founder accepts its recovery and update burden after independent review. Losing it would freeze membership-changing actions, not delete project data. We do not silently choose first-device authority or email-based replacement.

## Next Decisions

Review the [full semantic specification](proposals/owner-roster-trust.md) using [this request](review-request.md), then explicitly decide the root-custody direction. [Structured analysis](hardening.json) records the tradeoffs. D2 acceptance, canonical format, Windows pin/recovery proof and D4/D5 ordering remain open. D3 specification may proceed separately; no schema or mechanism implementation follows automatically.

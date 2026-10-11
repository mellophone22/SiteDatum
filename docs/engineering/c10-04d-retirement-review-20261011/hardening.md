# Security Hardening Review: SiteDatum C10-04D

We are designing safe active-ring compaction, not implementing key destruction. The founder approved five conservative policies; that closes the policy-choice gate, not the technical proof or activation gates.

## Evidence Basis

Source base: `097e8daa4459c03655dc5046372aefca0c138d97`, clean at inspection, 2026-10-11. We inspected bounded keyring/recovery, single-device receipts, exact-byte retry history, single-key transfer and the existing isolated hosted write proof. The [evidence context](context.md) records checkout hashes and limitations. The supplied Claude two-verdict report is supplemental AI evidence, not certification or a reproduced experiment.

The report's broad statement that no hosted schema exists is too strong: an isolated encrypted-record proof exists. What remains missing is the retirement epoch, complete applied-device barrier and coordinated fence.

## Constraints

Every active device must participate; offline age cannot silently exclude one. Old recovery files and backups remain untouched and usable for captured material. Each event requires explicit founder consent. Membership must be owner-approved and independently anchored, not merely a provider list. Ordinary local work remains available. No implementation, migration, release, deployment or Sync activation is authorized by this packet.

## Opportunity Portfolio

| Opportunity | Evidence | Options | Recommendation | Proposal |
| --- | --- | --- | --- | --- |
| Preservation-first active-ring retirement | Bounded keys/recovery; local receipts/retries; approved policies; independent retirement prerequisites | 1. Preserve no-removal baseline. 2. Hot write ring plus protected historical read vault and epoch barrier. | Review/prove Option 2; keep Option 1 operational until all gates pass. | [Full proposal](proposals/safe-key-retirement.md) |

## Recommendation Summary

I recommend the second option as a design direction because it separates two obligations we cannot honestly combine: stopping old keys from authorizing new writes, and preserving keys needed by historical material. We would reclaim hot-ring slots, not erase every historical copy. The no-removal baseline remains a sound choice if the new recovery, trust-root or storage costs cannot be proved acceptable.

The proposal specifies the twelve-column census, separate epoch/device barrier storage, signed roster and acknowledgement bindings, final local rechecks, multi-store ordering and future adversarial cases. Its explicit blockers include offline edits after preparation, owner-root recovery, Windows vault budgets, full-history preservation/transfer and retention/bootstrap integration. It is not a complete wire protocol or an implementation approval.

## Next Decisions

Send the [review request](review-request.md) and [proposal](proposals/safe-key-retirement.md) to Claude for a design-only adversarial review. Ask for separate verdicts on policy fidelity, architectural direction and readiness of each future proof slice. Do not ask it to approve key deletion on the strength of this document.

After that review we can resolve the identified protocol questions and choose the mechanism separately. The existing founder policies do not need to be reapproved. Review approval of this draft would still not activate Sync or authorize an individual retirement event.

[Structured analysis](hardening.json) accompanies the readable proposal. No new cryptographic, concurrency or Windows behavioral tests were executed in this documentation phase.

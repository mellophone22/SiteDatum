# Independent review request: C10-04D preservation-first retirement

## Target and scope

Repository: https://github.com/mellophone22/SiteDatum
Branch: `codex/c10-02e-claude-review`
Source base: `097e8daa4459c03655dc5046372aefca0c138d97`
Review this packet at the exact documentation commit supplied in the accompanying message, not a moving branch head.

Read [the portfolio](hardening.md), [full proposal](proposals/safe-key-retirement.md), [structured analysis](hardening.json) and [evidence inventory](context.md). Then inspect the pinned source boundaries they cite. This is a design-only addition to the previously reviewed format foundation, not a retirement implementation. The implementation at the source base is unchanged.

## Instructions for Claude

Act as a critical independent AI reviewer, not an advocate for the proposed design. Do not modify the repository or any service; do not use customer data, reveal secrets, buy anything, create paid resources or activate Sync. Any local experiments must use disposable fictional fixtures and be explicitly reported. Do not represent an AI review as professional certification.

The founder approved these five policies: every active device must acknowledge; historical recovery files remain usable for captured material; old backups stay untouched and recoverable; each event needs explicit consent; membership requires complete owner-approved signed history, not a provider-only list. Those policies are settled. Do not treat them as approval of a vault, archive format, migration or key removal.

Give separate verdicts:

- Policy fidelity and honesty of the product promises.
- Option 2's architectural direction versus the no-removal baseline.
- Whether each D1-D7 slice has sufficient design detail to proceed, naming missing proofs or specifications. No blanket implementation approval.

Challenge at least:

- Historical-key read capability versus current-write authority; capacity reclamation is not cryptographic erasure.
- Complete roster/root binding, omission/rollback resistance and total-loss trust-anchor recovery.
- Workspace/epoch/event/device schema, existing workspace-only receipt callers, and private verified capability construction.
- Offline edits after preparation and before hosted commit/local pruning. Identify a counterexample; do not assume a signature lease solves it.
- Serialization of enrollment, revocation, restore, conflict creation and fresh writes with the fence; original committed retry bytes and outcomes.
- Every one of the twelve ciphertext columns, nullable/resolved paths, checkpoints, customer-held files and historical dependency availability.
- Protected vault integrity, quota/partial-write failures, bounded allocations and load behavior; missing entries must retain the original ring.
- Historical preservation format and full-history transfer, which are explicitly not implemented or specified as a new cryptographic construction here.
- Crash ordering across SQLite, Windows protected storage, hosted epoch and customer-held preservation copy; lost responses and cancellation after commit.
- Old clients, supported-reader negotiation, rollback without lowering floors and continued accountless local access.
- Tombstone retention caps, explicit deletion and bootstrap availability. Reject silent retention extensions.
- Source claims, unsupported edges in diagrams, evidence mapping and unmeasured engineering tradeoffs.

The supplied prior report's claim that no hosted schema exists was narrowed after source inspection: isolated hosted encrypted-record operations exist, but no retirement barrier/floor does. Verify that distinction.

## Expected report

For each issue: classify it as observed source fact, design contradiction, missing proof/specification or product policy question; explain a concrete failure scenario; cite source or proposal section; recommend a correction and acceptance test. Say which planned read-only/proof slices may proceed and which mutation slices must remain blocked. Report tests actually run separately from proposed tests.

Keep “approved design direction,” “approved implementation,” “implemented,” “deployed” and “activated” separate. We are requesting none of the last three. The current safe fallback is no removal and fail-closed capacity refusal.

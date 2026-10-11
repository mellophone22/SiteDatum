# Independent design review request — D2 owner-root and roster trust

Repository: https://github.com/mellophone22/SiteDatum
Branch: `codex/c10-02e-claude-review`
Source base: `91973f57d9d4f9033dcf3455299046784ddf9742`
Review the exact selection-record/handoff commit supplied with this packet, not a moving head. Previous D2 draft: `ce0bd4c92a9dea25b38582eecc947c54266189b8`; original production source base remains the one above.

Read [proposal](proposals/owner-roster-trust.md), [portfolio](hardening.md), [context](context.md), [structured analysis](hardening.json), [prior policy context](prior-context.md), and [validation](validation.md). Treat the report as specification-only. No schema, root issuance, key removal, service mutation or activation is requested.

Act as a critical independent reviewer. Do not change any repository/service, use customer data, expose secrets or buy resources. Any local fictional experiments must be labelled actually executed versus proposed. This is supplemental AI review, not professional certification.

Give separate verdicts:
- Fidelity to the five founder policies and the extra root-custody decision.
- Whether separate owner authority actually avoids first-device circularity under the stated trusted-bootstrap assumption.
- Complete chain, root generation, owner binding, revocation, forks and stale-copy signing semantics.
- Ordinary SQLite restore versus total independent-anchor loss/rollback. Do not accept signature validity or email recovery as freshness.
- Conservative owner-wide two-device roster and separate workspace barrier binding.
- D2 specification readiness versus remaining canonical encoding, artifact, migration and D4/D5 publication/journal proofs.

Try counterexamples: a hosted omitted device; both databases restored before revocation; equal-epoch forks; root private key recovered without latest anchor; stale duplicated owner artifact; all pins rewound; a device without workspace preservation; new enrollment while a retirement event is prepared; root succession after old-root loss; lost-response retry and cancelled intent.

Challenge whether any language implies a complete current roster can be proven when all independent latest anchors are lost. Verify that FreshnessUnestablished grants no retirement capability, preserves local work and does not erase captured keys. Verify no root key or serialized format has been assigned/created and no existing receipt is promoted into an all-device barrier.

Report source facts separately from design contradictions and missing proofs. Recommend corrections and proposed acceptance tests. No blanket implementation approval. D6 and D7 remain blocked. Selecting a direction would still require separate approval and review before any format/schema/code.

## Direction selected; technical review still required

Read [the founder decision](founder-decision.md) and [conditional handoff](implementation/offline-owner-root.md). Separate offline owner-signing custody is selected as a direction, not an accepted construction. Do not ask to repeat that decision or the five preservation policies. Challenge exact custody, independent latest-anchor recovery, trusted bootstrap, stale duplicated artifacts, root succession, cancelled-intent/epoch semantics and loss/fork handling. Planned protocol tests remain unexecuted. Explain required specification corrections before any format/schema/production code. D6/D7 remain blocked.

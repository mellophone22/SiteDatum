# Founder direction decision — D2 owner authority

Date: 2026-10-11.
Prior draft: `ce0bd4c92a9dea25b38582eecc947c54266189b8`.
Decision: **Option 2, separate offline owner-signing artifact, selected as the design direction for independent review.**

The founder answered yes to the explicit direction-review question, then clarified the unfinished “and” was accidental and asked to continue. We record that as direction selection, not cryptographic certification, implementation acceptance or permission to issue artifacts.

## Selection scope

We may prepare and publish the design-only review packet and conditional ordering of future work. The direction distinguishes owner signing from computer identity and proposes independently retained latest roster pins. A customer-held artifact is contemplated; its exact format, protection and ceremony remain unspecified and unissued.

## Exclusions

No root generation, artifact issuance, production parser/schema, credential migration, service mutation, main merge, release, key removal or Sync activation. No tested security or reliable recovery property is inferred from this decision. Material changes to custody or trust require a new decision.

The five earlier preservation policies remain unchanged. Selection does not waive independent review or accept unresolved protocol details. D2 technical acceptance, D3 preservation-format/drills, D4 fencing and D5 ordering remain open. D6/D7 remain blocked; no-removal capacity refusal stays operational.

## Next gate

Send [the review request](review-request.md) and [proposal](proposals/owner-roster-trust.md) at the exact follow-up commit to Claude for a critical read-only independent review. A reviewer report for packet 0bf2e4ae04b0445be751002bb6bc33aed4f19d95 has now been supplied. The founder authorized a documentation-only correction package. This authorizes proposed semantic corrections and another review handoff, not acceptance of ForkResolution, encoding, artifact issuance or implementation. Return findings for remediation before mechanism implementation.

# D2 independent-review semantic corrections — 2026-10-11

## Scope and evidence

We are responding to the conversation-supplied independent design review of packet `0bf2e4ae04b0445be751002bb6bc33aed4f19d95`, runtime source base `91973f57d9d4f9033dcf3455299046784ddf9742`. The founder authorized this documentation-only correction package. Independent review is supplemental AI evidence, not certification; none of these proposed protocol controls is implemented or tested. The original report remains evidence, not an instruction authorizing code.

I verified the pinned HEAD, clean worktree, matching-line inventory and the hosted two-device checks. We use those observations to correct source claims. Protocol cases remain analytical and planned. [Proposal](proposals/owner-roster-trust.md) contains the normative proposed semantics; this ledger explains deltas and proof gaps.

## Finding-by-finding disposition

| Finding and meaning | Proposed correction | Acceptance case — proposed, not executed |
| --- | --- | --- |
| DG-1 — replay plus stale consistent chain | Event/workspace bound ack, independent signer tip, exact member count/digest, higher/lower/forked/missing pins block, pending events invalidated on newer membership | Replay ack into new event refuses; A's higher pin prevents stale B readiness; unchanged-epoch fresh-event positive control; membership change after collection invalidates |
| DG-2 — hosted-only device | Compare both sets and exact keys at collection/final assembly; hosted-only identity has no ack authority; signed-only member is not silently omitted | Hosted insert with absent root authorization blocks with distinct reason; missing hosted member blocks; explicit reconciliation only |
| DG-3 — no fork recovery | Separate proposed multi-parent ForkResolution, no implicit single-parent exception; full histories and branch inventory, root plus required witnesses, union revocations, no active-set expansion; no-removal freeze fallback | Equal-epoch children freeze; missing branch/witness/owner binding refuses; complete eligible two-branch join positive control, then fresh readiness required |
| DG-4 — stale copies cannot prevent forks | Single-instance journal reduces accidents; duplicated offline copies cannot know latest; custody plus detection, not prevention guarantee | Stale copy can form valid fork in model; reconciliation catches it; no claim that signature alone prevents it |
| DG-5 — unbounded counter prose | Semantic MAX_SAFE_INTEGER ceiling; distinct issuance/cancellation advances; identical retry advances none; maximum clean refusal, no wrap/reset | Reduced-bound cancellation model and safe-integer boundary; exact lost-response retry no increment; partial/committed intent cannot be cancelled by assumption |
| SF-1 — stale caller inventory | 21 matching lines / five named files pinned to source base | Recompute per-file matching-line counts and classify production/test; fail review on drift rather than automatic PK migration |
| SF-2 — cap citation | 03e checks at lines 269/508; distinguish 03b existing-device gate/rate limit | Verify exact source locations; no invented numeric check in 03b |
| DG-6 — authority diagram | Hosted list feeds comparison verifier, not signed-chain authority; pin loss has inbound edge | Inline and standalone diagrams exact agreement; inspect edge meaning |
| PD-1 — combined freezes | Capacity disclosure links trust freeze, replacement/enrollment/barrier loss, eventual eight-key refusal and surviving local capabilities | Review complete disclosure, no immediate-rotation-loss overclaim, no recovery promise for missing keys or override |

## Two-device freshness fixture

The report's three simultaneously active devices cannot be a current-product positive control. We instead propose: at epoch 7 A/B are active; A advances owner-authorized revoke-B at epoch 8 and enroll-C at epoch 9, with at most two active identities at any instant. B has a legitimate old pin 7 and receives a consistent chain truncated there. Replaying A's epoch-7 ack into a fresh event must fail by nonce/scope. If A is asked freshly to acknowledge that event, A's independently pinned tip 9 refuses stale epoch 7 rather than excluding A. B's revoked hosted identity should also fail hosted authorization under the honest hosted boundary; this fixture must record which control refused and must not claim a reachable attack on current source.

A second fixture with A/B still active isolates unchanged-membership epoch advance or cancellation: A has a newer pin and B is behind; fresh response carries that newer pin. Positive readiness uses two currently authorized devices with matching independent tips and fresh scoped responses. No third active member or relaxed cap is introduced.

These cases demonstrate the intended freshness-witness semantics only after a fictional proof exists. They do not prove global freshness when every witness is stale or a provider hides all new branches. Nonce replay within the same event and a late membership change still require D4/D5 invalidation/final recheck; cached old success cannot substitute.

## Fork proposal limitations

We recommend review, not acceptance, of ForkResolution. A single artifact cannot ensure globally unique signing when copied. A root signature cannot establish that every competing branch was disclosed. The proposed join requires complete verified known histories and confirmations from every device active on either branch, retaining all revocations; any active-on-one/revoked-on-another conflict blocks the proposed automated path. Unknown history, unavailable witnesses, root compromise, root loss and resource exhaustion retain freeze. Independent review must challenge whether this stringent rule provides useful recovery and whether it accidentally excludes a valid branch member.

If a useful safe join cannot be justified, permanent freeze with explicit custody/capacity disclosure remains acceptable operational behavior. This correction does not select a weaker recovery override. Fork adjudication cannot create retirement authority or repair missing encryption keys; D3/D4/D5 and later D6 remain required.

## Numeric bound and resource limits

The earlier u64 proposal is superseded by the re-review N-2 correction: MAX_SAFE_INTEGER = 9,007,199,254,740,991, matching the existing numeric protocol convention. At a hypothetical million issued events per day, exhaustion takes approximately 24.66 million years; arithmetic only, not a rate allowance. [Re-review refinement ledger](re-review-remediation.md) pins source evidence and boundary cases. Storage/parse budgets can bind first and must refuse rather than discard history. Exact integer transport remains required on every surface. Cancellation burns a distinct issued epoch once; retries of identical bytes burn none. D5 must resolve failed reservations and publication explicitly without reuse. No counter-space successor is assigned.

## Gate state

Semantic corrections drafted: yes. Independent acceptance: pending. Encoding assigned: no. Format/parser/schema/root issuance: no. Fictional verifier executed: no. Service or production changes: no. D6/D7 blocked. All keys and ordinary local work retain the existing no-removal baseline.

## Sharper freshness fixture from N-5

Keep the preceding revoke-B case as defense in depth, but add the isolated semantic model: A/B active at epoch 7; the owner ceremony on A authorizes revoke-A at epoch 8 and enroll-C at epoch 9. B stays hosted-active with a legitimate old pin 7 and a provider-truncated chain. Replayed A ack from a different event refuses. A's locally retained independent pin 9 refuses a fresh request scoped to roster 7 as ParticipantPinAdvanced; unavailable A blocks instead of being omitted.

A is revoked, so ordinary hosted collection may reject A before this response is transported. The isolated fictional signer model can inspect the retained pin/refusal path without granting A a new valid acknowledgement or normal network authority. Record separately that B's own hosted revocation does not protect this variant, and distinguish the conceptual freshness guard from existing source reachability. Positive readiness uses B/C with matching current pins. At most two active identities at any instant; no fixture claims implemented protocol behavior.

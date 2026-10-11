# D2 re-review refinements — 2026-10-11

## Evidence and authorization

We respond to Claude's conversation-supplied independent re-review of `1dd7368331a6070693733ea62537a74b680d63eb`, runtime base `91973f57d9d4f9033dcf3455299046784ddf9742`. The founder authorized a documentation-only package covering N-1 through N-5. The report treats earlier issues as closed at specification level, not as runtime fixes. ForkResolution remains a proposed mechanism for independent review and later explicit acceptance.

I verified the preparation HEAD/clean checkout and re-opened MAX_SAFE_INTEGER at src-tauri/src/sync_v2_record_codec.rs:14 plus numeric jsonb_build_object checkpoints at C10-03g:380, 406 and 428. These are source observations; no proposed protocol exists to execute. The ten-file LF/Git-blob inventory remains tied to the original runtime base, not to the new report or authored text. [Proposal](proposals/owner-roster-trust.md) supplies the refined semantics; [prior ledger](review-remediation.md) preserves the earlier correction context.

## Corrections and acceptance cases

| Finding and meaning | Refinement | Acceptance cases — proposed, not executed |
| --- | --- | --- |
| N-1 — narrower fork recovery | Publish divergent-revocation freeze, eligible duplicate-intent/sub-cap cases, required historical availability, no membership-cap exception | Conflicting tombstones refuse; duplicate-intent join requires root plus A; sub-cap divergence requires A/B/C branch witnesses with a two-member result and explicit losing-candidate revocation; missing witness refuses |
| N-2 — numeric convention mismatch | Epochs and related authority counters bounded by MAX_SAFE_INTEGER; no u64 exception, no rounded comparison | Exact 2^53-1 on all future surfaces; 2^53/out-of-range/fractional input refuses; boundary increment refuses as EpochExhausted, preserves pins/keys; exact retry consumes none |
| N-3 — parallel initiations | One admitted event per owner/workspace across tips; owner-scoped issuance position plus durable independent signer reservation, no device-local allocator authority | Healthy A/B race admits exactly one and refuses competitor as EventAlreadyInFlight; same request idempotent; split/fault may admit neither, never two; partial/aborted/committed events cannot leave replayable readiness |
| N-4 — view-divergence denial | Explicit provider-mediated availability residual and ParticipantPinAdvanced diagnostic | Higher legitimate tip delivered to one signer only blocks; missing extension keeps blocked; repetition never weakens safety or silently discards member; reason content-free and distinct from corruption/fork/missing witness |
| N-5 — fixture confound | Keep revoke-B defense-in-depth case; add revoke-A so stale B remains hosted-active | Old A ack fails scope; fictional fresh A signing attempt sees retained newer pin and refuses; unavailable A blocks; B/C positive control. Distinguish revoked A transport denial from isolated signer semantics |

## Fork taxonomy and custody

We preserve the union-revocation/no-resurrection rule. Divergent revoke-A/revoke-B branches from {A,B} are structurally unresolvable; a duplicate revoke-B fork may be eligible with A; enroll-B/enroll-C from {A} may be eligible with three historical witnesses but never three active memberships. “Eligible” means a candidate for the unimplemented reviewed join, not tested recovery. Witness collection must not temporarily enroll a revoked/excluded device or grant record authority. D4/D5 must review any isolated confirmation channel.

We do not turn the reviewer's inference about likely fork shapes into measured frequency. The customer consequence is concrete: independently used signer copies can cause permanent freeze, especially conflicting revocations, and losing a required historical computer closes recovery. If independent review cannot justify useful recovery, freeze is the fallback. Later explicit acceptance of ForkResolution is still required.

## Counter convention and arithmetic

We choose the existing numeric-safe ceiling instead of a protocol-wide string-only u64 exception. The limit is 9,007,199,254,740,991 (2^53-1), verified in native source. The proposed semantics apply it to roster epochs and owner issuance positions. All future JSON/native/SQL/signature surfaces require exact integers and pre-comparison range validation; canonical encoding remains unassigned.

Arithmetic checked here: bound / 1,000,000 / 365.25 is approximately 24,660,368 years. This is a hypothetical consumption illustration, not a permitted rate or benchmark. Resource limits can bind earlier and must refuse without silent history deletion. No root-generation reset or counter successor is a workaround. Boundary fixtures should use exact integer representations, never literal JS numbers above the bound that have already rounded.

## Event-uniqueness semantics and proof gap

The uniqueness domain is (owner, workspace), not device or roster tip. Event identity additionally binds owner/root generation, owner-scoped non-reused issuance position, nonce, roster tip and immutable parameters. Position allocation and reservation are not proven by a hosted success flag. D4 must serialize the canonical owner/workspace admission boundary; D5 must reconcile the owner authorization history and independently durable per-device slot before any complete admission.

Each honest current member reserves only one identity before acknowledging and checks that reservation again at final commit. Two complete aggregations must not exist for competing events. An exact retry returns the same identity. A healthy contention test expects one winner; crash/split evidence may correctly block both. We must not promise availability while requiring safety. A later event needs a verified terminal outcome, invalidated old acknowledgements and reconciled release; expiry/lost response alone is not release authority.

The detailed journal format, lock ordering, abort cleanup and liveness proof belong to D4/D5. Root-copy forks and all-device rollback remain declared residuals. We do not assert that adding a textual single-event rule has implemented a lock or supplied consensus against every malicious provider.

## Freshness fixture isolation

N-5's revoke-A/replacement sequence keeps stale B active, isolating the fact that B's own hosted revocation is not a defense. A is revoked, so an actual hosted path may reject it before a new ack is transported. The proposed fixture should inspect the fictional signer decision with A's retained newer pin separately, treating refusal as refusal, not as resurrection of A's membership. If A is unavailable, old membership must not omit it to finish stale readiness. Record each control's result; do not claim a production exploit or executed protocol test.

## Gate state

All five document refinements drafted. Independent semantic re-review pending. Canonical encoding remains unassigned and must be reviewed separately after accepted semantics. ForkResolution is not explicitly accepted or implemented. D3 preservation artifacts/Windows total-loss drills, D4 fences and D5 journals remain prerequisites. D6/D7 blocked; ordinary local work and no-removal capacity refusal unchanged.

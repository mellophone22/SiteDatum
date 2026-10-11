# 2026-10-11 review remediation ledger

Prior packet: `8be1f5e8b415b62f9b696adb66a96d7d5e758b7c`. The supplied review is supplemental AI evidence, not certification. “Corrected specification” does not mean implemented mechanism.

| Finding | This follow-up | Outstanding gate |
| --- | --- | --- |
| OS-1 hosted lock mismatch | D4 scope explicitly includes inventory/re-keying already-reviewed affected operations, owner-wide invariants and deterministic lock hierarchy. No SQL changed. | D4 protocol, regression review and real two-session contention tests. |
| OS-2 invented name/order | Correct function name and advisory-lock → idempotency → row-lock distinction; executed source-order regression. | Preserve order when future floors are added. |
| DC-1 undefined preservation artifact | D3 is a hard predecessor of D6: reviewed existing full-retained-set format plus passed total-loss drills at 1/8/100/1000 versions. | Format, implementation, independent crypto review and actual drills; none supplied here. |
| MP-1 dependency growth | Preservation completeness chosen as design direction; census diagnostic, barrier readiness/membership, not stable dependencies. Retired-version reads remain available. Executed local new-dependency test, with no pruning. | Distributed/offline edit and rebase protocol, full retained-set preservation proof. |
| MP-2 root/rollback | D2 specifies bootstrap circularity, separate-root alternatives, pins outside database restore and fail-closed disagreement/total-anchor loss. | Choose/review owner root and recovery; no schema yet. |
| MP-3 role separation | Completed test-only sealed distinct read/write contract, two compile-fail fixtures plus positive control, historical write-None and queue-refusal tests. | Production interface migration requires its own gated review. |
| MP-4 abort cleanup | D5 specifies explicit retention/deletion abort, fence release on return, void event nonce, preserved keys/files and no committed-floor rollback. | Model/fault injection and implementation proofs. |
| MP-5 vault budget | D3 specifies multi-entry authenticated catalog versus encrypted-file candidate and official per-entry bound; no budget measurement claimed. | Windows measurement and reviewed construction. |
| DC-2/DC-3 diagrams | Backups feed SQLite ciphertext, not keys; future preservation node labels format unspecified; inline and standalone sources agree. | Rendering not executed. |
| PP-1 deletion/consent | ADR-019 §8 and C10-03H clarify hosted deletion versus local decryption capability; draft consent names retirement, not erasure. | Real D6 consent surface/copy tests remain blocked. |
| PP-2 offline capacity | Draft consent and key-recovery limitation disclose indefinite unrevoked offline-device blocking. | Product surface before activation. |
| PP-3 hash framing | Original inventory explicitly LF/Git-blob at original source base, with same collection recipe. No mismatch may be dismissed as line endings. | Reproduce original pinned inventory during independent review. |

The five founder policies are unchanged and need no repeat approval. D1 is complete only as the [fictional proof](d1-proof.md). Next: D2 specification. D6 key mutation and D7 activation remain blocked; no removal and fail-closed capacity refusal remain operational.

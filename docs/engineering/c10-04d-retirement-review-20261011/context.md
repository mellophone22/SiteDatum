# Evidence context: C10-04D retirement design

Date: 2026-10-11. Repository: mellophone22/SiteDatum; GitLab mirror Kikito22/sitedatum.
Inspected source revision: `097e8daa4459c03655dc5046372aefca0c138d97`; working tree was clean at the original inspection. This follow-up adds test-only D1 source and corrects design/policy documents; original production behavior remains unchanged. The twelve-file inventory below continues to bind the original source base, not the extended packet.

This is a standalone design analysis, not a sealed security scan. The supplied Claude review of `579cfed` is conversational evidence, not a locally hashed report or reproduced experiment. Its approval is conditional for the compactable format, and its retirement-design verdict is NOT APPROVED FOR IMPLEMENTATION. Founder approval covers five conservative policies, not key destruction or an archival mechanism.

## Evidence registry

| ID | Reader-facing title | Source and observed boundary |
| --- | --- | --- |
| review-claude | Supplied two-verdict independent AI review | Conversation report at 579cfed: R-1 local schema, R-2 missing protocol proofs, R-3 policies, R-4 concurrent mutation fence. Supplemental AI evidence, not certification. |
| source-ring | Bounded exact-version keyring and recovery | src-tauri/src/sync_v2_key_recovery.rs: KeyRing, WorkspaceKeys, protected storage, recovery formats, rotation staging. |
| source-local | Local ciphertext persistence and exact retries | src-tauri/migrations/0012_sync_v2_local_queue.sql; 0014_sync_v2_apply_protocol.sql; 0015_sync_v2_key_rotation.sql; src-tauri/src/sync_v2_local_queue.rs; sync_v2_safe_apply.rs. |
| source-transfer | Single-key approved device transfer | src-tauri/src/sync_v2_approved_device_transfer.rs. A full historical ring and roster trust anchor are not transferred by this protocol. |
| source-hosted | Existing isolated hosted encrypted-record proof | proofs/sync-v2-identity/supabase/migrations/20261010022507_c10_03g_encrypted_record_operations.sql: public.sync_v2_record_bridge_push_batch. Not a retirement or all-device barrier implementation. |
| policy-founder | Approved five retirement policies | docs/engineering/C10-04D-KEY-ROTATION-RECOVERY.md at 097e8da; source policy record. |
| policy-retention | Existing privacy and hosted deletion boundaries | docs/engineering/ADR-019-SYNC-V2-PRIVACY-AND-SECURITY-BOUNDARY.md; C10-03H-HOSTED-RETENTION-AND-DELETION.md. |
| packet-existing | Current review and activation gate record | docs/engineering/C10-04D-CLAUDE-REVIEW-INSTRUCTIONS.md. |

## Integrity inventory

The following SHA-256 values bind the twelve inspected checkout files, not the conversational review. These twelve recorded hashes match LF/Git-blob content at the immutable source base. Reviewers should hash the exact Git blob bytes (or an LF checkout); do not dismiss a Git-blob mismatch as a line-ending artifact. CRLF checkout bytes are a different representation, not this inventory's reference. Collection SHA-256 is computed from sorted `path + space + lowercase SHA-256` records joined with LF and no trailing LF:
`5cccd585c644bcdaf8f3df7afc186391e48d1b3b6e61db0973628bdee3f450ac`.

| Repository-relative file | Checkout SHA-256 |
| --- | --- |
| docs/engineering/ADR-019-SYNC-V2-PRIVACY-AND-SECURITY-BOUNDARY.md | c4473265e7226ed8639ac70645994d8eacf37f2984de41130c6a70cf7d6da457 |
| docs/engineering/C10-03H-HOSTED-RETENTION-AND-DELETION.md | 6593700a0badb13e8d117d2e0a0386f3412a0a99ad972fd228d7f4bfb825ad91 |
| docs/engineering/C10-04D-CLAUDE-REVIEW-INSTRUCTIONS.md | 6d10056a4612bd41096b6371a6186f13f49a2a2f75fdd308057aac26cef6b5cd |
| docs/engineering/C10-04D-KEY-ROTATION-RECOVERY.md | 98bbf3340b020558c539c248af6b2d12b8af2bf2b0737b84b3a1e1644b86ac93 |
| proofs/sync-v2-identity/supabase/migrations/20261010022507_c10_03g_encrypted_record_operations.sql | ce9381a46d825f85b3600c00a74ef5b4dbb53472e0d54356bf747f993b84a38d |
| src-tauri/migrations/0012_sync_v2_local_queue.sql | fe961f4101b238995cec1d0f5d5e5abf0428bf41b09c75acb097f4b8e7485916 |
| src-tauri/migrations/0014_sync_v2_apply_protocol.sql | ba2ee4f28552d172f4ccd49fe5e1699a5d29ffbf62d9bf0897b7ad73259e823e |
| src-tauri/migrations/0015_sync_v2_key_rotation.sql | 8f22ebb0ccb4c396beef4a49db36918bfe8d9926ea9eb52b0d03d3eac4101d80 |
| src-tauri/src/sync_v2_approved_device_transfer.rs | 62e5fd5a6cb8a208debc96b697995bb47ebe18ee1e57b419a301644e4bb86667 |
| src-tauri/src/sync_v2_key_recovery.rs | b77f33b24e7a2d4e8d0de4726844118ac6d0a31963f91a5a7fa44f40bc0d79f2 |
| src-tauri/src/sync_v2_local_queue.rs | b975196ccdc085be44d037e5b421fc52342c38cb5ee0468a1419ce106c3ff7a9 |
| src-tauri/src/sync_v2_safe_apply.rs | a029bc54a81e15a5c459b80e7a256e0d2f0d70f774f8a19ef799a11fe6d3e991 |

## Coverage and limits

I inspected the keyring/rotation, queue replay, applied-receipt, transfer and hosted write boundaries used by this proposal. The device enrollment signing boundary was also read for context; this inventory is not an exhaustive repository audit. No retirement implementation, hosted concurrency experiment, new recovery format, Windows vault operation or cryptographic benchmark was executed. This follow-up executes the fictional D1 census/authority tests described in d1-proof.md; that is not a vault or retirement implementation. Prior format test results belong to their prior commits, not this proposal.

External reference checks were limited to current official Supabase RLS/grant guidance and PostgreSQL locking guidance. They support control responsibilities, not proof of the proposed protocol. No Supabase service was changed. The analysis uses a balanced engineering profile; the founder supplied no latency, memory or storage budgets.

No secrets, customer records, recovery material, absolute file paths or real backup contents are included in the distributable packet.

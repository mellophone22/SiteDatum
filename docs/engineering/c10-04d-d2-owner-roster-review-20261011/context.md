# D2 evidence context — 2026-10-11

Repository: https://github.com/mellophone22/SiteDatum
Source base: `91973f57d9d4f9033dcf3455299046784ddf9742`; review branch `codex/c10-02e-claude-review`. Worktree was clean when inspected. Graphify map is absent in this checkout; relevant pinned source was inspected directly. No scan or certification is claimed.

## Input collection

The supplied Claude preservation-first design review of 2026-10-11 supplies the MP-2 first-device/rollback concern. Its text is in the user conversation, not a separately hashed repository file. The ten-file inventory below pins the source and prior derived policy packet inspected here; it does not pretend to hash the conversation report.

| File | SHA-256 |
| --- | --- |
| docs/engineering/c10-04d-retirement-review-20261011/proposals/safe-key-retirement.md | 57298272fab157c0cabb4f212001e9bc48143311c102c74fc511b092d5dfd442 |
| docs/engineering/c10-04d-retirement-review-20261011/review-remediation.md | f8004196d4fcaa61bbf715230049612fb3cf65f87973e1a75d91a0ce6ada1bf9 |
| proofs/sync-v2-identity/supabase/migrations/20261009200228_c10_03b_initial_device_enrollment.sql | d2068200ae7056c3e56b435997c8dda466b334acca652819ea3752181840696f |
| proofs/sync-v2-identity/supabase/migrations/20261009214234_c10_03e_approved_device_authorization.sql | 43095cf967f4e6ee87fb7d939d61ce5ca5f5278bd0ff89382840a7af6e037518 |
| src-tauri/migrations/0011_sync_v2_checkpoint_anchors.sql | f78c181dc9e58217abac287406d28d12e2868cbe9adcbaad7de2d264250f938d |
| src-tauri/migrations/0012_sync_v2_local_queue.sql | fe961f4101b238995cec1d0f5d5e5abf0428bf41b09c75acb097f4b8e7485916 |
| src-tauri/migrations/0014_sync_v2_apply_protocol.sql | ba2ee4f28552d172f4ccd49fe5e1699a5d29ffbf62d9bf0897b7ad73259e823e |
| src-tauri/src/sync_v2_approved_device_transfer.rs | 62e5fd5a6cb8a208debc96b697995bb47ebe18ee1e57b419a301644e4bb86667 |
| src-tauri/src/sync_v2_device_enrollment.rs | b9275f77cd672896af72acc372a46bc6c0127e5e508a06e96157e31f46281b3f |
| src-tauri/src/sync_v2_key_recovery.rs | b77f33b24e7a2d4e8d0de4726844118ac6d0a31963f91a5a7fa44f40bc0d79f2 |

Collection digest: `7c71d7df19e8ae93b51fce435efecaab4172c67644e3c4b0c6dfcd3e8092204a`.
Recipe: sort paths ascending; join `path + " " + sha256` with LF and no final LF; SHA-256 of UTF-8 recipe bytes. Individual hashes are exact Git-blob/LF bytes at the source base; do not dismiss a mismatch as line-ending conversion.

## Observations and limits

Native device identity is an OS-random Ed25519 seed in Windows Credential Manager, with distinct transfer keys and scope-bound signatures. Existing hosted initial/approved enrollment has owner/session/device checks and a two-device limit. SQLite anchors and workspace-keyed local receipts are not independent all-device roster history. The current receipt/ack reference inventory has nineteen references across persistence, key recovery, queue and safe apply, including tests; do not blindly change their primary keys.

No D2 owner-root format, complete chain, independent pin, schema or capability is implemented here. Source inspection is not a simulated-attack result. No Windows credential, root material, real customer data or live Supabase database was read. D2 validation is document/source correspondence and publication integrity only.

Prior local D1 regression belongs to its commit: 12 focused tests, full Rust 214 passed/five ignored, frontend 113, lint/build/check/format pass. Those results do not test D2. No new behavioral tests or cryptographic construction is claimed.

# SiteDatum D3 vault implementation — focused independent code review

Date: 2026-10-11  
Repository: https://github.com/mellophone22/SiteDatum  
Mirror: https://gitlab.com/Kikito22/sitedatum  
Branch: codex/c10-02e-claude-review  
**Exact review target: 07e8c90e1fd6598dbd162a1f3e683cb3ca439c29**  
Storage-slice base: 1bf424a1ad6d498fc51e63a09dd60bca23449135

This packet is a documentation-only handoff. Review the pinned target, not a moving branch head. Packet publication changes documentation only; it grants no implementation, deployment, root issuance, customer import, key deletion or Sync activation authority.

## What we need from this review

We have implemented the approved encrypted-file direction and tested it. We now need a critical code review of the actual archive construction, Windows file/directory boundary and recovery ceremony. This is not another broad D2 policy/owner-roster semantics round.

Read the source and challenge the claims. Return three separate verdicts:

1. Experimental archive/recovery encoding and cryptographic composition: safe to retain for further gated development, conditions required before format acceptance, or not acceptable.
2. Windows storage/private-directory boundary: identify unsafe native memory use, path/ACL assumptions, publication/interruption weaknesses and blockers before customer use.
3. Recovery-confirmation backend: identify false confirmation, replay/mismatch, secret-handling or lifecycle problems; state whether gated UI work can proceed and what restrictions apply.

An independent AI review is supplemental evidence, not a professional cryptographic audit or certification. Approval to continue gated development must not be worded as approval to ship or activate Sync. Do not claim Windows tests ran if the reviewer only has Linux.

## Source inventory and verification

[inventory.json](inventory.json) contains twelve exact Git-blob SHA-256 hashes at the pinned target. Hash Git blobs, not platform-converted checkout text. A mismatch is a blocker to evidence correspondence; do not dismiss it as newline conversion.

Collection digest: c3eaf6b9cec08170d6f752b773103a16946c4f0a1caf0dddd99b90165c1bc551

Recipe: sort entries by repository-relative path, join each `path + " " + sha256` with LF, no trailing LF, encode UTF-8 and SHA-256. Both review mirrors were verified at the target before handoff construction.

| Source | Review boundary |
| --- | --- |
| src-tauri/src/sync_v2_vault_file_prototype.rs | Secret/Archive/Pin, strict serialization/parsing, HKDF/AEAD, recovery text, DPAPI and plaintext lifetime |
| src-tauri/src/sync_v2_vault_storage.rs | Bundle framing, expected pin trust, exclusive immutable writes, bounded reads, subprocess/fault tests |
| src-tauri/src/sync_v2_vault_directory.rs | Process-token SID, native SD/ACL memory, protected root, ancestor handle lease, reparse/permission tests |
| src-tauri/src/sync_v2_recovery_confirmation.rs | One-time reveal, consuming save, secret-free pending state, fresh readback, full-set/ceremony matching |
| src-tauri/src/sync_v2_retirement_census_proof.rs | Actual twelve-column recovery and no-write-authority fixtures |
| src-tauri/src/lib.rs; Cargo.toml; Cargo.lock | Disabled default gate, dependency scope and absence of commands/callers |
| sync_v2_key_recovery.rs; sync_v2_record_codec.rs; sync_v2_record_protocol.rs; sync_v2_local_queue.rs | Existing encryption/checkpoint/read/write boundaries and refusal contracts |

Repository-relative paths are used above; the last row's files are under src-tauri/src. Follow relevant callers rather than treating an inventory row as a complete audit.

## Implementation records to read

- [Windows adapter measurements](../C10-04D-D3-WINDOWS-MEASUREMENTS.md): environment-specific inventory refusal, not a universal capacity bound.
- [Encrypted-file prototype](../C10-04D-D3-FILE-VAULT-PROTOTYPE.md): experimental framing, domains, budgets and twelve-column recovery limitations.
- [Bundle storage](../C10-04D-D3-VAULT-STORAGE.md): one immutable local package, expected pins, orphan and subprocess semantics.
- [Private root / confirmation](../C10-04D-D3-DIRECTORY-RECOVERY-CONFIRMATION.md): native ACL/lease and secret re-entry behavior.

Read these as author claims to verify, not as proof that the code is correct. The source remains experimental and gated, despite being reusable compiled code under an explicit review feature.

## Priority questions and counterexamples

### Archive and secrets

Challenge exact header/count/length framing, integer bounds, sparse versions, duplicate ordering, owner/workspace validation, domain separation, random nonce assumptions, relabelling, tampering and authenticated malformed payloads. A valid digest is not authentication or freshness. Try tampering even after recomputing public hashes.

Examine all copies/allocations and drop paths for raw secrets, decrypted entries and DPAPI buffers. Neither Zeroizing nor a library audit proves that every compiler/cipher/internal buffer is wiped. Review the native failure branches, successful buffer lifetime and size caps.

The test/review recovery text is canonical unpadded URL-safe base64 over a random 32-byte secret, not a password/KDF or a shipped human recovery-code contract. Check canonical decoding, exposure-once behavior and error/cancellation paths. Avoid printing or persisting secret text.

### Storage and independent pins

Challenge local bundle wrapper lengths and inner/outer authenticity; swap wrappers, local/recovery markers, owners and generations. Challenge oversize/truncation and an existing destination. Review exclusive sharing, same-handle readback and reopen after plaintext secret discard.

ExpectedBundle::from_independent_pin does not verify provenance. Durable independent pin custody remains unimplemented. Evaluate whether the API clearly preserves that boundary; do not count an untrusted hash or a current-looking filename as freshness.

A fully written orphan may authenticate after a lost response. No active-generation pointer, orphan adoption or multi-store journal is implemented. Test evidence must not be promoted into such authority. Review both partial writes and completed-but-unacknowledged files.

### Windows private roots

Review every unsafe call/cast and pointer lifetime: aligned TOKEN_USER storage, SID conversion, GetSecurityInfo-owned buffers, ACE header/type/size before widening, NativeMemory cleanup and DPAPI output handling.

Challenge the protected DACL's current-user/SYSTEM-only claim, existing weak roots, ownership mismatch, permission drift, null/extra/duplicate ACEs, inherited flags and unsupported filesystem behavior. Windows-native execution is needed to validate platform behavior; static Linux inspection does not substitute for it.

Challenge ancestor/final reparse points, junctions, path normalization, embedded NUL, streams, relative/device/UNC inputs, race windows around creation/acquisition and lease release. Ancestor handles intentionally deny competing write/delete opens. The lease must be short-lived; assess compatibility/denial risks. Same-user, administrator/SYSTEM and privileged/kernel compromise remain residuals, not properties this module claims to solve.

### Recovery ceremony

Try confirming solely from a saved flag, wrong/corrupt copy, wrong secret, a stale copy, a different ceremony, scope, generation, retained version set or changed key bytes. Confirm failure must not advance state; cancellation must not delete a copy.

A successful receipt is in-process readback evidence only. It is not verified offline custody, an independently durable pin, a current roster, an all-device barrier or retirement authorization. Review whether code/types/copy preserve those distinctions.

## Tests and safe reviewer execution

Last implementation verification at this source reported **250 Rust passed, 0 failed, 9 ignored**; **114 frontend passed in 35 files**, fmt/lint/build and both compile modes passed. That is prior executed implementation evidence, **not a new test run by packet preparation**. Clippy was unavailable.

The code is unchanged since that verification. This handoff preparation executed source/feature/caller inspection, exact Git-blob hashing and remote-head correspondence. It did not rerun cryptographic, Windows or frontend tests.

Suggested commands from src-tauri:

```text
cargo fmt --check
cargo test --locked
cargo check --locked --lib
cargo check --locked --lib --features sync-v2-vault-review
```

Default Windows tests create uniquely scoped fictional encrypted files/directories and test ACLs/junctions, then clean exact fixture paths. They do not enumerate/modify real credentials or customer files. The ignored subprocess probe is invoked through its parent tests; other ignored credential drills must **not** be enabled by a blanket --ignored run.

Keep repository/service state read-only. Use disposable fictional fixtures only. If adding adversarial tests, use a disposable review checkout and report/revert exact temporary changes. Do not dump Credential Manager, account tokens, native payloads, real database contents, secrets or recovery material. No purchases, paid resources, provider changes, main merge, release or activation.

Frontend commands are npm test, npm run lint and npm run build from the repository root. No UI was added by these storage slices; there is no customer recovery screen to evaluate yet.

## Expected report

For each issue provide scope/classification, severity calibrated to reachability, exact source evidence, reproducible counterexample or test, impact, minimal correction and retest. Separate an observed defect from an unimplemented prerequisite or declared residual. Document negative controls and avoid counting a zero-test success as executed evidence.

State which checks actually ran, operating system, exact revision, counts/ignores, temporary fixture changes, tests not run and reviewer limits. Explicitly distinguish:
- implemented but experimental/gated;
- source defect;
- unimplemented acceptance gate;
- residual outside the claimed threat boundary;
- approved continued development versus format acceptance versus shipping.

Give the three separate verdicts requested above and a concrete minimal blocker list. Do not reopen unrelated founder policies unless the actual implementation contradicts them. ForkResolution, D2 encoding, D4 hosted floor/barrier and D5 production journal are not implemented in this slice and do not become approved by this review.

## What stays blocked

No customer recovery UI with real keys, accepted shipping magic/format, production import, key pruning or Sync activation follows from the handoff. Recovery UI work should remain gated until relevant review blockers are resolved. D3 final budgets/custody, durable pins, Windows account/roaming/reboot and disk/power-loss drills remain open. D2/D4/D5 and D6/D7 remain separate gates.

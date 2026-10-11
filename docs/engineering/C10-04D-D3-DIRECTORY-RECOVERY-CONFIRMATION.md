# C10-04D D3 — private-directory and recovery-confirmation backend

Date: 2026-10-11  
Source commit: b524cdee2f51810d1568c5e7a7129926d7af7ff8  
Base: eca575e9a526425648e621e25df668969ae11e23  
Branch: codex/c10-02e-claude-review

## Scope and decision

We continued the approved encrypted-file direction with a private-directory boundary and recovery ceremony backend. These are implemented and tested, but remain under tests/the disabled-by-default sync-v2-vault-review gate. No application root is created automatically. No UI, clipboard, Tauri command, migration, transport, customer-file operation, key removal or Sync activation was added.

This is the next slice after [bundle storage](C10-04D-D3-VAULT-STORAGE.md), not D3 production acceptance or a replacement for D2/D4/D5 authority and ordering. Existing file encryption and experimental markers are unchanged.

## Private directory

The Windows backend creates only an explicitly requested, previously absent leaf directory. It supplies the security descriptor at creation rather than creating a permissive folder and repairing it afterward. The owner is the current process-token user. The protected DACL grants full file access, inheritable to child files/directories, only to that user and SYSTEM.

We obtain the current SID through the native process-token API, without credential enumeration or logging SID/account information. SID/control/ACE inspection uses handle-based GetSecurityInfo. Acceptance requires the expected owner, protected DACL and exactly the two expected allow entries, masks and inheritance flags. Unexpected inherited, permissive, duplicated or extra access entries refuse. Existing folders are never silently repaired or adopted on a failed check.

We walk the absolute local-drive path one component at a time. UNC/device/relative/parent-traversal inputs refuse; embedded NUL, alternate-stream colon and trailing dot/space components refuse before native directory creation. Each opened component must be a directory without a reparse attribute. Handles use backup/open-reparse semantics and share-read only, denying competing directory write/delete opens while held. Handles are retained through vault operations, so accepted ancestors cannot be renamed/replaced through ordinary competing opens during the lease. We also recheck held directory attributes and the root DACL before generating a vault path.

Generation filenames are derived only from a non-nil UUID, never a user filename, record name, path segment or provider payload. Create/open delegates to the existing immutable, bounded storage boundary. The root is accepted again when reopened, and a weakened DACL blocks later operations.

The directory object is a **short-lived operation lease**, not something to hold indefinitely in application state. Holding ancestor handles can interfere with competing filesystem operations; duration and application compatibility must be evaluated before UI integration. This is not a guarantee against the same Windows user, administrators, SYSTEM, kernel/filesystem filters or privilege-based access. Other ancestors' ACLs are not promoted into account-authority evidence.

## Recovery ceremony backend

The state flow is RecoverySetup → PendingRecovery → ConfirmedRecovery.

Setup generates a fresh 32-byte random recovery secret. The test/review representation is 43-character canonical unpadded URL-safe base64. It is not a password, email token, owner root, shipped recovery-code contract or authentication scheme. Wrong but well-formed secrets fail authenticated file opening. Padding, malformed/noncanonical encodings and oversized text refuse before acceptance.

Secret text is exposed through a Zeroizing string at most once per setup object; secret types have no Debug/Serialize implementation. No clipboard or secret printing is implemented. A future UI must not infer that this API guarantees screenshot, clipboard, browser-memory or off-computer custody safety.

Saving consumes the setup, encrypts and verifies the new recovery file through the existing storage API, then drops the original archive/setup secret. The pending object retains only scope/generation, the file pin, a full retained-key-set comparison digest and a random local ceremony identifier. It does not retain the recovery secret.

Confirmation requires re-entered secret text and a fresh authenticated reopen of the saved file. The recovered complete manifest must match the expected scope, generation, original versions and key bytes. Wrong/malformed secrets or corrupt copies leave the pending state unconfirmed, allowing a later valid attempt. A successful confirmation can only occur once. Its receipt must match the caller's local ceremony ID as well as the full archive; a different generation, owner, retained set, changed key or ceremony ID refuses matching.

The comparison digest is in-process only and never logged or published as key/census authority. This ceremony UUID is **not** an allocated owner/workspace retirement event or a roster epoch. No receipt can deserialize from a saved flag or construct a retirement barrier. Cancelling/dropping ceremony metadata does not delete the saved customer copy.

The backend demonstrates knowledge of the secret and successful file readback. It does not prove physical offline custody, the latest global state, future storage durability, a complete historical manifest discovered from backups, or an independently durable pin. Those remain separate gates.

## Executed tests

Four directory tests passed:

- Restricted root creation/opening and immutable generation reopening succeed. Existing root creation refuses. Ancestor rename and competing write opens refuse while the lease is held.
- An ordinary inherited directory refuses without repair. Relative/UNC and embedded-NUL creation inputs refuse; the NUL-truncated prefix is not created.
- A real NTFS junction created solely in a fictional test directory refuses both as root and as an ancestor. A would-be redirected target vault is never created. The exact junction is removed without recursively deleting its target.
- Explicitly weakening only a fictional root's DACL blocks later operations and reopening; the backend does not automatically repair it.

Three ceremony tests passed:

- At 1, 8, 100 and 1,000 retained keys, revealing twice refuses; saving alone is not confirmation; wrong secret refuses without advancing state; re-entering the correct secret opens the file and confirms the entire set.
- Confirmed receipt matching refuses a different generation/owner/count/key byte/ceremony ID, and a second confirmation attempt refuses.
- Cancellation preserves the copy; malformed and padded secrets refuse. A deliberately corrupted saved copy cannot mark the ceremony confirmed.

These last cases are grouped into three test functions. Counts below are actual Rust harness counts, not the number of assertions.

An early negative fixture accidentally attempted to construct 1,001 keys beyond the prototype budget and failed in fixture setup. We corrected it to use a different valid in-budget set, reran verification and removed the one exact encrypted fictional file left by that failed fixture after checking its path and size. Recovery tests now own a newly created UUID-scoped directory and exact-file cleanup guard. No claim of cryptographic failure is inferred from the fixture setup error.

All successful tests removed their explicitly owned fictional artifacts. ACL modification occurred only in the newly created weak-permission test root; no customer directory ACL or Windows account/profile was changed. No real credential or project data was accessed. No secret was printed or written in plaintext.

## Verification

Executed at final source:

- cargo test --locked: **250 passed, 0 failed, 9 ignored** across Rust targets.
- cargo fmt --check: pass.
- cargo check --locked --lib: pass.
- cargo check --locked --lib --features sync-v2-vault-review: pass.
- npm test: **114 passed across 35 files**.
- npm run lint: pass.
- npm run build: pass.
- git diff --check: pass.

The child-process probes from the preceding slice still execute through their parent fixtures. Ignored credential-store drills were not rerun. Clippy remains unavailable; no Clippy pass is claimed. No UI changed, so visual review does not apply.

No dependency version or Cargo.lock changed. We enabled additional documented Windows API namespaces on the already pinned optional/dev windows-sys dependency; System IO is dev-only for the real junction fixture.

## Remaining acceptance gates

We completed this directory/confirmation backend slice, not a customer recovery screen or production approval. Next vault work includes the gated recovery UI and privacy-safe handling, the independent format/custody review, final resource budgets, durable independent pin custody and restart-aware ceremony state. User-interface consent must distinguish readback from verified offline custody and explain loss of both local protection and recovery material.

The tested ACL is structurally restricted, but this run does not impersonate another Windows account or test administrator bypass, reboot, roaming, account reset, disk failure or power loss. The no-delete/no-write ancestor lease and native ACL implementation need review and application compatibility testing. No claim is made that ownership/pins establish current roster completeness.

D2 authority verification, D4 all-device coordination/hosted floors and D5 multi-store journal remain open; D6 retirement and D7 activation remain blocked. Ordinary local work and the production eight-key ring are unchanged.

## Primary references

[Microsoft CreateDirectoryW](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-createdirectoryw) documents applying a security descriptor when creating a directory. [GetSecurityInfo](https://learn.microsoft.com/en-us/windows/win32/api/aclapi/nf-aclapi-getsecurityinfo) documents handle-based descriptor retrieval and its race limitations. [REPARSE_DATA_BUFFER](https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/ntifs/ns-ntifs-_reparse_data_buffer) and [FSCTL_SET_REPARSE_POINT](https://learn.microsoft.com/en-us/windows-hardware/drivers/ifs/fsctl-set-reparse-point) describe the test-only junction construction. These platform sources do not certify this application boundary or replace the remaining review.

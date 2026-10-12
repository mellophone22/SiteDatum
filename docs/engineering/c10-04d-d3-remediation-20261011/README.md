# C10-04D D3 remediation — Windows verification record

Date: 2026-10-11
Source commit: `66ea74dd511435b7606da716f6e2f25a4293fc69`
Parent: `4ed85cee33a23893e2a42dc2919e3bc70edc1529`
Reviewed source before remediation: `07e8c90e1fd6598dbd162a1f3e683cb3ca439c29`
Branch: `codex/c10-02e-claude-review`

## Outcome and authority boundary

The six focused code findings and five cleanup observations were addressed in source and tested on Windows. This is implementation and regression evidence for the dormant D3 development slice, not a professional security certification or format/customer-release approval. No recovery UI, production import, root issuance, key pruning, Sync activation, main merge, release or customer-data migration was added.

Ordinary local work remains independent of Sync. Existing dormant module feature/test gates remain in place. No customer credential, keyring content, recovery artifact or project data was used. Subprocesses received fictional fixture paths and public fixture metadata only.

## Remediation mapping

| Finding | Change | Executed evidence |
| --- | --- | --- |
| W-1 unsafe ACL/SID read | Replaced native unbounded GetSecurityInfo/GetAce/EqualSid traversal with GetKernelObjectSecurity into a caller-owned, initialized aligned buffer. Validate returned length against allocation. Parse self-relative descriptor, owner offset/SID, ACL header/size/count, exact ACE ranges/types/flags/masks and SID revision/count/length through bounded Rust slices. No native SID comparison sees hostile bytes. | Exhaustive truncation of a valid descriptor, malformed owner/ACL/ACE/SID cases, valid control, native restrictive-root tests and a malformed final ACE placed against a PAGE_NOACCESS guard page pass. |
| W-2 long-lived ancestor leases | Root object stores only trusted configured boundary/path, identities and immutable profile. Reacquire boundary-to-root handles inside each storage operation, recheck identities and DACL, hold through storage open/readback/reopen, then release on success or failure. Creation reuses parent handles and opens only the new leaf. No volume-root lease. | Isolated subprocess test measures zero additional idle directory handles, exactly two during the boundary+leaf operation, and return to idle after failure. Second process can open boundary for write when idle, cannot during operation, and can afterward. |
| C-3 unmarked Pin construction | Private fields in nested pin module. Codec-only verified-seal constructor; public-to-crate provenance-named from_independent_custody validates generation. It does not discover or verify custody. | Custody positive control compiles; outside literal fails specifically E0451; outside verified-seal constructor fails specifically E0624; zero/out-of-range generation refused. |
| C-4 duplicated framing bounds | Share HEADER/LIMIT/MIN_FILE/MAX_FILE/MAX_DPAPI from codec; storage aliases these rather than maintaining independent literals. Bundle bound derives from shared bounds; compile-time minimum/maximum relationship asserted. | Default and review-feature cargo checks; maximum archive/storage tests; malformed/oversized inputs; full Rust suite. |
| R-1 misleading durable-confirmation name | Renamed to InProcessReadbackReceipt, must_use, type-level documentation, and proves_readback_of. No Serialize/Debug or persistence/IPC API. Explicitly not durable custody, independent pin, roster, barrier or retirement/readiness authorization. | Existing changed-scope, changed-generation, changed-key/count, ceremony mismatch and one-shot tests pass. Source inspection finds no external lifecycle or IPC callers. |
| R-2 missing ceremony substitutions | Added confirmation tests against another seal for the same archive AND recovery secret, a local bundle instead of recovery archive, and intact correctly pinned recovery bytes with mismatching expected full-set manifest. | Every substitution refuses without advancing confirmed; restoring the correct expected manifest confirms as a positive control. |
| O-1 generic bounded writer mistaken for parser | Renamed write_new_bounded and documented bounded IO only, including DPAPI blobs. Authentication remains at open. | Existing no-overwrite/truncation tests and all twelve-column total-loss drill pass through renamed helper. |
| O-2 stable unkeyed secret fingerprint | Full-set hash is passed through HMAC-SHA256 via HKDF extract under independently random per-ceremony comparison key. Opaque RetainedManifest has no byte-export, Debug or Serialize API. Comparison key uses Zeroizing; intermediate hash is wrapped in Zeroizing. | Same archive in separate ceremonies produces distinct manifests; altered full-set bytes/count refuse. Comparison keys are not record/recovery secrets and confer no write authority. |
| O-3 conflated failures | DestinationExists distinguished from Storage. ReadbackMismatch distinguished from Incomplete, retained through write completion and reopen verification. Failures leave new files unconfirmed for inspection. | Explicit destination collision and injected integrity readback failure produce distinct outcomes; interrupted writes preserve prior generation; no automatic deletion. |
| O-4 mutable self-referential token backing | Allocate immutable Box<[usize]> BEFORE GetTokenInformation writes the interior SID pointer. Validate returned initialized length. SAFETY comment records allocation/pointer invariants; bound token SID before native conversion. | Native restrictive root/open/drift tests pass. No resize API remains on Profile token storage. |
| O-5 tautological inner pin | Replaced self-computed inner Pin with private-field AuthenticatedLocalPayload constructed only after independently supplied OUTER bundle digest validates. Codec consumes that typed verified payload; comment denies a second freshness check. | Corrupt outer bundle, recomputed counterfeit outer digest with invalid DPAPI/AEAD, wrong scope/kind and old bundle substitutions refuse. Fresh read-only candidate review found no unchecked construction path. |

## Actual Windows checks

Final `cargo test --locked`: **258 passed, 0 failed, 9 ignored**:
229 unit + 29 integration passed; five unit and four integration drills remain registered as ignored.
Focused vault modules: 24 passed, one registered child probe ignored; recovery confirmation: five passed.
The storage parent tests invoke only the uniquely scoped child reopen/interruption probes explicitly; a blanket ignored-test run was not used.

- cargo fmt --check: pass.
- cargo check --locked --lib: pass.
- cargo check --locked --lib --features sync-v2-vault-review: pass.
- npm test: 114 passed across 35 files.
- npm run lint: pass.
- npm run build (TypeScript + Vite): pass.
- git diff --check: pass.
- Existing containment integration test: pass; default features remain empty; no Tauri command or application lifecycle caller was introduced.
- Fresh required pre-fix boundary investigator and post-candidate read-only reviewer completed. Candidate reviewer found no confirmed surviving bypass or actionable regression. These are source-only supplemental checks, not another executed protocol or cryptographic audit.

The full final Rust run includes the fictional total-loss recovery census at 1, 8, 100 and 1000 retained keys, resolving all twelve ciphertext columns without granting write authority. This does not establish a final product budget.

## Negative controls and measured limitations

Initial process-handle measurement included Windows lazy security-provider initialization (13 persistent process-global handles) and later subprocess/pipe initialization (three). The final test isolates its process and warms those APIs before establishing its baseline; it measures lease deltas, not an assumption that Windows has no internal cached handles.

A real Windows compatibility restriction was observed: child-file rename is refused while the directory lease is held, even though child creation and deletion succeed. The test requires that denial during the operation and successful rename afterward. The boundary can be opened for write from a second process before/after the lease, but not during it. This is not a claim that all installer/backup workloads are compatible. No actual MSI installation was performed.

The malformed final ACE is executed adjacent to an inaccessible page, so any read past the actual buffer would fault the test process. Application Verifier/ASan was **not** run; the guard-page test is the executed memory-boundary check, not a claim of verifier coverage. Rust Clippy is not installed and was not run. Formatting, compilation, frontend lint/typecheck and runtime tests were run.

Boundary ancestors above the configured application-data directory are trusted OS/caller context, not newly claimed protection against administrator, SYSTEM, kernel or same-user compromise. A scoped lease prevents rename/reparse substitution within that boundary while the operation uses path opens. File identities are checked on reacquisition.

## Still-open product gates, not these six remediation defects

Final preservation-format acceptance and independent cryptographic review; independently durable pin custody/current-generation pointer; D2 canonical encoding and owner-root ceremony; D4 floor/barrier/fences and hosted membership reconciliation; D5 ordered multi-store publication/recovery journal and hosted safe-integer upper-bound enforcement; final storage/performance budgets; account/roaming/reboot and physical disk/power-loss drills remain separate obligations. D6 mutation and D7 activation remain blocked.

No UI flag or persisted 'confirmed' marker may substitute for verified file readback or durable independent custody. InProcessReadbackReceipt must never become retirement authority. A future recovery UI can be developed only within these explicit dormant/gated constraints; this change did not add UI.

Cryptographic clarification: authenticating the header and using it as HKDF salt defends tampering/relabeling. It does NOT make an identical full-header nonce collision harmless: an identical header under the same secret repeats derived key and nonce. Random 192-bit nonce uniqueness remains an assumption, not a property proven by these tests.

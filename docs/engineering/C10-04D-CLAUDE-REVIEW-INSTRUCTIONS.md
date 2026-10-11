# C10-04D — Independent review instructions

Repository: https://github.com/mellophone22/SiteDatum
Branch: codex/c10-02e-claude-review
Format-slice base: c885bc3c9fa8070b36fd4f10a0e0cd4ce3af12ef
Original C10-04D review target: 7cdf0610b63c8fcd3611c650b86c85975891ad91
Original base: ce8f431da38a0571cd9cf631ca83501c7d15eebe

Pin the exact implementation commit supplied with this packet, not a moving tip.

## Copyable request

Act as a critical independent application-security and cryptography reviewer.
Review the pinned C10-04D commit against the base above. Read AGENTS.md, ADR-019,
the Sync v2 threat model/inventory, prior review/remediation requirements and
C10-04D-KEY-ROTATION-RECOVERY.md. Do not alter the repository or services, use
customer data, print secrets, purchase anything or activate Sync. Return a
source-linked Markdown report with severity, reproducible evidence, tests
actually run and an explicit approve / conditional / reject decision.

Inspect sync_v2_key_recovery.rs, record_codec, record_protocol, local_queue,
paging, safe_apply, persistence and migration 0015. Check:

1. Strict parsing/bounds, scoped authenticated header, exact key versions,
   tamper/downgrade behavior, nonce generation and HKDF domain separation.
2. Generated 256-bit entropy, printable checksum semantics, zeroization and
   absence of secrets from Debug/Serialize, SQL, logs and diagnostics.
3. Protected key persistence before encryption; atomic rotation, partial
   receipts/restart/exact retries, active-version writes vs retained reads,
   missing keys, conflicts and capacity refusal without losing local work.
4. Live/backup-bound export, independently trusted floors, stale/equal-counter
   rejection and durable minimum installation. Recovery must never claim
   newest provider state.
5. Windows storage scoping/write-read verification, create_new and durable-write
   behavior including partial failures.
6. The unscoped array compatibility adapter remains proof-only; no future runtime
   lifecycle should use it.
7. Cross-device conflict/revocation limitations, retained compromised keys,
   full-keyring/anchor transfer and remaining hosted/UX work are honestly stated.

Run cargo fmt --check and cargo test --locked from src-tauri. Critically review
adversarial test gaps. The ignored Windows protected roundtrip may run only on
Windows with its own fictional credential; never dump Credential Manager or
recovery material. Frontend checks: npm test, npm run lint, npm run build.
Distinguish validated defects from speculation; identify conditions for accepting
this slice and eventual activation. AI review is supplemental evidence, not a
professional certification or guarantee.

## Gate

### Additional compactable-format review

The founder chose safe compaction. Critically review the pinned new format
slice and its diff against c885bc3. Read the 2026-10-10 conditional review and
the subsequent remediation as context. Inspect SDREC002 domain separation,
strict SDREC001 read compatibility, unchanged SDR1 checksum, SDKR0002 protected
storage tagging/legacy handling, exact length/count/scope/version constraints,
no renumbering, fail-closed overflow and read/write client compatibility.

Verify that simulated test key removal is not a runtime retirement API. Sparse
format support must not be represented as closure of complete compaction. Review
the retirement census, signed all-active-device applied barrier, authenticated
hosted/replay/bootstrap floor, historical backup/recovery policy and crash-safe
cross-storage ordering requirements. State which require founder policy approval
and which protocol proofs are missing. Do not approve key destruction based on
a current-snapshot receipt or staging acknowledgement alone.

Return separate verdicts for (1) the format foundation and (2) the unimplemented
retirement design/activation prerequisites. Test old/new file tampering,
downgrade, sparse/high versions and all native regressions. Windows tests use
only their uniquely scoped fictional credential and must never print secrets.

Send the report back to the founder for remediation and explicit acceptance.
No main merge, release, production deployment or Sync activation follows
automatically from this review.

## Format-condition follow-up — 2026-10-11

The 2026-10-10 review of 579cfed approved the format with conditions and did NOT
approve retirement implementation. Review the supplied exact follow-up commit
against `579cfedc2c5f670db22ce440a15e03f8b44f4c03`, not a moving branch tip.

Verify the new content-free `SYNC_V2_RECOVERY_NEWER_FORMAT` refusal is an
unauthenticated diagnostic only. Known-format corruption/authentication failure,
wrong recovery scope, malformed lengths and unknown tags must still refuse;
exact legacy frames with SDKR0002/0003 UUID prefixes must remain readable.
No older installed binary can be retrospectively fixed. The published supporting
application version and release/transfer negotiation remain activation gates.
Verify legacy emission is structurally cfg(test)-only, production export emits
SDREC002 for all resident counts, and strict legacy reads still pass. The
create-new persistence helper may copy existing bounded legacy ciphertext; this
does not generate, re-encrypt or downgrade a recovery file.

Check the explicit twelve-column census against migrations 0012/0014/0015,
including nullable baselines/pending/resolution and resolved conflict rows.
Review the plan's separate owner/workspace/epoch/device barrier state as an
alternative to changing the existing single-local-device receipt tables. No
migration or runtime barrier/removal implementation is included. Assess roster
trust, sparse retained-set binding, typed retirement authority and the durable
quiesce-and-fence epoch spanning enrollment, restore, writes/conflicts and
recovery generation. Five founder policy decisions remain unresolved; do not
construe this plan as their acceptance or approval to destroy keys.

Return separate verdicts for condition remediation and plan prerequisites.
Run native/frontend checks and adversarial format cases; clearly distinguish
local checks from CI and Windows from Linux evidence. Nothing may activate Sync.

# C10-02E independent-review remediation

**Status:** Conditionally approved for C10-03 by independent AI re-review —
2026-10-09

## Review being answered

This change answers the independent AI second-opinion report dated 2026-10-08
for source commit `738b72dd6d2b896ae75cc2106f8298dda0c3fd3f`.
The report returned **NOT APPROVED FOR C10-03** with two High findings, ten
Medium findings, and lower-severity observations. It explicitly stated that it
was not a professional security certification, penetration test, or formal
cryptographic audit. This response preserves that qualification.

The exact immutable remediation source commit and archive digest are recorded
in the subsequent handoff commit so this document does not claim a recursive
commit identity.

Sync remains unavailable to customers. This package does not enable
`CommercialFeature::MetadataSync`, create a hosted Sync project, add a Sync UI,
upload project content, or modify billing.

## Blocking findings

### H-1 — authenticated device transfer

Resolution:

- HPKE now uses RFC 9180 Auth mode with the source device static keypair.
- The target uses an independently retained source public key and enrollment
  expectation; no source identity is accepted from the relay.
- Owner, workspace, source device, target device, enrollment ID, workspace-key
  version, expiry, and target public key must all match local expectation.
- The six-digit comparison code is derived from the HPKE exporter secret with
  a versioned exporter context, not from public transcript values.
- Every attempted approval remains single-use in the proof model.
- Fresh attacker-created envelopes with substituted source identity or any
  substituted context field are rejected even when their AAD is internally
  consistent and the attacker knows the code derived from its own envelope.

The proof remains test-only and its cryptographic dependencies remain
development-only. Durable server-side attempts/rate limits remain a C10-03
hosted-boundary requirement.

### H-2 — legacy plaintext Sync containment

Resolution:

- `require_feature(CommercialFeature::MetadataSync)` is now enforced at the
  shared legacy network/data command boundary and denies Precommercial, Free,
  and Pro access with `SYNC_DEFERRED`.
- `get_cloud_sync_availability` is a read-only check. It no longer reads a
  credential as authorization and never persists eligibility.
- A meaningless Credential Manager entry cannot enable Sync.
- Disconnect remains callable as a privacy cleanup action, deletes the local
  credential, and records an explicit local opt-out without deleting Sync
  history, project records, or documents.
- The retired live legacy Supabase URL/key are removed from production source.
  Disabled `.invalid` coordinates remain so accidental internal calls cannot
  reach a service.
- A clean optimized release build was scanned: executable, DLL, RLIB, and LIB
  outputs contained neither retired identifier and the executable contained
  only the disabled endpoint marker.
- Legacy provider failures retain stable status codes only; raw response bodies
  and submitted email values are not written to technical logs.

## Confirmed Medium and lower findings addressed now

- **M-1 record key-version binding:** `workspace_key_version` is part of record
  AAD, the deterministic vector was regenerated, wrong-version decryption is
  rejected, and ADR/inventory text matches the proof.
- **M-2 set-level rollback/omission:** ADR-019 now requires an authenticated
  encrypted workspace checkpoint, monotonic counter/digest floors on every
  device, transfer/recovery anchors, and explicit total-loss freshness limits.
  Implementation and concurrency tests remain correctly assigned to C10-04.
- **M-8 legacy token/session exposure:** the legacy network surface is
  unreachable behind the centralized kill switch, credentials are not
  authorization, disconnect is reversible cleanup, and live coordinates are
  absent from clean release artifacts. No effort was spent extending the
  retired token lifecycle.
- **M-9 destructive legacy import:** snapshots missing any required table are
  rejected before a transaction; SQLite BLOB values now fail closed instead of
  silently becoming JSON null. Tests prove the existing workspace remains.
- **F-1 log leakage:** legacy email and raw provider response bodies were
  removed from technical error details.
- **F-11 malformed JWT UUID claims:** a private fixed-search-path parser returns
  null for missing or malformed claims, so RLS fails closed without statement
  errors.
- **F-12 privilege coverage:** pgTAP now asserts explicit table and function
  privileges for authenticated and anonymous roles.
- **F-13 recovery format split:** both proofs use
  `sitedatum.sync-v2.total-loss-recovery.v1` and the recovery nonce is generated
  internally with the operating-system CSPRNG.
- **F-14 plaintext key copies:** recovered fixed-size workspace-key buffers are
  allocated under `Zeroizing` before copying.

## Findings intentionally assigned to later gates

The review correctly identified several components that do not exist yet. They
must not be disguised as C10-02 proof fixes:

- production nonce allocation, persistence, and crash/retry behavior — C10-04;
- native identity-to-Sync claim issuance and validation — C10-03;
- the full hosted envelope, monotonic versions, idempotency, tombstones,
  ciphertext limits, deletion cascades/receipts, and server-side enrollment
  attempt enforcement — C10-03 and C10-05;
- device proof-of-possession, revocation-triggered workspace-key rotation, and
  final token shape — decision required before C10-03 closes;
- consent, retention/deletion UX, restore drills, two-profile beta, and
  operational runbooks — C10-05 through C10-07; and
- the inherent same-user Windows compromise, provider metadata/timing leakage,
  provider availability, human comparison/recovery behavior, and AI-review
  limitations remain explicit residual risks.

None of those items is enabled or represented as complete by this remediation.

## Verification evidence

- `cargo fmt --check` — pass.
- `cargo test --locked` — 90 passed, 0 failed, 1 deliberately ignored live
  Windows Credential Manager drill.
- focused authenticated-transfer tests — 9 passed.
- focused record/recovery cryptographic tests — 5 passed.
- legacy containment tests — 2 passed.
- disposable local Supabase pgTAP — 31 passed, 0 failed; local containers and
  volumes then removed with `supabase stop --no-backup`.
- frontend Vitest — 99 passed across 31 files.
- ESLint — pass.
- TypeScript/Vite production build — pass.
- clean locked optimized Rust release build — pass.
- clean binary scan — retired legacy project/key identifiers absent; disabled
  `.invalid` endpoint marker present.

## Gate state

The independent 2026-10-09 re-review of immutable source commit `2009675`
confirmed H-1 and H-2 closed and returned **CONDITIONALLY APPROVED FOR C10-03**.
The supplied report's SHA-256 is
`FCFA1A2167B269623AFADBE71D165A9797663942FE11DAD8797D4819B5CF802B`.
It identified one new Low fail-closed availability/error-hygiene issue and
assigned the remaining identity, hosted-envelope, deletion, enrollment,
device-proof, OAuth, session, keyring, nonce, checkpoint, consent, and recovery
conditions to C10-03 through C10-07. This approval authorizes isolated C10-03
implementation only. It does not authorize a hosted production resource,
customer Sync, general availability, or reuse of the retired token lifecycle.


# C10-02 cryptographic and identity proof plan

**Status:** C10-02A through C10-02D complete; C10-02E independent AI review returned NOT APPROVED and remediation is awaiting re-review

## Selected design

The founder approved two decisions on 2026-10-08:

1. Workspace recovery uses a customer-held, randomly generated 256-bit recovery secret plus explicit device approval. SiteDatum services store only encrypted key envelopes and cannot derive the workspace key.
2. Identity starts with a disposable OAuth 2.1/OIDC authorization-code-with-PKCE proof. A separate dedicated Sync account is the fallback if native ownership mapping, revocation, deletion, or no-additional-cost requirements fail.

Neither decision enables Sync. `CommercialFeature::MetadataSync` remains denied, `SYNC_DEFERRED` remains enforced for fresh installations, and the legacy boundary is not expanded.

## Cryptographic suite under proof

- Record and recovery-envelope encryption: XChaCha20-Poly1305 from the mature RustCrypto `chacha20poly1305` crate.
- Recovery-key separation: HKDF-SHA-256 with an explicit protocol label and workspace-specific salt.
- Device-to-device key transfer: RFC 9180 HPKE using DHKEM(X25519, HKDF-SHA-256), HKDF-SHA-256, and ChaCha20-Poly1305; this is a later isolated proof, not part of the first slice.
- Secret lifetime: bounded secret buffers with zeroization where supported, followed by a Windows Credential Manager proof for device-private material.
- Protocol serialization: versioned canonical binary fields with explicit lengths. Owner, workspace, entity, record, workspace-key version, revision, and protocol version are authenticated data.

A high-entropy generated recovery secret is key material, not a human password. It must not be weakened through a password-style transform, sent to SiteDatum, logged, or included in support diagnostics. The printable encoding and transcription/error-detection format remain a separate reviewed decision.

## Work packages

### C10-02A — Offline record and recovery proof

- Keep all code in an integration test so it cannot enter the production command graph.
- Freeze a canonical authenticated-data encoding.
- Verify an independent RFC 5869 HKDF vector.
- Freeze deterministic implementation vectors for record and recovery-envelope encryption.
- Reject modified ciphertext, wrong record context, wrong workspace, and wrong recovery secret.
- Model nonce reuse as a hard failure.

Exit: focused Rust tests pass and production binaries/UI remain unchanged.

**Evidence:** Complete. The isolated integration proof passes deterministic
record/recovery vectors, context and tamper rejection, customer-held recovery,
and modeled nonce-reuse refusal. Its dependencies remain development-only.

### C10-02B — Device key storage and lifecycle proof

- Generate device keys with the operating-system CSPRNG.
- Store device-private material through Windows Credential Manager with a versioned entry name.
- Prove read, replacement, revocation, deletion, and reinstall behavior with fictional data.
- Review process-memory exposure and zeroization limitations on supported Windows versions.

Exit: no private key appears in SQLite, logs, repository files, crash text, or client diagnostics.

**Implementation boundary:** The lifecycle proof is a Windows-only integration
test with a unique fictional service/account, an automatic cleanup guard, and an
ignored live test. Normal builds and test runs do not touch Windows Credential
Manager. The proof is not imported by the Tauri library, exposed as a command,
or reachable from the UI.

**Evidence:** Complete on the supported Windows workstation. The explicitly
selected live test proved create, read, version replacement, fresh-handle
reopen, revoke/delete, and post-delete `NoEntry`. No credential value was
printed or retained by the proof.

### C10-02C — Approved-device transfer and recovery drill

- Prove RFC 9180 authenticated-mode HPKE device enrollment using a short owner-visible comparison code derived from the HPKE exporter secret.
- Bind envelopes to owner, workspace, target device, key version, and expiry.
- Reject substituted keys, replay, expired enrollment, revoked devices, and wrong comparison codes.
- Exercise surviving-device transfer and total-device-loss recovery with the customer-held secret.

Exit: both drills recover the same workspace key locally without server decryption authority.

**Evidence:** Remediated after independent review in an isolated integration proof. RFC 9180 authenticated-mode HPKE with
DHKEM(X25519, HKDF-SHA-256), HKDF-SHA-256, and ChaCha20-Poly1305 recovered the
same fictional workspace key on an approved target device. The proof binds the
owner, workspace, source device, target device, key version, one-time enrollment
ID, and expiry against target-held enrollment state; it rejects fresh
attacker-sealed source/context substitutions, a substituted target key, replay,
expired enrollment, revoked target, wrong comparison code, and modified context. A separate
customer-held-secret drill recovered the same key class after total device loss.
The `hpke` dependency is development-only and independent review remains an
explicit C10-02E gate.

### C10-02D — Disposable identity proof

- Use disposable/sandbox resources and fictional identities only.
- Prove authorization code plus PKCE, issuer/audience/subject mapping, state/nonce checks, native Sync session issuance, and owner-scoped RLS identity.
- Measure logout, session revocation, device revocation, account deletion, and signing-key rotation delay.
- Inspect actual plan usage before enabling any resource that could create an additional charge.

Exit: all identity and negative-authorization tests pass with no production change and no unapproved cost. Otherwise select the separate-account fallback.

**Evidence:** Complete as a disposable engineering proof. Five isolated Rust
tests cover authorization code plus S256 PKCE, redirect/state/nonce and token
claim validation, one-time codes, native session issuance, revocation,
deletion, and signing-key rotation behavior. A separate local Supabase stack
passed 31 pgTAP assertions covering owner-scoped CRUD, immutable ownership,
anonymous/cross-owner denial, active session/device enforcement, explicit
privilege boundaries, and fail-closed missing or malformed UUID claims. No hosted
resource was created or changed, and the disposable containers and volumes were
removed after the run. See `C10-02D-DISPOSABLE-IDENTITY-PROOF.md`.

### C10-02E — Independent review and evidence

- Commission an independent cryptographic/protocol review.
- Resolve every high or critical finding.
- Record test vectors, dependency versions, threat-model deltas, drills, and residual risks.

Exit: C10-02 evidence is approved before C10-03 begins.

**Current evidence:** The internal preflight was followed by an independent AI
second-opinion review of immutable commit `738b72d`. Its two High findings were
remediated in immutable source commit `2009675`. The independent 2026-10-09
re-review reproduced the archive digest, executed the focused proofs and
adversarial checks, closed both High findings, and returned **CONDITIONALLY
APPROVED FOR C10-03**. Its conditions are assigned to C10-03 through C10-07;
none authorizes production Sync or customer release. C10-03A begins with the
native identity/authorization conditions. See
`C10-02E-INDEPENDENT-REVIEW-REMEDIATION.md` and
`C10-03A-NATIVE-IDENTITY-AUTHORIZATION.md`.

## Failure behavior

- Authentication, decryption, context, version, or nonce failure stops the affected Sync operation and never applies partial remote data locally.
- Errors are stable and content-free; they never include keys, plaintext, ciphertext, project identifiers, or file paths.
- Local work continues normally through every proof or future Sync failure.
- Rollback removes only test code and fictional test state. It never touches customer project records or documents.

## Explicitly deferred

- customer-visible Sync controls;
- database migrations or encrypted outbox integration;
- production identity/provider changes;
- Supabase schema, RLS, RPCs, or retention jobs;
- legacy snapshot migration;
- telemetry; and
- enabling `MetadataSync`.

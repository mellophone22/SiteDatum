# C10-02C approved-device transfer and recovery proof

**Status:** Remediated in isolated fictional tests; independent re-review conditionally approved C10-03 on 2026-10-09

## Purpose

Prove that a surviving authorized computer can transfer a fictional workspace
key to one explicitly approved target device, and that a customer-held recovery
secret can recover the same key class after total device loss. The proof does
not enable Sync or establish a production protocol.

## Selected suite

- RFC 9180 HPKE Auth mode;
- DHKEM(X25519, HKDF-SHA-256);
- HKDF-SHA-256; and
- ChaCha20-Poly1305.

The proof uses `hpke` 0.14.1 with only `alloc`, `getrandom`, `x25519`, and
`chacha` features. It is a development-only dependency. The crate implements
RFC 9180 and zeroizes its X25519 private-key backing type, but its own published
documentation does not claim a paid independent audit. C10-02E therefore
remains mandatory.

## Approved-device protocol model

The surviving source computer encrypts the workspace key directly to the target
device public key while authenticating with its static source-device key. The
target starts with an independently retained expectation containing the source
public key and the complete enrollment context. It does not learn either value
from the relay. Canonical authenticated context binds:

- protocol version;
- authenticated owner ID;
- random workspace ID;
- source and target device IDs;
- one-time enrollment ID;
- workspace-key version; and
- expiry time.

Both computers independently derive a six-digit comparison code from the HPKE
exporter secret using a versioned comparison context. The owner must compare
the source display with the target and enter the code. A relay-only attacker
cannot calculate or grind that code from public transcript values. Each
enrollment permits one approval attempt: a wrong code consumes it and requires
a fresh enrollment. A production design must durably enforce that single-use
rule, rate limits, and expiry and must present the comparison on trusted local
screens. The six-digit code is a human confirmation signal, not an independent
cryptographic authenticator or a quantified one-in-a-million security claim.

The modeled relay sees only the bounded routing context, target public key,
encapsulated key, and ciphertext. It never receives the workspace key, device
private key, or comparison input.

## Verified cases

The integration proof in
`src-tauri/tests/sync_v2_device_transfer_proof.rs` passes:

1. surviving-device transfer recovers the same workspace key;
2. a wrong comparison code consumes the enrollment and replay is refused;
3. a substituted target key is refused before decryption;
4. modified owner/workspace/device/version/expiry context cannot authenticate,
   even when an attacker creates an internally consistent fresh envelope;
5. substituted source identity is refused using the independently retained
   source public key;
6. expired and revoked-device enrollments are refused;
7. every expected context field is checked against target-held state; and
8. a customer-held 256-bit secret recovers the workspace key locally after
   total device loss, while a wrong secret fails authentication.

No key, recovery secret, comparison code, plaintext, ciphertext, project field,
file path, or customer identifier is printed by the tests.

## Isolation and limitations

- The proof is an integration test and is not imported by the Tauri library.
- It exposes no command or UI control and reads no SQLite database or project
  file.
- It creates no account, hosted row, Supabase resource, policy, secret, or cost.
- Replay/revocation state is modeled in memory; durable server enforcement is a
  later C10-03 responsibility with negative authorization tests.
- Clock authority, device attestation, rate limiting, enrollment cancellation,
  and accessible comparison-code UX remain production design work.
- The customer-held recovery material's printable encoding and error-detection
  format remain a separately reviewed decision.

## Commands

```powershell
cargo test --test sync_v2_device_transfer_proof
cargo test
```

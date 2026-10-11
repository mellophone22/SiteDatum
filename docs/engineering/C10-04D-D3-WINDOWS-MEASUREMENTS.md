# C10-04D D3 — Windows credential-adapter measurements

Date: 2026-10-11  
Source commit: 20e73caa177ca500051bcb5ad3dfdf0e8812cf7d  
Branch: codex/c10-02e-claude-review  
Scope: test-only Windows measurements, not a historical vault, custody decision, recovery format, deployment or activation.

## Outcome

On this Windows host, the pinned keyring 3.6.3 adapter round-tripped the tested per-entry boundaries. Separate inventories of 1, 8 and 100 entries succeeded. A request for 1,000 entries wrote **478** entries before the next write returned a platform failure. The cause was not diagnosed by examining real credentials or printing raw OS errors. This is an environment-specific refusal, **not** a universal Windows maximum or a safe product capacity.

This makes an unbounded multi-credential historical vault an unsupported assumption. No new storage shape or cryptographic format is selected by this work. The existing no-removal, fail-closed key-capacity baseline remains operational. D3 is not complete; D6 and D7 remain blocked.

## Implemented harness

Source: src-tauri/tests/sync_v2_windows_storage_measurement.rs.

The Windows-only integration target uses a fixed test service and a fresh UUID account namespace for each fixture. It checks each exact target is absent before writing, tracks only targets created by the fixture, and never enumerates existing credentials. Payloads are public fixture strings and bytes, not generated keys or customer material.

Three live tests are ignored by default. Explicit invocation:

```powershell
cargo test --locked --test sync_v2_windows_storage_measurement -- --ignored --nocapture --test-threads=1
```

One ordinary test checks UTF-16 fixture-size arithmetic without accessing Credential Manager.

The inventory test records storage refusal as a measurement outcome, not as proof the requested count fits. Successfully written entries must still read back exactly. Every tracked target, including the refused-write target, is deleted and re-opened to verify absence. A Drop guard also attempts exact-target cleanup on an early failure; it never discovers or deletes untracked targets.

## Executed evidence

The first live invocation failed its original all-writes-must-succeed assertion during the 1,000-entry run. Its cleanup guard attempted removal of its exact tracked targets and reported no deletion failures; that initial version did not independently read back absence in the guard. Do not call that initial run a capacity pass or a verified absence audit.

The harness was corrected to report a refusal category/count, verify exact-target absence after cleanup, and keep functional readback/cleanup failures fatal. The explicit rerun completed: **3 passed, 0 failed**. Its inventory observation was:

| Requested entries | Written entries | Refusal | Public payload bytes | Write microseconds | Known-target read microseconds | Cleanup microseconds | Targets verified absent |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 1 | 1 | none | 192 | 2464 | 1855 | 1372 | 1 |
| 8 | 8 | none | 1536 | 18613 | 3736 | 14690 | 8 |
| 100 | 100 | none | 19200 | 928621 | 24350 | 872705 | 100 |
| 1000 | 478 | platform_failure | 91776 | 19318645 | 50468 | 18652487 | 479 |

Each entry contained 96 ASCII characters, stored by the adapter as 192 UTF-16 bytes. Write timing includes exact-target absence checks; read timing scans only the known fixture targets using fresh handles. One pass only: these are not percentile benchmarks, platform quotas, production latency guarantees, or catalog-enumeration measurements. Platform overhead is not included in the payload-byte totals.

The boundary test verified:

- ASCII passwords at 1, 256, 444, 1024 and 1280 characters read back exactly.
- 1281 ASCII characters refused as TooLong; the previous value remained unchanged.
- 640 surrogate-pair characters fit 2560 UTF-16 bytes; 641 refused, retaining the previous value.
- A 2560-byte binary fixture read back exactly; 2561 bytes refused, retaining the previous value.
- Its one exact target was verified absent after cleanup.

The missing/substituted test detected a missing fixture entry and a substituted public fixture value, with an independent fictional control unchanged. All nine tracked targets were verified absent afterward. **Known expected-value equality is not authenticated catalog verification**, and the control is not a production hot KeyRing.

## Regression verification

Executed after the harness correction:

- cargo test --locked: **227 passed, 0 failed, 8 ignored** across Rust targets.
- The three live measurement tests above were additionally invoked explicitly; the eight default ignores were not all executed.
- cargo fmt --check: pass.
- cargo check --locked --lib: pass.
- npm test: **114 passed across 35 files**.
- npm run lint: pass.
- npm run build: pass.
- git diff --check: pass.

A non-fatal incremental-cache hard-link fallback warning occurred. No visible UI changed, so no visual behavior claim is made.

## Source and platform constraints

[Microsoft CREDENTIALW documentation](https://learn.microsoft.com/en-us/windows/win32/api/wincred/ns-wincred-credentialw) specifies a 2560-byte credential blob ceiling. Password character counts alone are insufficient: keyring 3.6.3 encodes passwords as UTF-16 little-endian bytes and checks that byte budget before saving. Its binary secret method checks raw byte length. The observed oversized replacement behavior is consistent with that validation order.

The current bounded protected ring is at most 333 raw bytes / 444 base64 ASCII characters / 888 UTF-16 blob bytes. That does not imply a historical inventory of arbitrary size fits.

Inspection of the pinned adapter's Windows source also shows **CRED_PERSIST_ENTERPRISE** and a warning that concurrent writes to the same entry are not serialized in caller order. Microsoft documents enterprise persistence as potentially roaming where profiles are configured accordingly. Therefore this adapter alone is not evidence of machine-only custody, strict local persistence, serialized publication, or durable independent freshness pins. Those require a separately specified custody and ordering decision.

## Still unproven / not implemented

This harness does not provide a vault, authenticated catalog, owner root, freshness anchor, full-history transfer, canonical encoding, or destructive key-retirement API. It does not test process restart, logout/reboot, roaming, DPAPI loss, administrator reset, full-disk behavior, contention, authenticated catalog substitution, or recovery from a customer-held artifact.

D3 still requires an explicitly chosen and reviewed historical preservation construction, bounded load/resource behavior, custody semantics, and total-loss recovery drills at 1, 8, 100 and 1000 retained versions covering all twelve ciphertext columns. These entry measurements are **not** those drills. A multi-entry catalog cannot be approved from these results; an encrypted-file alternative would also require its own format/custody review.

D4 authenticated admission/fences and D5 production multi-store recovery ordering remain open. Test-only measurements and prior fictional state models do not satisfy them. No main merge, release, production migration, deployment or Sync activation follows.

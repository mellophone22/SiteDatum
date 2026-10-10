# C10-04A local checkpoint integrity

**Status:** Implemented and locally verified; Sync remains unavailable — 2026-10-10

## Purpose

This first C10-04 slice gives the Windows client a deterministic way to prove
that an encrypted workspace checkpoint describes the complete record set it
was given. It also records a local high-water anchor so a provider cannot
silently return an older checkpoint or substitute different content at an
already accepted counter.

The slice adds no Tauri command, visible control, network call, or production
service. The existing `SYNC_DEFERRED` boundary remains unchanged.

## Protocol

- A canonical SHA-256 manifest sorts records by random record UUID and binds
  each UUID, record revision, workspace-key version, and tombstone state.
- Duplicate record IDs and zero revisions/key versions fail before encryption.
- XChaCha20-Poly1305 encrypts the checkpoint under the local workspace key.
  Authenticated associated data binds the owner, workspace, protocol version,
  workspace-key version, checkpoint counter, and through-change cursor.
- Opening a checkpoint requires the complete supplied manifest to reproduce
  the authenticated digest. Omission, header substitution, ciphertext
  alteration, or use under another workspace fails closed.
- The local-only `sync_v2_checkpoint_anchors` table stores the highest accepted
  counter, digest, and cursor. It contains no project content, file path,
  account token, or key material and is not part of the replicated record set.
- Lower counters, equal counters with a different digest/cursor, and higher
  counters that do not advance the cursor are rejected without changing the
  durable anchor. An exact equal checkpoint is an idempotent success.

## Migration safety

Migration 11 creates only the local anchor table. It uses the existing
forward-only transactional migration runner, so an existing schema-10
workspace receives the normal verified pre-migration backup before the table
is added. The migration does not read, rewrite, move, or delete project rows or
ordinary Windows files.

## Verification

Focused Rust tests cover:

1. order-independent manifest hashing and duplicate rejection;
2. authenticated round-trip plus omitted-record, workspace-substitution, and
   ciphertext-tamper refusal; and
3. close/reopen durability, exact retry, lower-counter rollback refusal,
   equal-counter substitution refusal, and valid monotonic advancement.

The full Rust suite also exercises the real migration runner, representative
schema-9 upgrade/backup preservation, and newer-schema refusal.

## Remaining C10-04 work

C10-04A is a prerequisite, not a releasable Sync feature. Later slices must
still add versioned encrypted record codecs, an atomic outbox, durable pull
cursor/device acknowledgements, pre-apply validation and safety backup,
record-scoped conflict preservation, deterministic two-computer scenarios,
and the authenticated recovery-anchor format. Tombstone compaction remains
blocked until those acknowledgement and replacement-checkpoint tests pass.

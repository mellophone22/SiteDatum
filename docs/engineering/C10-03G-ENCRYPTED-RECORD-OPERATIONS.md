# C10-03G encrypted hosted record operations

**Status:** Disposable local schema, trusted bridge, signed batch protocol, and integration proof verified; Sync and production deployment remain unauthorized — 2026-10-09

## Purpose

Prove the smallest hosted record boundary needed by Sync v2 without giving the
service plaintext project data. The service stores random identifiers,
constrained routing/version metadata, bounded opaque ciphertext, server
timestamps, and an encrypted workspace checkpoint. It never receives project
names, record titles, task/RFI/submittal/note/contact fields, file paths,
documents, database contents, workspace keys, or device private keys.

This slice adds no Tauri command or UI, changes no production service, and does
not enable Sync. The central `SYNC_DEFERRED` denial remains unchanged.

## Atomic record protocol

1. An already enrolled live device creates a random opaque workspace
   identifier through the authenticated service bridge.
2. The desktop will locally encrypt each versioned record and a workspace
   checkpoint. C10-03G accepts only the resulting opaque envelopes; local
   codecs and key use remain owned by C10-04.
3. The device signs a canonical versioned message binding the owner and Auth
   session, device, workspace, batch, checkpoint counter/version/ciphertext,
   and every ordered mutation including its expected server version,
   mutation identifier, tombstone flag, and ciphertext.
4. The Edge Function verifies the Ed25519 signature with the server-owned
   active-device public key before calling the service-only database wrapper.
5. One database transaction validates every mutation before it writes any
   envelope. It then writes the current envelopes, immutable change rows,
   encrypted checkpoint, idempotency receipt, and workspace cursor together.
6. Pull returns cursor-ordered opaque changes and only a checkpoint whose
   through-cursor is covered by that page. Ciphertext is never interpreted by
   the service.

## Failure and retry behavior

- An exact accepted batch retry returns `already_applied`; an altered reuse of
  its batch identifier is rejected.
- Duplicate record or mutation identifiers, reused mutation identifiers,
  stale expected versions, skipped checkpoint counters, invalid bounds,
  wrong sessions/devices/owners, and disabled workspaces fail closed.
- Validation precedes every write, so one stale record rejects the entire
  batch without advancing the checkpoint or cursor.
- Tombstones are opaque versioned records with a server deletion timestamp;
  retention and permanent hosted deletion remain C10-03H work.
- Requests and responses are bounded, rate-limited, authenticated, and return
  content-free errors. HTTPS is mandatory outside the disposable loopback
  proof.

## Privilege boundary

- Workspace, envelope, change, checkpoint, idempotency, and rate-window tables
  are private, force RLS, and grant no direct access to `anon`,
  `authenticated`, or `service_role`.
- Public-schema bridge functions are executable only by `service_role`; every
  security-definer function fixes an empty search path and revalidates the
  exact live Auth session and device where required.
- The desktop receives only the public Supabase identifier and its short-lived
  user token. No service-role key, secret API key, private key, or webhook
  secret enters the desktop module or repository.

## Verification evidence

- Four protocol tests cover exact request shapes, duplicate rejection, a
  stable canonical SHA-256 vector, and Ed25519 change detection.
- The C10-03G pgTAP suite passes 41 assertions covering forced RLS, grants,
  live-session/device enforcement, opaque workspace creation, atomic batch
  commit, exact retry, stale/reused mutation rejection, tombstones, paging,
  checkpoint withholding, and cross-owner refusal.
- The complete disposable database suite passes 242 assertions across
  C10-03A through C10-03G, and database lint reports no schema errors.
- A disposable-user integration drill passes initial device enrollment,
  workspace creation, independently signed two-record push, exact retry,
  whole-batch stale-version rejection, byte-exact ciphertext/checkpoint pull,
  and unauthenticated refusal.

## Remaining gates

C10-03 still requires the retention/deletion workflow, clean reconstruction
and security-advisor evidence, and production-independent client/repository
secret proof before it can close. C10-04 owns local codecs, encrypted outbox,
durable cursor/checkpoint state, conflict handling, and atomic local apply.
C10-05 owns explicit consent, device management, deletion, and recovery UX.
No customer Sync beta is authorized until the later C10 gates pass.

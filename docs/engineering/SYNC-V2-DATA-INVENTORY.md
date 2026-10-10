# Sync v2 data inventory

**Status:** C10-01 design baseline — 2026-10-08

This inventory is the allowlist for Sync v2 design. It does not enable transmission. Any new local field is excluded by default until this document, the consent copy, encryption tests, and privacy review are updated.

## Classification rules

- **Encrypted content:** may be synchronized only inside an authenticated-encryption envelope created on the Windows computer.
- **Encrypted relationship:** stable IDs and relationship data are also content; they are not exposed to the service.
- **Server-visible routing:** the minimum fields the storage service must read to authorize, order, deduplicate, retain, and delete ciphertext.
- **Local only:** never enters a Sync v2 request or hosted record.
- **Document bytes:** never handled by Sync v2.

## Encrypted local metadata

| Entity | Fields inside the encrypted envelope | Reason/sensitivity |
| --- | --- | --- |
| Project | `id`, number, name, status, phase/custom phase, customer, GC, engineer, PM, superintendent, location, dates, description, important notes, portable project-root reference, pin/archive state, timestamps | Identifies customers, sites, personnel, schedules, and project context |
| Task | `id`, project ID, title, description, priority, status, category, dates, waiting state, related contact ID, timestamps | Commitments, blockers, schedule, and people |
| RFI | `id`, project ID, number, subject, question, recipient, status, dates, response, notes, location, drawing number, cost/time impact, suggested solution, requested by, timestamps | Technical, contractual, schedule, and potentially commercial content |
| Submittal | `id`, project/parent IDs, number, name, package, revision, recipient, status/disposition, dates, resubmission flag, notes, timestamps | Technical review and delivery history |
| RFI/submittal task relationship | Both record IDs and relationship timestamp | Reveals workflow structure |
| RFI/submittal attachment reference | `id`, parent ID, portable path token, display name, timestamp | Reveals document names and local organization; no file bytes |
| Registered file | `id`, project ID, file name, portable path token, operation, discipline, drawing metadata, current flag, timestamps | Reveals document identity and organization; no file bytes |
| Note | `id`, optional project ID, body, timestamps | Free-form and potentially highly sensitive |
| Contact | `id`, name, company, email, phone, role, timestamps | Personal data |
| Activity event | `id`, event/entity types and ID, summary, timestamp | May summarize sensitive record activity |
| Work item | All constrained and optional fields, project ID, dates, parties, company, location, amount, checklist counts, archive state, timestamps | Operational, personal, location, and commercial data |
| Project template | `id`, name, task/milestone title arrays, timestamps | Internal process and workflow data |

Every row is encrypted independently with authenticated additional data binding the protocol version, owner, workspace, record kind, record ID, workspace-key version, and expected record revision. The server-visible envelope must not include a searchable copy of any content field.

## Server-visible Sync envelope

| Field | Purpose | Constraint |
| --- | --- | --- |
| `owner_id` | RLS ownership | Supplied from authenticated subject, never trusted from client JSON |
| `workspace_id` | Groups one user's synchronized workspace | Random opaque identifier; not a project/customer name |
| `record_id` | Addresses one encrypted record | Random stable identifier |
| `record_kind` | Enables bounded validation/dependency ordering | Constrained code only; no user-provided label |
| `server_version` | Optimistic concurrency | Server-controlled monotonic integer |
| `client_mutation_id` | Idempotent retry | Random unique value |
| `protocol_version` | Compatibility gate | Constrained supported integer |
| `workspace_key_version` | Selects the locally held decrypting key | Constrained positive integer and authenticated as envelope AAD |
| `ciphertext` | Encrypted record payload | Size-limited authenticated ciphertext |
| `is_tombstone` | Propagates deletion | Boolean; tombstone body contains no plaintext |
| `created_at`, `updated_at`, `deleted_at` | Retention and pull cursor ordering | Server-generated timestamps |

No provider-facing column may contain a project number, record title, contact detail, file name/path, document text, local database path, encryption key, or licensing token.

An encrypted workspace-checkpoint envelope authenticates a monotonic client
counter and the digest of the complete sorted record/version/key-version/
tombstone set. The provider may route it by opaque workspace ID and counter but
cannot read or forge its contents. Each device keeps the highest accepted
counter and digest locally; device-transfer and customer-held recovery material
carry the minimum acceptable checkpoint anchor so a replayed partial set fails
closed.

## Local-only data

- `app_settings`, appearance, window/navigation state, project-root absolute path, backup destinations, notification choices, and device-local reminders.
- `schema_migrations`, local outbox processing state, pull cursors, content hashes, local retry/error detail, and unresolved plaintext conflict candidates.
- licensing credentials, subscription state, complimentary-grant state, billing identifiers, and Stripe data.
- Supabase/service refresh tokens and device private keys, stored only through Windows-protected credential facilities.
- logs containing local technical paths or raw provider responses; Sync logs use stable error codes and content-free identifiers instead.
- local backup files, recovery archives, CSV exports, generated PDFs, and installer/update artifacts.

## Document and filesystem boundary

Sync v2 never uploads, downloads, opens, transforms, indexes, or deletes project-document bytes. Portable path tokens may be encrypted as record content so another computer can rebase a reference under its own selected project root. A path outside that root is reduced locally to an encrypted external filename reference and must appear as missing until the user locates it. Sync never guesses a destination or moves a file.

## Prohibited data flows

- Project content to licensing, billing, telemetry, email, or support services.
- Sync ciphertext or keys to Stripe or the licensing database.
- Service-role credentials or encryption keys to the desktop bundle or repository.
- Plaintext record data in URLs, logs, analytics, crash reports, audit rows, or deletion receipts.
- Broad `SELECT *` serialization of newly added tables or columns. Each record codec is an explicit versioned allowlist.

## Change control

C10-04B's schema-1 content codecs implement this inventory as an explicit field
allowlist. Local absolute paths are replaced by typed portable references;
template title JSON is represented as bounded text arrays. These conversions
add no data category. Hosted kind codes and the encrypted work-item/template
subtype are recorded in `C10-04B-ENCRYPTED-LOCAL-QUEUE.md`. Local stream state,
encrypted staging snapshots/outbox, and staging acknowledgements are excluded
from replicated content and CSV portability. Later adapters must validate
dependency relationships, local ID mapping, and safe filesystem rebasing before
any live project application.

C10-04C implements this inventory without new content categories. Schema-14
committed baselines, encrypted batches/history, retained conflict choices,
backup-bound applied receipts and subtype/activity mappings are local-only
infrastructure, excluded from replicated content and CSV export. External
references use an unresolved local marker; original filename tokens remain
encrypted content only. Receipt output exposes opaque IDs/counters/digest, never
a backup path. See `C10-04C-COMPLETION.md`.

Adding an entity or field requires all of the following in the same work package:

1. update this inventory and the user-facing consent categories;
2. add an explicit versioned codec and round-trip tests;
3. add ciphertext/plaintext-leakage tests;
4. review retention and deletion behavior;
5. test old/new client compatibility; and
6. obtain a new independent security review if the field changes the threat surface.

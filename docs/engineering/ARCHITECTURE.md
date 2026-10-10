# Architecture

Preferred baseline: Tauri desktop shell + React + TypeScript UI + SQLite. Validate current stable versions and Windows packaging first.

Layers: Presentation -> Application/use-cases -> Domain/rules -> Persistence/SQLite -> Filesystem service -> Platform integrations. React components must not directly own database/filesystem logic.

Principles: typed boundaries, migrations, foreign keys, transactions, structured errors, deterministic Attention rules, minimal dependencies, no network requirement. Normalize/validate paths and never execute project files.

## Dormant encrypted Sync application boundary

C10-04C supplies explicit native record adapters and atomic live/outbox hooks.
Encrypted pages are assembled durably, authenticated as a complete checkpoint
set and rehearsed in a private SQLite copy. A verified safety backup precedes
atomic application/cursor/anchor advancement. Conflicts retain encrypted
candidates and require explicit retry-safe choices; applied receipts differ
from staging evidence. File operations are metadata-only and external references
never guess destinations. No Tauri command, UI or transport exposes these APIs.
Legacy Sync stays denied by `SYNC_DEFERRED`; consent, recovery, rotation and
hosted gates remain mandatory. See `C10-04C-COMPLETION.md`.

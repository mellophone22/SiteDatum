# WP8 — Hardening

WP8 adds an explicit app-local SQLite backup action in Settings. Backups are written beneath the application-local data directory after a WAL checkpoint; project documents are never copied, moved, or modified by this action. Failures return structured recovery guidance for storage, checkpoint, and copy errors.

Final review confirms visible actions use typed Rust/Tauri commands, controls expose text labels, Ctrl+K/Escape keyboard behavior exists for search, Ctrl+N starts the current screen's supported create workflow, dense list interfaces avoid decorative dashboard patterns, and missing-file states retain recoverable metadata with an explicit Locate recovery action. Search results carry exact entity identifiers into the destination screen, including same-screen navigation. Filters and the last workspace screen are retained locally.

Drawing records now support revision dates, received dates, and explicit current/superseded state. Notes and contacts support create, edit, and explicit confirmed deletion. Windows packaging produces MSI and NSIS installers; release signing remains a distribution concern because no signing certificate is configured.

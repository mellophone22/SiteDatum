# WP5 — Drawings and Files

WP5 introduces a local registered-file/drawing register. The user explicitly chooses Register, Copy, or Move. Register preserves the selected external path; Copy and Move target the selected project’s drawing folder and reject destination collisions before any file bytes are changed.

The Rust boundary owns validation, filesystem mutation, persistence, and missing-file checks. Missing files retain their metadata record with recovery guidance. Removing a reference deletes metadata only; it never deletes the physical file. The UI exposes drawing number, title, revision, discipline, project, and explicit operation.

## Verification

Rust tests cover Register, Copy, Move, source preservation, destination collision prevention, and existing domain/persistence behavior. Frontend build/typecheck and lint validate the Files register screen.

# Filesystem & Error Handling

Default structure:
Projects/{Number} - {Name}/
- 01 Drawings/{Controls,Mechanical,Electrical,Architectural}
- 02 Submittals/{Created,Submitted,Approved}
- 03 RFIs/{Open,Closed}
- 04 Specifications
- 05 Sequences
- 06 Field/{Photos,Startup,Commissioning}
- 07 Closeout
- 99 Archive

Root is configurable. Preview path before creation. Sanitize Windows-invalid characters. Copy/Move/Register are distinct explicit operations. Never overwrite silently. Missing file is recoverable, not auto-deleted.

Native Windows dialogs and `std::fs::canonicalize` may return verbatim paths such as `\\?\C:\Projects` or `\\?\UNC\server\share`. The frontend removes that transport-only prefix for immediate display. Rust independently normalizes it before validation and again before persistence, storing ordinary `C:\...` or `\\server\share` syntax. Device namespace paths remain rejected. The normalized canonical path must pass the same validation again so Browse -> Check -> Save cannot reject its own output.

Handle: DB/migration failure, unavailable root, permission denial, locked/missing files, collisions, partial folder creation, DB/filesystem partial success, external rename/move, backup failure. Never report success until the whole user-visible operation is valid. Log technical detail locally; show concise recovery guidance.

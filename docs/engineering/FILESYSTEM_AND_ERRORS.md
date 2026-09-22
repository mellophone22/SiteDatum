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

Handle: DB/migration failure, unavailable root, permission denial, locked/missing files, collisions, partial folder creation, DB/filesystem partial success, external rename/move, backup failure. Never report success until the whole user-visible operation is valid. Log technical detail locally; show concise recovery guidance.

# ADR-003: RFI PDF export from the supplied Excel template

## Status

Accepted.

## Decision

The supplied `Blank RFI.xls` remains the editable design source in `assets/templates`. A reviewed, one-page PDF rendering of that workbook is embedded in the Rust binary as the immutable page background. Rust uses `lopdf` to place saved RFI and project metadata onto that background and writes the completed PDF to the user-approved destination.

The frontend never edits workbook or PDF bytes. It saves the RFI through the existing typed command boundary, requests a default project-folder filename, obtains an explicit destination through the native Save dialog, and invokes a typed Rust export command.

The exporter:

- does not require Excel or LibreOffice at runtime;
- never overwrites an existing file;
- writes through a temporary file in the destination folder, then renames it;
- reports unavailable folders, permission failures, invalid paths, collisions, and content that cannot fit the one-page template through the structured error contract;
- registers the generated PDF as an RFI attachment reference after a successful write;
- treats the PDF as a normal project document while SQLite remains the metadata source of truth.

## Consequences

Template layout changes require updating the source workbook, regenerating and visually approving the embedded PDF background, and rechecking field coordinates. This deliberate build-time step provides consistent output on workstations that do not have Microsoft Excel installed.

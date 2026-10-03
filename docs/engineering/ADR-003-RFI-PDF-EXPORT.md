# ADR-003: Local RFI PDF generation and customer-owned templates

## Status

Accepted. Revised 2026-10-03.

## Decision

SiteDatum must not bundle a founder, employer, or customer-specific RFI form. The default RFI PDF is generated locally from SiteDatum-owned layout primitives and optional locally stored organization branding: company name, contact line, and accent color.

Pro customers may instead reference their own one-page portrait US Letter PDF. SiteDatum validates the file, retains only its local path, and overlays saved RFI and project metadata at the documented field positions. The customer PDF is not copied, modified, uploaded, or placed in the application database. If it becomes unavailable, export stops with recovery guidance.

The frontend never edits workbook or PDF bytes. It saves the RFI through the existing typed command boundary, requests a default project-folder filename, obtains an explicit destination through the native Save dialog, and invokes a typed Rust export command.

The exporter:

- does not require Excel, LibreOffice, or a bundled company form at runtime;
- never overwrites an existing file;
- writes through a temporary file in the destination folder, then renames it;
- reports unavailable folders, permission failures, invalid paths, collisions, and content that cannot fit the one-page template through the structured error contract;
- registers the generated PDF as an RFI attachment reference after a successful write;
- treats the PDF as a normal project document while SQLite remains the metadata source of truth.

Custom templates and generated professional RFI PDFs remain subject to the centralized `ProfessionalReports` entitlement check. Downgrade never removes the saved template reference or any existing PDF.

## Consequences

The SiteDatum layout can change only with visual review and coordinate regression tests. Customer templates must follow the published field-position contract. Supporting arbitrary field mapping or multi-page customer templates would require a separate, versioned template editor and is outside this decision.

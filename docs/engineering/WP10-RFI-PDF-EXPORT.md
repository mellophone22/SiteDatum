# WP10 - RFI PDF export

## Delivered behavior

- The New RFI and RFI detail forms include RFI location, drawing number, cost impact, time delay, suggested solution, and requested-by fields used by the SiteDatum PDF layout and compatible customer templates.
- Project name and project location come from the selected project. RFI number, date, recipient, subject, and question come from the RFI record.
- **Save and create PDF** saves the RFI, opens a native Save dialog defaulted to the project's `03 RFIs/Open` or `03 RFIs/Closed` folder, creates the one-page PDF, and registers it as an attachment reference.
- **Save only** remains available when the user is not ready to create a document.
- Existing files are never overwritten. Cancelling the Save dialog preserves the saved RFI without creating a file.
- Settings provides a neutral SiteDatum layout with local company name, contact details, and accent color.
- Pro customers may reference their own compatible PDF. SiteDatum validates but never copies, modifies, uploads, or deletes the source file.
- No founder, employer, or customer-specific PDF or editable source template is included in the repository or application binary.

## Verification

Rust coverage validates generated-layout output, company branding, custom-template validation and immutability, persisted settings, field overlay text, one-page output, and collision refusal. The generated sample is rendered to PNG and visually checked for alignment, wrapping, clipping, and legibility. TypeScript build, ESLint, frontend tests, native Tauri startup, and the production installer build form the remaining quality gate.

# WP10 - RFI PDF export

## Delivered behavior

- The New RFI and RFI detail forms include RFI location, drawing number, cost impact, time delay, suggested solution, and requested-by fields used by the supplied Brooks Building Solutions template.
- Project name and project location come from the selected project. RFI number, date, recipient, subject, and question come from the RFI record.
- **Save and create PDF** saves the RFI, opens a native Save dialog defaulted to the project's `03 RFIs/Open` or `03 RFIs/Closed` folder, creates the one-page PDF, and registers it as an attachment reference.
- **Save only** remains available when the user is not ready to create a document.
- Existing files are never overwritten. Cancelling the Save dialog preserves the saved RFI without creating a file.
- The original Excel workbook is retained as the template source; exported PDFs do not require Excel at runtime.

## Verification

Rust coverage validates template loading, field overlay text, one-page output, persisted template fields, and collision refusal. The generated sample is rendered to PNG and visually checked against the original template for field alignment, wrapping, clipping, and legibility. TypeScript build, ESLint, frontend tests, native Tauri startup, and the production installer build form the remaining quality gate.

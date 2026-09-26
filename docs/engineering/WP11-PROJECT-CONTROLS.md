# WP-11 — Project Controls Reorganization

The operational workspace is now consistently presented as Project Controls. Its ten existing registers are grouped by purpose without changing their stored type identifiers or backend behavior.

## Register hierarchy

- Coordination: Meeting Minutes and Transmittals
- Commercial: Procurement and Change Events
- Schedule / Field: Milestones, Daily Reports, and Punch List
- Startup & Commissioning: Startup Checks, Commissioning Checks, and Commissioning Issues

The selected register remains textually and visually explicit. Group headings use structure and separators rather than decorative cards, and the hierarchy collapses from four columns to two and then one at narrower desktop widths.

Calendar, Project Templates, Reports, create/edit forms, search, status filters, bulk status updates, import preview, CSV export, Excel export, and report generation are preserved. A pure hierarchy module and tests ensure all ten register types remain reachable exactly once.

No schema, migration, Rust command, import/export format, or report-template change was required.

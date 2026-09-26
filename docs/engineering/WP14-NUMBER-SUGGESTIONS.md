# WP-14 — Smart Defaults and Number Suggestions

New RFIs and Submittals now receive conservative project-scoped number suggestions after a project is known. Suggestions are visible, explicitly labeled as editable, and never prevent manual numbering.

## Rules

- RFIs use the canonical `RFI-001` pattern.
- Submittals use the canonical `SUB-001` pattern.
- Existing canonical values determine the next numeric value and preserve padding wider than three digits.
- Matching is case-insensitive.
- Noncanonical customer formats are ignored rather than interpreted or copied.
- All Projects creation waits until the user chooses a project before suggesting a number.
- Creating a revision preserves its parent Submittal number instead of generating a new one.

The pure suggestion helper and tests cover initial values, sequence advancement, case-insensitive matching, padding, and noncanonical formats. Existing SQLite constraints remain authoritative: RFI numbers are unique per project, and Submittal number/revision combinations are unique per project. A stale suggestion therefore fails safely rather than overwriting or merging a record.

No schema, migration, or numbering-convention configuration was added.

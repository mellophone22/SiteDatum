# WP-06 — Shared Detail Panel

## Pattern

`DetailPanel` is the shared non-modal side-panel foundation for dense registers. It keeps the register mounted and full width while closed, then participates in the register grid while open. The panel owns its accessible heading, close control, Escape behavior, focus entry, and focus restoration.

Opening controls must:

- retain a reference to the button that opened the panel;
- expose `aria-expanded` and `aria-controls`;
- expose row selection separately when a record is selected.

The panel receives record-specific content as children. It does not own persistence, validation, or lifecycle semantics.

## Representative migration

RFIs are the first migrated register. Existing search and project/status filters remain mounted while creating or editing. Native button keyboard behavior opens a row, Escape closes the panel, and close returns focus to the originating row button or New RFI button.

At widths below 980px, the existing register grid stacks the panel below the table. The one-column form breakpoint below 720px remains in effect, which also covers enlarged desktop zoom behavior.

## Verification

- Closed register retains its full-width layout.
- New RFI opens the labeled panel and moves focus to it without scrolling away from the panel heading.
- Escape closes the panel and restores focus to New RFI.
- Disabled save actions remain visibly and semantically disabled for an incomplete form.
- No Vite error overlay or browser console errors were present.
- The browser-only preview cannot load Tauri-backed project records; populated-row selection will receive full native verification during the later end-to-end hardening package.

# ADR-017 - Windows, PDF, and filesystem correctness validation

**Status:** Accepted and Windows-verified  
**Date:** 2026-10-08

## Context

The remediation audit was static and could not fully establish native Windows rename, canonicalization, PDF rendering, or filesystem-preservation behavior. SiteDatum's local-first boundary depends on those behaviors: ordinary project files must remain normal Windows files, every mutating operation must be explicit, collisions must not overwrite existing content, and RFI PDF output must be readable and finalized atomically.

## Decision

Retain the existing Rust-owned filesystem and PDF boundaries. Validate them with focused native Windows tests plus rendered-output inspection rather than adding a second implementation path or a browser substitute.

The validation covers:

- project-root recognition, verbatim drive/UNC normalization, unavailable locations, invalid paths, and permission errors;
- creation of the documented project tree and refusal to reuse an existing project folder without changing its contents;
- explicit Register, Copy, and Move behavior in disposable directories;
- preservation of source and destination bytes when a copy collision is refused;
- refusal of a missing source without creating a destination;
- a locally generated, unencrypted, one-page portrait US Letter RFI PDF;
- readable representative RFI text and visual layout after pixel rendering;
- refusal to overwrite an existing PDF without changing its bytes;
- refusal of relative or unavailable PDF destinations without leaving output artifacts; and
- customer-owned template output without modifying the source template.

## Evidence

The focused test run executed on Windows outside the restricted Codex temp sandbox. The sandboxed attempt was not accepted as product evidence because Windows denied rename/move operations inside the redirected package temp directory; the identical tests passed when run against ordinary disposable Windows paths.

The representative PDF reported one page, no encryption, and a 612 x 792 point media box. A 1530 x 1980 pixel render showed no clipped text, overlap, broken glyphs, missing labels, or content outside the page.

The user's existing installed SiteDatum workspace was not used for automated filesystem mutation. Native application startup and the Help/package interaction review were separately confirmed on the same Windows workstation before this validation package.

## Boundaries and accepted limitations

- The focused tests do not prove behavior on every removable drive, network share, filesystem, printer, or third-party PDF viewer.
- `Move` currently relies on the operating system rename operation. A cross-volume move can be refused with recovery guidance; SiteDatum does not emulate it with an implicit copy-and-delete sequence.
- `Show in Explorer` remains the supported file-opening boundary. SiteDatum does not execute registered documents.
- No project content, customer data, hosted service, billing resource, or production setting is involved.
- Release-candidate install/upgrade and broad Windows-version coverage remain release-assurance responsibilities rather than PDF/filesystem unit-test concerns.

## Consequences

The package strengthens regression proof without changing persistence, file-operation semantics, licensing, or PDF layout. Any future cross-volume move workflow must be explicit, preserve collision guarantees, handle partial copy/delete outcomes, and receive its own architecture decision and Windows validation.

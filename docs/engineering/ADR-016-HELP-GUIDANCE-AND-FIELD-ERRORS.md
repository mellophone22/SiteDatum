# ADR-016 — Help, guidance, and field-level errors

**Status:** Accepted and Windows-verified  
**Date:** 2026-10-08

## Context

SiteDatum already returned structured boundary errors, but product guidance was distributed across individual screens. New users had no permanent in-app explanation of the record model, local-file behavior, backup boundaries, or Free/Pro boundary. Several creation forms also disabled their primary action while required values were blank, leaving keyboard and assistive-technology users without a field-specific explanation.

## Decision

SiteDatum provides a permanent **Help** destination in the System navigation and command palette. The guide is task-oriented rather than a feature catalogue: establish a project context, choose the right register, understand explicit file operations, protect the database and normal Windows files, and understand the narrow commercial boundary. Its navigation buttons open existing working screens; it introduces no placeholder controls or network requirement.

Required-field feedback uses a shared, deterministic client helper. After an attempted submission, each incomplete field receives concise text, `aria-invalid`, and an `aria-describedby` relationship. Focus moves to the first incomplete field. The first-run workspace flow and project create/edit flow adopt the pattern first because they gate all later work. Server-side validation remains authoritative.

Empty states distinguish an empty register from a filtered-out register and provide a working next action where one is appropriate. Projects can create the first workspace or clear filters; Files can open the existing explicit Register/Copy/Move workflow.

## Boundaries

- The guide is bundled with the desktop app and works offline.
- Help does not send project data, diagnostics, or telemetry.
- Support text tells users to retain a correlation reference without asking them to transmit project documents or database contents.
- Billing, licensing, sync, file mutation, and database behavior are unchanged.
- Native and Rust validation remain in force; client feedback is guidance, not a security boundary.

## Verification

- `npm run lint`
- `npm test -- --run`
- `npm run build`
- `cargo build --locked --manifest-path src-tauri/Cargo.toml --target-dir src-tauri/target-codex-preview`
- `git diff --check`
- Windows preview review confirmed the Help route, functional navigation, and required-field error behavior on 2026-10-08


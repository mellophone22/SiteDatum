# Product

<!-- impeccable:product-schema 1 -->

## Platform

web

## Users

The primary user is a Project Engineer managing multiple active construction and controls projects from Windows workstations. The daily job is to preserve project context, record commitments quickly, find normal Windows documents, and act on overdue, blocked, waiting, and due-soon work without relying on memory.

The current product remains a single-user workspace. This is inferred from the approved architecture and MVP specification; expanding to internal user administration or multi-user assignment remains an open product decision.

## Product Purpose

AnyDesk is a local-first Windows command center for projects, tasks, RFIs, submittals, documents, notes, contacts, and operational registers. Success means the user can trust it every workday to surface commitments, preserve relationships, and produce useful project records without taking ownership of the underlying Windows files.

## Positioning

AnyDesk combines structured project metadata with ordinary Windows project folders. SQLite owns application records and relationships, while project documents remain normal files that can still be opened, copied, backed up, and synchronized with familiar Windows tools.

## Operating Context

The core loop is: open Attention, select a project or item, perform or record the work, link its artifacts, mark the resulting state, and continue. Work frequently moves between office and home Windows workstations through optional Supabase metadata synchronization and a dedicated OneDrive project root.

## Capabilities and Constraints

- React communicates through typed application-level Tauri commands; Rust owns SQLite and filesystem operations.
- The product must remain useful offline. Optional cloud sync cannot become a launch dependency.
- No AI, LLM, embedding, vector database, AI key, or AI placeholder functionality is permitted.
- Filesystem actions must be explicit, collision-safe, and recoverable. Ordinary metadata deletion must not delete physical files.
- Statuses and phases use constrained machine-readable values without becoming rigid workflow engines.
- The approved expansion includes saved views, quick capture, project overview, bulk actions, project templates, local reminders, calendar/timeline, CSV/Excel import and export, document templates, audit/recovery, meeting minutes, procurement, change events, punch lists, daily reports, startup/commissioning checklists, commissioning issues, transmittals, and project milestones.
- Local Windows notifications are the inferred reminder boundary for this expansion. External email/calendar integration remains undecided.
- Work is delivered as dependency-ordered, independently verified work packages. This delivery choice is inferred because the structured interview was unavailable.

## Brand Commitments

The product name is AnyDesk. Existing supplied AnyDesk logo and Windows application icon assets must remain recognizable and cohesively integrated. The voice is concise, operational, and professional.

## Evidence on Hand

The repository contains implemented Windows workflows, migrations, automated tests, approved architecture decisions, a supplied RFI Excel template, generated RFI PDF assets, and release builds. It contains no testimonials, commercial claims, or sample customer data; future work must not fabricate them.

## Product Principles

1. Nothing falls through the cracks.
2. Daily work should require fewer clicks and less repeated entry.
3. Project context and relationships should remain visible while the user acts.
4. Normal Windows files remain user-owned and recoverable.
5. Every visible control performs real work and every failure explains recovery.

## Accessibility & Inclusion

Target WCAG 2.2 AA principles where applicable: complete keyboard access for core workflows, visible focus, semantic labels, sufficient contrast, non-color state cues, reduced-motion support, logical focus order, and associated validation feedback.

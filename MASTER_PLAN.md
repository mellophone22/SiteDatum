# Master Plan

## Vision
Create a daily command center for a Project Engineer juggling many jobs. Preserve project context and surface overdue, due-soon, blocked, and waiting-on-others work.

## Pillars
1. Nothing falls through the cracks.
2. Projects are connected workspaces, not isolated forms.
3. Files remain normal Windows files; the DB stores metadata/relationships.
4. Daily operation is fast: quick-add, search, keyboard actions, remembered state.
5. Designed, not generated: restrained, professional, original UI.

## Daily loop
Open -> review Attention -> select item/project -> perform/record work -> link artifacts -> complete/block/wait -> continue.

## Core domains
Attention, Projects, Tasks, RFIs, Submittals, Drawings/Files, Notes, Contacts, Activity, Search/Command Palette, Settings.

## Success
The user trusts the app enough to rely on it every workday to remember commitments.

## Commercial direction
SiteDatum has a deliberately narrow Free/Pro commercial perimeter without changing its local-first operating model. Free remains accountless and supports up to three active projects with complete manual register workflows and essential data portability. Pro is priced at $15 monthly or $150 annually and adds unlimited projects, workflow acceleration, and professional outputs. Paid identity, hosted billing, and entitlement recovery remain separate from project data; expiration preserves access and editing and never deletes customer work. Pro uses a 21-day connectivity grace period and supports two active Windows computers. The initial paid channel is a deliberately disclosed unsigned Windows Early Access release while trusted code signing remains deferred. Optional metadata sync is deferred from the initial commercial launch and may return later as a Pro capability after redesign and security validation. Architecture and launch gates are recorded in `docs/engineering/ADR-006-COMMERCIAL-LICENSING-BOUNDARY.md`, `docs/engineering/ADR-011-CONTROLLED-UNSIGNED-EARLY-ACCESS.md`, and `docs/engineering/C9-PRODUCTION-COMMERCE-PROMOTION.md`.

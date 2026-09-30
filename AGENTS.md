# Codex Operating Instructions

Build a local-first Windows Project Engineer workspace. Priorities: (1) seamless/reliable functionality, (2) exceptional deliberate UI/UX.

Read `MASTER_PLAN.md`, then every file under `docs/`, then `.codex/skills/project-ui-design/SKILL.md` before coding.

## Hard rules
- No AI/LLMs, embeddings, vector DB, or AI placeholders.
- SiteDatum remains a single-user, local-first Windows product. Do not add collaboration, organizations, RBAC, seat billing, or a cloud requirement for ordinary project work.
- A narrowly scoped commercial boundary is permitted: minimal customer identity for paid entitlement recovery, merchant-of-record hosted checkout/customer portal, a trusted licensing service, signed updates, and privacy-safe operational telemetry only when separately approved.
- Free local use must not require an account. Project records and documents remain local, and billing/licensing/telemetry systems must never receive project names, tasks, RFIs, submittals, notes, contacts, file paths, documents, or database contents.
- Never ship secret keys, webhook secrets, signing private keys, service-role credentials, or billing-provider API keys in the desktop client or repository. Public client identifiers are permitted only when documented and protected by server-side authorization/RLS as applicable.
- Subscription enforcement must be centralized, testable, and non-destructive. Expiration never deletes, moves, hides, or makes existing customer records uneditable; backup and essential data export remain available.
- Real project documents remain normal Windows files.
- No fake UI: every visible control works.
- Never silently overwrite/delete/move files.
- Prefer mature minimal dependencies.
- Do not build the entire app in one uncontrolled pass.

## UI rules
No generic vibe-coded dashboard: avoid giant cards, gradients, glassmorphism, huge headings, excessive whitespace, emoji icons, decorative charts, and pill-heavy UI. Prefer information-dense tables/lists, typography, alignment, separators, restrained surfaces, keyboard support, and context-preserving interactions.

## Workflow
For each work package: read specs -> plan -> implement a small vertical slice -> test -> lint/typecheck -> visually review -> fix -> document.

Start by validating Tauri + React + TypeScript + SQLite for Windows. Document any meaningful architecture change.

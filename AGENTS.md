# Codex Operating Instructions

Build a local-first Windows Project Engineer workspace. Priorities: (1) seamless/reliable functionality, (2) exceptional deliberate UI/UX.

Read `MASTER_PLAN.md`, then every file under `docs/`, then `.codex/skills/project-ui-design/SKILL.md` before coding.

## Hard rules
- No AI/LLMs, embeddings, vector DB, API keys, or AI placeholders.
- Single-user/local MVP: no auth, SaaS, billing, organizations, or cloud dependency.
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

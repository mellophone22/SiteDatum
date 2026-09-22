# Architecture

Preferred baseline: Tauri desktop shell + React + TypeScript UI + SQLite. Validate current stable versions and Windows packaging first.

Layers: Presentation -> Application/use-cases -> Domain/rules -> Persistence/SQLite -> Filesystem service -> Platform integrations. React components must not directly own database/filesystem logic.

Principles: typed boundaries, migrations, foreign keys, transactions, structured errors, deterministic Attention rules, minimal dependencies, no network requirement. Normalize/validate paths and never execute project files.

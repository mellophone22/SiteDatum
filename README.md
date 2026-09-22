# Project Engineer Workspace

A local-first Windows desktop workspace for project engineers, built with Tauri, React, TypeScript, Rust, and bundled SQLite.

## Continue development on another workstation

Prerequisites:

- Windows 10 or 11 with WebView2
- Node.js 24 and npm 11
- Rust stable with the `x86_64-pc-windows-msvc` target
- Visual Studio 2022 Build Tools with the Desktop development with C++ workload

Clone and run:

```powershell
git clone https://github.com/kiktio123/AnyDesk.git
cd AnyDesk
npm ci
npm run tauri -- dev
```

Quality gates:

```powershell
npm run lint
npm run test
npm run build
cd src-tauri
cargo fmt --all -- --check
cargo test
```

Build Windows installers:

```powershell
npm run tauri -- build
```

The application database, structured logs, and backups are stored in Windows application-local data and are not committed. Real project documents remain normal Windows files beneath the project root selected in Settings.

## Release artifacts

The `releases/0.1.0/` directory contains the current standalone executable, MSI installer, and NSIS installer. These binaries are unsigned development builds.

Project requirements and architecture documentation are in `MASTER_PLAN.md`, `AGENTS.md`, and `docs/`.

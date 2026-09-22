# WP0 Environment Validation

Validated on 2026-09-21 for Windows development.

| Component | Result |
| --- | --- |
| Node.js | `v24.19.0` |
| npm / npx | `11.17.0`; normal launcher repaired by correcting a stale user-level npm prefix to the installed Node.js directory |
| Corepack | `0.35.0` |
| Rust | `rustc 1.97.1`, `cargo 1.97.1` |
| Rust target | `stable-x86_64-pc-windows-msvc` |
| Rust formatting | `rustfmt-x86_64-pc-windows-msvc` installed and `cargo fmt --check` validated |
| C++ build tools | Visual Studio 2022 Build Tools detected |
| Tauri | `2.11.6` resolved by Cargo |
| SQLite | `rusqlite 0.38.0` with bundled `libsqlite3-sys 0.36.0` |
| React/Vite | React `19.1.0`, Vite `8.3.0` |

The Tauri development launch and production bundle are recorded in the WP0 completion report after their final verification run. Tauri uses Edge WebView2 on Windows; the development baseline is the runtime confirmation.

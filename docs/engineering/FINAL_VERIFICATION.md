# Final MVP Verification

Verified on 2026-09-21 on Windows with Node.js `v24.19.0`, npm `11.17.0`, rustc `1.97.1`, and cargo `1.97.1`.

## Completed behavior

- Missing registered files retain their database records and can be re-associated with an existing replacement through Locate file.
- Drawing metadata includes discipline, drawing number, title, revision, revision date, received date, and current/superseded state.
- Notes and contacts support create, edit, delete, persistence, and activity recording.
- Global search routes to the exact project, task, RFI, submittal, file, note, or contact, including when the result belongs to the already-open screen.
- Ctrl+K opens search, Escape closes it, Ctrl+N starts the active screen's create action, and table record buttons remain standard keyboard-activatable controls.
- Last screen and list filters are remembered in local application preferences.
- Settings creates a checkpointed application-local SQLite backup without copying or mutating project documents.

## Automated results

- Rust formatting: pass (`cargo fmt --all -- --check`).
- Rust tests: 19 passed, including backup reopen and a 500-task persistence/activity volume test.
- ESLint: pass.
- Vitest: 4 passed across structured-error and exact-record-focus tests.
- TypeScript and Vite production build: pass.
- Tauri release build: pass; MSI and NSIS bundles generated.

## Windows bundles

| Artifact | Bytes | SHA-256 |
| --- | ---: | --- |
| `Project Engineer Workspace_0.1.0_x64_en-US.msi` | 4,124,672 | `1F9E04F92561B41EF468BB9B6A841C061F35E59D39C4E4F75DEBD5A894A2B3C9` |
| `Project Engineer Workspace_0.1.0_x64-setup.exe` | 2,277,977 | `9863771CD86506A1246345E5D891702BEAFA860B58CD56FECF056BA183906478` |

Both installers are intentionally unsigned because a Windows code-signing certificate is not configured. Installation was not performed over an existing user installation during verification, avoiding mutation of the user's installed-app state.

## Review limitations and warnings

The Tauri development executable launched successfully and WebView2 hosted the application in the earlier runtime baseline. During the final live visual/accessibility review, the desktop automation helper could not connect because the host's trusted RPC service was not configured (`sky`). This is an external review-tool limitation, not an application error. The final pass therefore used successful Tauri runtime/build checks plus source review of labels, focus outlines, keyboard shortcuts, dialog semantics, horizontal overflow, and reduced-motion behavior. A hands-on release-candidate review remains advisable before external distribution.

Rust commands emit a non-fatal warning that they cannot canonicalize `C:\Users\cabre`; compilation and all tests complete normally. The warning originates from the local toolchain environment rather than application path handling.

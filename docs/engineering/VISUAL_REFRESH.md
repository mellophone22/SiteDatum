# SiteDatum visual refresh and About

## User job and scope

Find, understand, and act on project work quickly while maintaining a clear sense of the current workspace. The highest-frequency surfaces remain project selection, search, quick capture, filters, registers, and record detail.

The visual system now uses the established navy/teal/orange identity consistently across navigation, Home, Attention, registers, detail panels, forms, dialogs, onboarding, and system pages. Thin rules and restrained surfaces replace repeated Home cards. Short entrance/feedback transitions add responsiveness, with a complete reduced-motion override. Short-window navigation, header sizing, and content-sized Quick Capture were corrected during visual review.

About includes the full shared logo, "The project record you control", "Created by Francisco Cabrera", and the software version. The desktop uses Tauri's `getVersion()`; browser review uses package metadata. Failed native reads explicitly say version unavailable. No separately hardcoded version is maintained.

No dependency, database, filesystem, Rust, or sync change. This work is released as SiteDatum 1.3.0 with new Windows packages; the application identifier remains unchanged for upgrade compatibility.

## Verification

- TypeScript/Vite production build, ESLint, and all 53 Vitest tests across 20 files pass.
- Added version tests for native lookup, browser metadata, configuration consistency, and native failure behavior.
- Added persisted About routing/project preservation and command-palette discovery tests.
- `scripts/verify-visual.mjs` uses isolated Playwright browser contexts with 25 fixture projects and 240 fixture tasks. Fixtures are injected only by the test runner and are not shipped with the application.
- Browser verification covers Home, Attention, Projects, Tasks, RFIs, Submittals, Files, Project Controls, Notes & Contacts, Settings, Recovery, About, and first-run preview.
- About text, logo loading, version, project preservation, search-to-About, task-panel Escape/focus restoration, disabled incomplete creation, Quick Capture project inheritance and Escape, and reduced-motion behavior are asserted.
- Layout checks cover 1440x1000, 1280x720, the native default 960x680, native minimum 760x540, and narrow 720/640px widths. Registers retain contained horizontal scrolling rather than overflowing the page.
- No console errors or unexpected alert notices occur in the populated fixture review. An unmocked browser check also confirmed About works without the native bridge.
- Screenshots were inspected and adjustments made for dialog height, compact navigation, and responsive header sizing. Selected screenshots under `docs/ui-snapshots/visual-refresh-*` use test data, not the user's actual records.

Run with `node scripts/verify-visual.mjs` where Playwright is installed, or set `PLAYWRIGHT_MODULE` to an existing Playwright module directory. The runner uses installed Microsoft Edge and the local Vite server on port 1420. Generated review captures go to ignored `tmp/visual-review/`.

## Boundaries

This verifies the React interface using controlled IPC fixtures, not native persistence or an installed Windows release. No real project records or documents were modified. Native installer and end-to-end filesystem release verification remain separate packaging gates.

The Project UI Design Review skill guided density, real-control preservation, keyboard/focus states, restrained color, and motion. Impeccable was searched for in the plugin directory but no matching plugin was returned; no plugin was installed. The available React review guidance was applied to shared branding, typed routes, effect cleanup, and keeping animation dependency-free.

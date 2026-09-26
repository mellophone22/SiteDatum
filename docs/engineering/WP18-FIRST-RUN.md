# WP18 — First-Run Experience

## Entry rule

First-run setup appears only when both conditions are true: no project root is configured and no projects exist. A configured root or any existing project bypasses setup, preventing an upgrade from interrupting established users.

## Flow

1. Choose an existing local folder or UNC share and validate it through the existing Rust command.
2. Enter the first project's minimal identity and review the exact folder path returned by the existing preview command.
3. Create the project through the existing transactional/filesystem-safe backend path and open its Home workspace.

The user can choose **Set up later** to enter normal Settings. Cloud synchronization is explicitly outside the required flow, so offline use is never blocked.

## Safety and testing

No filesystem logic moved into React. Existing validation, refusal to overwrite project folders, and partial-failure reporting remain owned by Rust. The empty-workspace entry predicate is centralized and unit tested. Development builds accept `?first-run-preview` solely for visual verification without altering persisted workspace data; production builds ignore it.

import { describe, expect, it } from "vitest";
import { defaultWorkspaceState, readWorkspaceState, workspaceReducer } from "./workspaceState";

const storage = (values: Record<string, string>) => ({ getItem: (key: string) => values[key] ?? null });

describe("workspace state", () => {
  it("opens and restores About without changing the working project", () => {
    const state = { ...defaultWorkspaceState, currentProjectId: "project-2" };
    expect(workspaceReducer(state, { type: "navigate", screen: "about" })).toMatchObject({ currentScreen: "about", currentProjectId: "project-2" });
    expect(readWorkspaceState(storage({ "workspace.screen": "about" })).currentScreen).toBe("about");
  });
  it("restores valid persisted context and rejects an invalid screen", () => {
    expect(readWorkspaceState(storage({ "workspace.screen": "rfis", "workspace.projectContext": "project-7" }))).toMatchObject({ currentScreen: "rfis", currentProjectId: "project-7" });
    expect(readWorkspaceState(storage({ "workspace.screen": "not-a-screen" })).currentScreen).toBe("attention");
  });

  it("opens a record while preserving an unspecified project", () => {
    const state = { ...defaultWorkspaceState, currentProjectId: "project-2" };
    const next = workspaceReducer(state, { type: "open-record", screen: "tasks", record: { type: "task", id: "task-4" } });
    expect(next).toMatchObject({ currentScreen: "tasks", currentProjectId: "project-2", focusedRecord: { type: "task", id: "task-4" }, navigationRevision: 1 });
  });

  it("clears stale persisted project context after projects load", () => {
    const state = { ...defaultWorkspaceState, currentProjectId: "missing-project" };
    expect(workspaceReducer(state, { type: "reconcile-projects", projectIds: ["project-1"] }).currentProjectId).toBeNull();
  });

  it("increments project and workspace refresh revisions independently", () => {
    const projects = workspaceReducer(defaultWorkspaceState, { type: "refresh-projects" });
    expect(projects).toMatchObject({ projectListRevision: 1, navigationRevision: 0 });
    const workspace = workspaceReducer(projects, { type: "refresh-workspace" });
    expect(workspace).toMatchObject({ projectListRevision: 2, navigationRevision: 1 });
  });

  it("establishes a project workspace before navigating home", () => {
    const selected = workspaceReducer(defaultWorkspaceState, { type: "set-project", projectId: "project-9" });
    const opened = workspaceReducer(selected, { type: "navigate", screen: "overview" });
    expect(opened).toMatchObject({ currentProjectId: "project-9", currentScreen: "overview" });
  });
});

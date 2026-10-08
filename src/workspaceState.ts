export const screens = [
  "overview", "projects", "tasks", "rfis", "submittals", "files", "notes",
  "operations", "attention", "recovery", "settings", "help", "about",
] as const;

export type Screen = (typeof screens)[number];
export type FocusedRecord = { type: string; id: string };

export type WorkspaceState = {
  currentProjectId: string | null;
  currentScreen: Screen;
  focusedRecord: FocusedRecord | null;
  navigationRevision: number;
  projectListRevision: number;
};

export type WorkspaceAction =
  | { type: "set-project"; projectId: string | null }
  | { type: "navigate"; screen: Screen }
  | { type: "open-record"; screen: Screen; record: FocusedRecord; projectId?: string | null }
  | { type: "clear-focus" }
  | { type: "refresh-projects" }
  | { type: "refresh-workspace" }
  | { type: "reconcile-projects"; projectIds: string[] };

export const defaultWorkspaceState: WorkspaceState = {
  currentProjectId: null,
  currentScreen: "attention",
  focusedRecord: null,
  navigationRevision: 0,
  projectListRevision: 0,
};

export function isScreen(value: unknown): value is Screen {
  return typeof value === "string" && screens.includes(value as Screen);
}

export function readWorkspaceState(storage: Pick<Storage, "getItem">): WorkspaceState {
  const storedScreen = storage.getItem("workspace.screen");
  const storedProject = storage.getItem("workspace.projectContext");
  return {
    ...defaultWorkspaceState,
    currentScreen: isScreen(storedScreen) ? storedScreen : defaultWorkspaceState.currentScreen,
    currentProjectId: storedProject?.trim() || null,
  };
}

export function workspaceReducer(state: WorkspaceState, action: WorkspaceAction): WorkspaceState {
  switch (action.type) {
    case "set-project":
      return { ...state, currentProjectId: action.projectId, focusedRecord: null };
    case "navigate":
      return { ...state, currentScreen: action.screen, focusedRecord: null };
    case "open-record":
      return {
        ...state,
        currentProjectId: action.projectId === undefined ? state.currentProjectId : action.projectId,
        currentScreen: action.screen,
        focusedRecord: action.record,
        navigationRevision: state.navigationRevision + 1,
      };
    case "clear-focus":
      return { ...state, focusedRecord: null };
    case "refresh-projects":
      return { ...state, projectListRevision: state.projectListRevision + 1 };
    case "refresh-workspace":
      return { ...state, focusedRecord: null, navigationRevision: state.navigationRevision + 1, projectListRevision: state.projectListRevision + 1 };
    case "reconcile-projects":
      return state.currentProjectId && !action.projectIds.includes(state.currentProjectId)
        ? { ...state, currentProjectId: null, focusedRecord: null }
        : state;
  }
}

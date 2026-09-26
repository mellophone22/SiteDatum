import { createContext, useContext, useEffect, useMemo, useReducer, type ReactNode } from "react";
import { readWorkspaceState, workspaceReducer, type FocusedRecord, type Screen, type WorkspaceState } from "./workspaceState";

type WorkspaceContextValue = {
  state: WorkspaceState;
  setCurrentProject: (projectId: string | null) => void;
  navigate: (screen: Screen) => void;
  openRecord: (screen: Screen, record: FocusedRecord, projectId?: string | null) => void;
  refreshProjects: () => void;
  refreshWorkspace: () => void;
  reconcileProjects: (projectIds: string[]) => void;
};

const WorkspaceContext = createContext<WorkspaceContextValue | null>(null);

export function WorkspaceProvider({ children }: { children: ReactNode }) {
  const [state, dispatch] = useReducer(workspaceReducer, undefined, () => readWorkspaceState(localStorage));

  useEffect(() => localStorage.setItem("workspace.screen", state.currentScreen), [state.currentScreen]);
  useEffect(() => {
    const value = state.currentProjectId ?? "";
    localStorage.setItem("workspace.projectContext", value);
    window.dispatchEvent(new CustomEvent("workspace:project", { detail: value }));
  }, [state.currentProjectId]);

  const value = useMemo<WorkspaceContextValue>(() => ({
    state,
    setCurrentProject: (projectId) => dispatch({ type: "set-project", projectId }),
    navigate: (screen) => dispatch({ type: "navigate", screen }),
    openRecord: (screen, record, projectId) => {
      localStorage.setItem("workspace.focus", JSON.stringify({ screen, id: record.id }));
      dispatch({ type: "open-record", screen, record, projectId });
    },
    refreshProjects: () => dispatch({ type: "refresh-projects" }),
    refreshWorkspace: () => dispatch({ type: "refresh-workspace" }),
    reconcileProjects: (projectIds) => dispatch({ type: "reconcile-projects", projectIds }),
  }), [state]);

  return <WorkspaceContext.Provider value={value}>{children}</WorkspaceContext.Provider>;
}

export function useWorkspace() {
  const value = useContext(WorkspaceContext);
  if (!value) throw new Error("useWorkspace must be used within WorkspaceProvider");
  return value;
}

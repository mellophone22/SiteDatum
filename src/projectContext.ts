import { useWorkspace } from "./workspace";

export function useProjectContext() {
  return useWorkspace().state.currentProjectId ?? "";
}

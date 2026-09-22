import { useEffect, useState } from "react";

export function useProjectContext() {
  const [projectId, setProjectId] = useState(() => localStorage.getItem("workspace.projectContext") ?? "");
  useEffect(() => {
    const handleProject = (event: Event) => setProjectId((event as CustomEvent<string>).detail ?? "");
    window.addEventListener("workspace:project", handleProject);
    return () => window.removeEventListener("workspace:project", handleProject);
  }, []);
  return projectId;
}

export const projectModules = ["tasks", "rfis", "submittals", "files", "operations", "notes"] as const;
export type ProjectModule = typeof projectModules[number];

const phaseOrders: Record<string, ProjectModule[]> = {
  preconstruction: ["operations", "tasks", "files", "rfis", "submittals", "notes"],
  engineering: ["rfis", "submittals", "tasks", "files", "operations", "notes"],
  submittals: ["submittals", "rfis", "tasks", "files", "operations", "notes"],
  procurement: ["operations", "submittals", "tasks", "files", "rfis", "notes"],
  construction: ["operations", "tasks", "rfis", "submittals", "files", "notes"],
  programming: ["tasks", "files", "operations", "rfis", "submittals", "notes"],
  startup: ["operations", "tasks", "files", "rfis", "submittals", "notes"],
  commissioning: ["operations", "tasks", "files", "rfis", "submittals", "notes"],
  closeout: ["files", "submittals", "tasks", "operations", "rfis", "notes"],
};

export function projectModuleOrder(phase?: string | null): ProjectModule[] {
  return [...(phase && phaseOrders[phase] ? phaseOrders[phase] : projectModules)];
}

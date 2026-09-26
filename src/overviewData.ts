export type ProjectAttentionItem = { projectId: string; projectNumber: string; projectName: string };
export type ProjectAttentionSummary = ProjectAttentionItem & { tasks: number; rfis: number; submittals: number; total: number };

export function summarizeProjectAttention(tasks: ProjectAttentionItem[], rfis: ProjectAttentionItem[], submittals: ProjectAttentionItem[]): ProjectAttentionSummary[] {
  const summaries = new Map<string, ProjectAttentionSummary>();
  const add = (item: ProjectAttentionItem, kind: "tasks" | "rfis" | "submittals") => {
    const current = summaries.get(item.projectId) ?? { ...item, tasks: 0, rfis: 0, submittals: 0, total: 0 };
    current[kind] += 1;
    current.total += 1;
    summaries.set(item.projectId, current);
  };
  tasks.forEach((item) => add(item, "tasks"));
  rfis.forEach((item) => add(item, "rfis"));
  submittals.forEach((item) => add(item, "submittals"));
  return [...summaries.values()].sort((a, b) => b.total - a.total || a.projectNumber.localeCompare(b.projectNumber));
}

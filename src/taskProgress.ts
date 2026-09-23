export type ProgressTask = { status: string };

export type TaskProgressSummary = {
  total: number;
  completed: number;
  open: number;
  inProgress: number;
  waiting: number;
  blocked: number;
  percent: number;
};

export function summarizeTaskProgress(tasks: ProgressTask[]): TaskProgressSummary {
  const included = tasks.filter((task) => task.status !== "cancelled");
  const count = (status: string) => included.filter((task) => task.status === status).length;
  const completed = count("completed");
  const total = included.length;
  return {
    total,
    completed,
    open: count("open"),
    inProgress: count("in_progress"),
    waiting: count("waiting"),
    blocked: count("blocked"),
    percent: total === 0 ? 0 : Math.round((completed / total) * 100),
  };
}

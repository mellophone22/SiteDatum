export type EditableTask = {
  projectId: string;
  title: string;
  description: string | null;
  priority: string;
  status: string;
  category: string | null;
  dueDate: string | null;
  followUpDate: string | null;
  waitingOn: string | null;
};

export type TaskDraft = {
  projectId: string;
  title: string;
  description: string;
  priority: string;
  status: string;
  category: string;
  dueDate: string;
  followUpDate: string;
  waitingOn: string;
};

export function taskToDraft(task: EditableTask): TaskDraft {
  return {
    projectId: task.projectId,
    title: task.title,
    description: task.description ?? "",
    priority: task.priority,
    status: task.status,
    category: task.category ?? "",
    dueDate: task.dueDate ?? "",
    followUpDate: task.followUpDate ?? "",
    waitingOn: task.waitingOn ?? "",
  };
}

export function taskDraftIsValid(draft: TaskDraft): boolean {
  return Boolean(draft.projectId && draft.title.trim() && (draft.status !== "waiting" || draft.waitingOn.trim()));
}

export function taskDraftPayload(draft: TaskDraft) {
  return {
    ...draft,
    title: draft.title.trim(),
    description: draft.description.trim() || null,
    category: draft.category.trim() || null,
    dueDate: draft.dueDate || null,
    followUpDate: draft.status === "waiting" ? draft.followUpDate || null : null,
    waitingOn: draft.status === "waiting" ? draft.waitingOn.trim() : null,
  };
}

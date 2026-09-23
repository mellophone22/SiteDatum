export type TaskViewFilters = {
  query: string;
  project: string;
  status: string;
  priority: string;
  sort: string;
};

export type SavedTaskView = TaskViewFilters & { id: string; name: string };

const storageKey = "tasks.savedViews";

export function readTaskViews(storage: Pick<Storage, "getItem"> = localStorage): SavedTaskView[] {
  try {
    const value = JSON.parse(storage.getItem(storageKey) ?? "[]");
    if (!Array.isArray(value)) return [];
    return value.filter((item): item is SavedTaskView => Boolean(item && typeof item.id === "string" && typeof item.name === "string"));
  } catch {
    return [];
  }
}

export function writeTaskViews(views: SavedTaskView[], storage: Pick<Storage, "setItem"> = localStorage) {
  storage.setItem(storageKey, JSON.stringify(views));
}

export function createTaskView(name: string, filters: TaskViewFilters): SavedTaskView {
  return { id: crypto.randomUUID(), name: name.trim(), ...filters };
}

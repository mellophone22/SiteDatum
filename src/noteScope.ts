export type NoteScope = "context" | "workspace" | "all";

export function filterNotesByScope<T extends { projectId: string | null }>(
  notes: T[],
  scope: NoteScope,
  projectId: string,
) {
  if (scope === "all" || (scope === "context" && !projectId)) return notes;
  if (scope === "workspace") return notes.filter((note) => !note.projectId);
  return notes.filter((note) => note.projectId === projectId);
}

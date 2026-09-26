import { describe, expect, it } from "vitest";
import { filterNotesByScope } from "./noteScope";

const notes = [
  { id: "project-a", projectId: "a" },
  { id: "project-b", projectId: "b" },
  { id: "workspace", projectId: null },
];

describe("filterNotesByScope", () => {
  it("shows the selected project's notes in project context", () => {
    expect(filterNotesByScope(notes, "context", "a").map((note) => note.id)).toEqual(["project-a"]);
  });

  it("shows all notes when All Projects is the context", () => {
    expect(filterNotesByScope(notes, "context", "")).toEqual(notes);
  });

  it("can isolate workspace-wide notes", () => {
    expect(filterNotesByScope(notes, "workspace", "a").map((note) => note.id)).toEqual(["workspace"]);
  });

  it("can show every note from a project context", () => {
    expect(filterNotesByScope(notes, "all", "a")).toEqual(notes);
  });
});

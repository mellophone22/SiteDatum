import { beforeEach, describe, expect, it, vi } from "vitest";
import { takeFocus } from "./focus";

const values = new Map<string, string>();

beforeEach(() => {
  values.clear();
  vi.stubGlobal("localStorage", {
    getItem: (key: string) => values.get(key) ?? null,
    setItem: (key: string, value: string) => values.set(key, value),
    removeItem: (key: string) => values.delete(key),
  });
});

describe("takeFocus", () => {
  it("consumes an exact matching record focus", () => {
    localStorage.setItem("workspace.focus", JSON.stringify({ screen: "files", id: "file-17" }));
    expect(takeFocus("files")).toBe("file-17");
    expect(localStorage.getItem("workspace.focus")).toBeNull();
  });

  it("leaves focus intended for another screen", () => {
    localStorage.setItem("workspace.focus", JSON.stringify({ screen: "projects", id: "project-4" }));
    expect(takeFocus("tasks")).toBeNull();
    expect(localStorage.getItem("workspace.focus")).not.toBeNull();
  });

  it("clears malformed focus data safely", () => {
    localStorage.setItem("workspace.focus", "not-json");
    expect(takeFocus("files")).toBeNull();
    expect(localStorage.getItem("workspace.focus")).toBeNull();
  });
});

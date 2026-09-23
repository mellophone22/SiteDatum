import { describe, expect, it } from "vitest";
import { createTaskView, readTaskViews, writeTaskViews } from "./savedViews";

describe("task saved views", () => {
  it("round trips a named filter set", () => {
    let raw = "";
    const storage = { getItem: () => raw, setItem: (_key: string, value: string) => { raw = value; } };
    const view = createTaskView("Urgent work", { query: "", project: "p1", status: "open", priority: "urgent", sort: "priority" });
    writeTaskViews([view], storage);
    expect(readTaskViews(storage)).toEqual([view]);
  });

  it("recovers safely from malformed storage", () => {
    expect(readTaskViews({ getItem: () => "not-json" })).toEqual([]);
  });
});

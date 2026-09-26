import { describe, expect, it } from "vitest";
import { inheritedProjectId } from "./projectInheritance";

describe("inheritedProjectId", () => {
  it("defaults a new record to the selected project", () => {
    expect(inheritedProjectId("project-1")).toBe("project-1");
  });
  it("requires a choice in All Projects context", () => {
    expect(inheritedProjectId("")).toBe("");
  });
  it("preserves a deliberate project override", () => {
    expect(inheritedProjectId("project-1", "project-2")).toBe("project-2");
  });
});

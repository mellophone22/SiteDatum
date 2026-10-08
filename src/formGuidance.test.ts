import { describe, expect, it } from "vitest";
import { firstFieldError, requiredFieldErrors } from "./formGuidance";

describe("form guidance", () => {
  it("returns field-specific messages for required blank values", () => {
    expect(requiredFieldErrors([
      { key: "project", label: "Project", value: "" },
      { key: "title", label: "Task title", value: "  " },
      { key: "notes", label: "Notes", value: "", required: false },
    ])).toEqual({ project: "Project is required.", title: "Task title is required." });
  });

  it("treats nonblank values as complete and preserves field order", () => {
    const errors = requiredFieldErrors([
      { key: "number", label: "Project number", value: "C9-001" },
      { key: "name", label: "Project name", value: "" },
      { key: "phase", label: "Custom phase name", value: "" },
    ]);
    expect(firstFieldError(errors)).toBe("name");
    expect(errors).toEqual({ name: "Project name is required.", phase: "Custom phase name is required." });
  });
});

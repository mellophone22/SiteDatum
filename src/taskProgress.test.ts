import { describe, expect, it } from "vitest";
import { summarizeTaskProgress } from "./taskProgress";

describe("summarizeTaskProgress", () => {
  it("measures completed work against all non-cancelled tasks", () => {
    expect(summarizeTaskProgress([
      { status: "completed" },
      { status: "completed" },
      { status: "in_progress" },
      { status: "open" },
      { status: "cancelled" },
    ])).toEqual({ total: 4, completed: 2, open: 1, inProgress: 1, waiting: 0, blocked: 0, percent: 50 });
  });

  it("returns a stable zero state when no work is tracked", () => {
    expect(summarizeTaskProgress([{ status: "cancelled" }]).percent).toBe(0);
  });
});

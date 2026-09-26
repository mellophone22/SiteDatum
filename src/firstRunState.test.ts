import { describe, expect, it } from "vitest";
import { needsFirstRun } from "./firstRunState";

describe("needsFirstRun", () => {
  it("starts setup only for a completely empty workspace", () => {
    expect(needsFirstRun(null, 0)).toBe(true);
  });

  it("does not interrupt an already configured workspace", () => {
    expect(needsFirstRun("C:\\Projects", 0)).toBe(false);
    expect(needsFirstRun(null, 1)).toBe(false);
    expect(needsFirstRun("C:\\Projects", 12)).toBe(false);
  });
});

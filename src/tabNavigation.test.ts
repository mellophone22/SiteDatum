import { describe, expect, it } from "vitest";
import { nextTabIndex } from "./tabNavigation";

describe("nextTabIndex", () => {
  it("wraps arrow navigation in both directions", () => {
    expect(nextTabIndex(3, 4, "ArrowRight")).toBe(0);
    expect(nextTabIndex(0, 4, "ArrowLeft")).toBe(3);
  });

  it("supports Home and End", () => {
    expect(nextTabIndex(2, 4, "Home")).toBe(0);
    expect(nextTabIndex(1, 4, "End")).toBe(3);
  });
});

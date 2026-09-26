import { describe, expect, it } from "vitest";
import { shouldCloseDetailPanel } from "./detailPanelState";

describe("shouldCloseDetailPanel", () => {
  it("closes for an unhandled Escape key", () => {
    expect(shouldCloseDetailPanel({ key: "Escape" })).toBe(true);
  });

  it("does not close for other, handled, or composing keys", () => {
    expect(shouldCloseDetailPanel({ key: "Enter" })).toBe(false);
    expect(shouldCloseDetailPanel({ key: "Escape", defaultPrevented: true })).toBe(false);
    expect(shouldCloseDetailPanel({ key: "Escape", isComposing: true })).toBe(false);
  });
});

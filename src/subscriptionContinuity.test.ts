import { describe, expect, it } from "vitest";
import { continuityState } from "./subscriptionContinuity";

const DAY = 24 * 60 * 60;

describe("subscription continuity notices", () => {
  it("stays quiet through the first thirteen days", () => {
    expect(continuityState(1_000, 1_000 + 21 * DAY, 1_000 + 12 * DAY).stage).toBe("none");
  });

  it("uses a quiet recovery notice on days fourteen through seventeen", () => {
    expect(continuityState(1_000, 1_000 + 21 * DAY, 1_000 + 13 * DAY)).toEqual({ stage: "quiet", daysRemaining: 8 });
    expect(continuityState(1_000, 1_000 + 21 * DAY, 1_000 + 16 * DAY).stage).toBe("quiet");
  });

  it("uses the stronger recovery notice for the final four days", () => {
    expect(continuityState(1_000, 1_000 + 21 * DAY, 1_000 + 17 * DAY)).toEqual({ stage: "urgent", daysRemaining: 4 });
  });
});

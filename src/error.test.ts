import { describe, expect, it } from "vitest";
import { describeAppError } from "./error";

describe("describeAppError", () => {
  it("shows recovery and a correlation reference when supplied", () => {
    expect(describeAppError({ message: "Root is unavailable.", recovery: "Reconnect the share.", correlationId: "abc" })).toBe("Root is unavailable. Reconnect the share. Reference: abc.");
  });
});

import { describe, expect, it } from "vitest";
import { recoveryIssueCount, recoverySummary } from "./recoveryHealth";

describe("recovery health", () => {
  it("keeps a healthy workspace quiet", () => {
    const health = { missingFiles: 0, syncConflicts: 0 };
    expect(recoveryIssueCount(health)).toBe(0);
    expect(recoverySummary(health)).toBe("No recovery issues detected.");
  });

  it("summarizes each actionable recovery issue", () => {
    const health = { missingFiles: 2, syncConflicts: 1 };
    expect(recoveryIssueCount(health)).toBe(3);
    expect(recoverySummary(health)).toBe("2 missing files and 1 sync conflict need attention.");
  });
});

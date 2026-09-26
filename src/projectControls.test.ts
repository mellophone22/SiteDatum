import { describe, expect, it } from "vitest";
import { projectControlGroups, projectControlRegisters } from "./projectControls";

describe("project controls register hierarchy", () => {
  it("keeps every register reachable exactly once", () => {
    expect(projectControlRegisters).toHaveLength(10);
    expect(new Set(projectControlRegisters).size).toBe(10);
    expect(projectControlRegisters).toEqual(expect.arrayContaining(["meeting_minute","procurement","change_event","transmittal","milestone","punch_item","daily_report","startup_check","commissioning_check","commissioning_issue"]));
  });

  it("uses the four handoff categories", () => {
    expect(projectControlGroups.map(group=>group.label)).toEqual(["Coordination","Commercial","Schedule / Field","Startup & Commissioning"]);
  });
});

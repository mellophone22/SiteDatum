import { describe, expect, it } from "vitest";
import { projectModuleOrder, projectModules } from "./phasePrioritization";

describe("phase-aware module prioritization", () => {
  it("prioritizes phase-relevant modules", () => {
    expect(projectModuleOrder("engineering").slice(0,2)).toEqual(["rfis","submittals"]);
    expect(projectModuleOrder("construction").slice(0,2)).toEqual(["operations","tasks"]);
    expect(projectModuleOrder("closeout").slice(0,2)).toEqual(["files","submittals"]);
  });

  it("never hides or duplicates a project module", () => {
    for (const phase of ["preconstruction","engineering","submittals","procurement","construction","programming","startup","commissioning","closeout","custom",undefined]) {
      const order=projectModuleOrder(phase);
      expect(order).toHaveLength(projectModules.length);
      expect(new Set(order)).toEqual(new Set(projectModules));
    }
  });

  it("uses the stable default order for custom or missing phases", () => {
    expect(projectModuleOrder("custom")).toEqual(projectModules);
    expect(projectModuleOrder()).toEqual(projectModules);
  });
});

import { describe, expect, it } from "vitest";
import { summarizeProjectAttention } from "./overviewData";

const project = (projectId: string, projectNumber: string) => ({ projectId, projectNumber, projectName: `Project ${projectNumber}` });

describe("summarizeProjectAttention", () => {
  it("combines operational attention by project and sorts by total", () => {
    const result = summarizeProjectAttention([project("a", "100"), project("a", "100"), project("b", "200")], [project("b", "200"), project("b", "200")], [project("a", "100")]);
    expect(result.map(({ projectId, tasks, rfis, submittals, total }) => ({ projectId, tasks, rfis, submittals, total }))).toEqual([
      { projectId: "a", tasks: 2, rfis: 0, submittals: 1, total: 3 },
      { projectId: "b", tasks: 1, rfis: 2, submittals: 0, total: 3 },
    ]);
  });
});

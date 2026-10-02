import { describe, expect, it } from "vitest";
import { rfiPayload, type RfiFormInput } from "./rfiPayload";

const form = (overrides: Partial<RfiFormInput> = {}): RfiFormInput => ({
  projectId: "project-1",
  number: "RFI-001",
  subject: "Sensor mounting height",
  question: "What height should be used?",
  recipient: "",
  status: "draft",
  createdDate: "2026-10-02",
  submittedDate: "",
  responseDueDate: "",
  responseReceivedDate: "",
  response: "",
  notes: "",
  rfiLocation: "",
  drawingNumber: "",
  costImpact: "",
  timeDelay: "",
  suggestedSolution: "",
  requestedBy: "",
  relatedTaskId: "",
  ...overrides,
});

describe("rfiPayload", () => {
  it("omits blank optional dates and text instead of sending invalid empty strings", () => {
    expect(rfiPayload(form())).toMatchObject({
      recipient: null,
      submittedDate: null,
      responseDueDate: null,
      responseReceivedDate: null,
      relatedTaskId: null,
    });
  });

  it("preserves entered optional values", () => {
    expect(rfiPayload(form({
      recipient: "Fictional Design Lead",
      responseDueDate: "2026-10-12",
      relatedTaskId: "task-1",
    }))).toMatchObject({
      recipient: "Fictional Design Lead",
      responseDueDate: "2026-10-12",
      relatedTaskId: "task-1",
    });
  });
});

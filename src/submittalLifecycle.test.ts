import { describe, expect, it } from "vitest";
import { applySubmittalLifecycleAction, submittalLifecycleAction } from "./submittalLifecycle";

const draft = { status: "draft", recipient: "", submittedDate: "", responseDate: "", resubmissionRequired: false };

describe("submittal lifecycle helpers", () => {
  it("requires a recipient and defaults the submission date without replacing one", () => {
    expect(submittalLifecycleAction(draft, "approved")?.disabledReason).toContain("recipient");
    expect(applySubmittalLifecycleAction({ ...draft, recipient: "Architect" }, "submitted", "2026-09-25").submittedDate).toBe("2026-09-25");
    expect(applySubmittalLifecycleAction({ ...draft, recipient: "Architect", submittedDate: "2026-09-20" }, "submitted", "2026-09-25").submittedDate).toBe("2026-09-20");
  });

  it("records a disposition date and marks revise-and-resubmit as requiring revision", () => {
    const result = applySubmittalLifecycleAction({ ...draft, status: "under_review", recipient: "Architect" }, "revise_and_resubmit", "2026-09-26");
    expect(result).toMatchObject({ status: "revise_and_resubmit", responseDate: "2026-09-26", resubmissionRequired: true });
    expect(submittalLifecycleAction(result, "approved")?.kind).toBe("revision");
  });

  it("closes final dispositions and leaves closed records terminal", () => {
    const approved = { ...draft, status: "approved", recipient: "Architect", responseDate: "2026-09-26" };
    expect(submittalLifecycleAction(approved, "approved")?.label).toBe("Close submittal");
    expect(submittalLifecycleAction({ ...approved, status: "closed" }, "approved")).toBeNull();
  });
});

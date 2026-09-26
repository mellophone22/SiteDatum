import { describe, expect, it } from "vitest";
import { applyRfiLifecycleAction, rfiLifecycleAction } from "./rfiLifecycle";

const draft = { status: "draft", recipient: "", submittedDate: "", responseReceivedDate: "", response: "" };

describe("RFI lifecycle helpers", () => {
  it("requires a recipient before submission and retains a supplied submission date", () => {
    expect(rfiLifecycleAction(draft)?.disabledReason).toContain("recipient");
    expect(applyRfiLifecycleAction({ ...draft, recipient: "Engineer", submittedDate: "2026-09-20" }, "open", "2026-09-25").submittedDate).toBe("2026-09-20");
  });

  it("defaults lifecycle dates without replacing entered dates", () => {
    const opened = applyRfiLifecycleAction({ ...draft, recipient: "Engineer" }, "open", "2026-09-25");
    expect(opened).toMatchObject({ status: "open", submittedDate: "2026-09-25" });
    const received = applyRfiLifecycleAction({ ...opened, response: "Proceed with sequence B." }, "response_received", "2026-09-26");
    expect(received).toMatchObject({ status: "response_received", responseReceivedDate: "2026-09-26" });
  });

  it("exposes only the next normal action and leaves closed RFIs terminal", () => {
    expect(rfiLifecycleAction({ ...draft, status: "open", response: "" })?.label).toBe("Record response");
    expect(rfiLifecycleAction({ ...draft, status: "response_received", response: "Recorded" })?.label).toBe("Close RFI");
    expect(rfiLifecycleAction({ ...draft, status: "closed", response: "Recorded" })).toBeNull();
  });
});

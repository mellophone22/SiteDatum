export type RfiLifecycleStatus = "draft" | "open" | "response_received" | "closed";

export type RfiLifecycleDraft = {
  status: string;
  recipient: string;
  submittedDate: string;
  responseReceivedDate: string;
  response: string;
};

export type RfiLifecycleAction = {
  label: string;
  nextStatus: RfiLifecycleStatus;
  disabledReason: string | null;
};

export function rfiLifecycleAction(draft: RfiLifecycleDraft): RfiLifecycleAction | null {
  if (draft.status === "draft") return { label: "Submit RFI", nextStatus: "open", disabledReason: draft.recipient.trim() ? null : "Enter a recipient before submitting." };
  if (draft.status === "open") return { label: "Record response", nextStatus: "response_received", disabledReason: draft.response.trim() ? null : "Enter the response before recording it." };
  if (draft.status === "response_received") return { label: "Close RFI", nextStatus: "closed", disabledReason: draft.response.trim() ? null : "A recorded response is required before closing." };
  return null;
}

export function applyRfiLifecycleAction<T extends RfiLifecycleDraft>(draft: T, nextStatus: RfiLifecycleStatus, localDate: string): T {
  return {
    ...draft,
    status: nextStatus,
    submittedDate: nextStatus === "open" && !draft.submittedDate ? localDate : draft.submittedDate,
    responseReceivedDate: nextStatus === "response_received" && !draft.responseReceivedDate ? localDate : draft.responseReceivedDate,
  };
}

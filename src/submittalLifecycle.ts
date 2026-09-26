export type SubmittalDisposition = "approved" | "approved_as_noted" | "revise_and_resubmit" | "rejected";
export type SubmittalLifecycleStatus = "submitted" | SubmittalDisposition | "closed";

export type SubmittalLifecycleDraft = {
  status: string;
  recipient: string;
  submittedDate: string;
  responseDate: string;
  resubmissionRequired: boolean;
};

export type SubmittalLifecycleAction = {
  kind: "transition" | "revision";
  label: string;
  nextStatus?: SubmittalLifecycleStatus;
  disabledReason: string | null;
};

export function submittalLifecycleAction(draft: SubmittalLifecycleDraft, disposition: SubmittalDisposition): SubmittalLifecycleAction | null {
  if (["draft", "preparing"].includes(draft.status)) return { kind: "transition", label: "Submit submittal", nextStatus: "submitted", disabledReason: draft.recipient.trim() ? null : "Enter a recipient before submitting." };
  if (["submitted", "under_review"].includes(draft.status)) return { kind: "transition", label: "Record disposition", nextStatus: disposition, disabledReason: null };
  if (draft.status === "revise_and_resubmit" && draft.resubmissionRequired) return { kind: "revision", label: "Create revision", disabledReason: null };
  if (["approved", "approved_as_noted", "revise_and_resubmit", "rejected"].includes(draft.status)) return { kind: "transition", label: "Close submittal", nextStatus: "closed", disabledReason: draft.responseDate ? null : "Record a response date before closing." };
  return null;
}

export function applySubmittalLifecycleAction<T extends SubmittalLifecycleDraft>(draft: T, nextStatus: SubmittalLifecycleStatus, localDate: string): T {
  const isDisposition = ["approved", "approved_as_noted", "revise_and_resubmit", "rejected"].includes(nextStatus);
  return {
    ...draft,
    status: nextStatus,
    submittedDate: nextStatus === "submitted" && !draft.submittedDate ? localDate : draft.submittedDate,
    responseDate: isDisposition && !draft.responseDate ? localDate : draft.responseDate,
    resubmissionRequired: nextStatus === "revise_and_resubmit" ? true : nextStatus === "closed" ? draft.resubmissionRequired : false,
  };
}

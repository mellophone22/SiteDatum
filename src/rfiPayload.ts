export type RfiFormInput = {
  projectId: string;
  number: string;
  subject: string;
  question: string;
  recipient: string;
  status: string;
  createdDate: string;
  submittedDate: string;
  responseDueDate: string;
  responseReceivedDate: string;
  response: string;
  notes: string;
  rfiLocation: string;
  drawingNumber: string;
  costImpact: string;
  timeDelay: string;
  suggestedSolution: string;
  requestedBy: string;
  relatedTaskId: string;
};

export function rfiPayload(input: RfiFormInput) {
  return {
    ...input,
    recipient: input.recipient || null,
    submittedDate: input.submittedDate || null,
    responseDueDate: input.responseDueDate || null,
    responseReceivedDate: input.responseReceivedDate || null,
    response: input.response || null,
    notes: input.notes || null,
    rfiLocation: input.rfiLocation || null,
    drawingNumber: input.drawingNumber || null,
    costImpact: input.costImpact || null,
    timeDelay: input.timeDelay || null,
    suggestedSolution: input.suggestedSolution || null,
    requestedBy: input.requestedBy || null,
    relatedTaskId: input.relatedTaskId || null,
  };
}

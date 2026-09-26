import { describe, expect, it } from "vitest";
import { taskDraftIsValid, taskDraftPayload, taskToDraft } from "./taskEditor";

describe("task editor helpers", () => {
  it("retains complete task detail in an editable draft", () => {
    const draft=taskToDraft({projectId:"p1",title:"Coordinate controls",description:"Review sequence",priority:"high",status:"waiting",category:"Controls",dueDate:"2026-10-01",followUpDate:"2026-09-28",waitingOn:"Vendor"});
    expect(draft.description).toBe("Review sequence");
    expect(draft.category).toBe("Controls");
    expect(taskDraftIsValid(draft)).toBe(true);
  });

  it("requires waiting-on detail only for Waiting and clears it otherwise", () => {
    const waiting={projectId:"p1",title:"Task",description:"",priority:"medium",status:"waiting",category:"",dueDate:"",followUpDate:"",waitingOn:""};
    expect(taskDraftIsValid(waiting)).toBe(false);
    expect(taskDraftPayload({...waiting,status:"open",waitingOn:"Vendor",followUpDate:"2026-10-01"})).toMatchObject({waitingOn:null,followUpDate:null});
  });
});

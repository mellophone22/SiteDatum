import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { describeAppError } from "./error";

type Project = { id: string; number: string; name: string };
type CaptureType = "task" | "note" | "rfi" | "contact";

function today() {
  const value = new Date();
  return `${value.getFullYear()}-${String(value.getMonth() + 1).padStart(2, "0")}-${String(value.getDate()).padStart(2, "0")}`;
}

export function QuickCapture({ open, projects, projectContext, onClose, onSaved }: { open: boolean; projects: Project[]; projectContext: string; onClose: () => void; onSaved: (screen: string) => void }) {
  const [type, setType] = useState<CaptureType>("task");
  const [projectId, setProjectId] = useState(projectContext);
  const [title, setTitle] = useState("");
  const [detail, setDetail] = useState("");
  const [secondary, setSecondary] = useState("");
  const [date, setDate] = useState("");
  const [priority, setPriority] = useState("medium");
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState("");
  const dialogRef = useRef<HTMLElement>(null);

  useEffect(() => { if (open) { setProjectId(projectContext); setError(""); } }, [open, projectContext]);
  useEffect(() => { if (!open) return; const key=(event:KeyboardEvent)=>{if(event.key==="Escape")onClose()}; window.addEventListener("keydown",key); return()=>window.removeEventListener("keydown",key); }, [open, onClose]);

  function reset() { setTitle(""); setDetail(""); setSecondary(""); setDate(""); setPriority("medium"); setError(""); }
  function changeType(value: CaptureType) { setType(value); reset(); }
  async function save(event: React.FormEvent) {
    event.preventDefault(); setSaving(true); setError("");
    try {
      if (type === "task") await invoke("create_task", { input: { projectId, title, priority, status: "open", dueDate: date || null, followUpDate: null, waitingOn: null, description: detail.trim() || null, category: null } });
      if (type === "note") await invoke("create_note", { input: { projectId: projectId || null, body: detail } });
      if (type === "rfi") await invoke("create_rfi", { input: { projectId, number: secondary, subject: title, question: detail, recipient: null, status: "draft", createdDate: today(), submittedDate: null, responseDueDate: date || null, responseReceivedDate: null, response: null, notes: null, rfiLocation: null, drawingNumber: null, costImpact: null, timeDelay: null, suggestedSolution: null, requestedBy: null, relatedTaskId: null } });
      if (type === "contact") await invoke("create_contact", { input: { name: title, company: secondary.trim() || null, email: detail.trim() || null, phone: null, role: null } });
      const screen = type === "task" ? "tasks" : type === "rfi" ? "rfis" : "notes";
      reset(); onSaved(screen); onClose();
    } catch (caught) { setError(describeAppError(caught)); } finally { setSaving(false); }
  }
  const needsProject = type === "task" || type === "rfi";
  const valid = type === "note" ? detail.trim() : type === "rfi" ? projectId && title.trim() && secondary.trim() && detail.trim() : type === "task" ? projectId && title.trim() : title.trim();
  const trapFocus = (event: React.KeyboardEvent) => { if(event.key!=="Tab"||!dialogRef.current)return; const controls=Array.from(dialogRef.current.querySelectorAll<HTMLElement>("button:not(:disabled), input:not(:disabled), textarea:not(:disabled), select:not(:disabled)")); const first=controls[0],last=controls[controls.length-1]; if(event.shiftKey&&document.activeElement===first){event.preventDefault();last?.focus()}else if(!event.shiftKey&&document.activeElement===last){event.preventDefault();first?.focus()} };
  if (!open) return null;
  return <div className="palette-backdrop" onMouseDown={onClose}><section ref={dialogRef} className="palette quick-capture" role="dialog" aria-modal="true" aria-labelledby="capture-title" onMouseDown={(event)=>event.stopPropagation()} onKeyDown={trapFocus}>
    <div className="palette-heading"><div><h2 id="capture-title">Quick capture</h2><p>Record the item without leaving your current workspace.</p></div><button type="button" className="quiet" onClick={onClose}>Close</button></div>
    <div className="capture-types" role="tablist" aria-label="Record type">{(["task","note","rfi","contact"] as CaptureType[]).map((value)=><button key={value} type="button" role="tab" aria-selected={type===value} className={type===value?"selected":"secondary"} onClick={()=>changeType(value)}>{value === "rfi" ? "RFI draft" : value[0].toUpperCase()+value.slice(1)}</button>)}</div>
    <form onSubmit={(event)=>void save(event)}>
      {type !== "contact" && <label>Project {type === "note" && <span className="optional">(optional)</span>}<select autoFocus value={projectId} onChange={(event)=>setProjectId(event.target.value)} required={needsProject}><option value="">{needsProject?"Choose project":"General workspace"}</option>{projects.map((project)=><option key={project.id} value={project.id}>{project.number} — {project.name}</option>)}</select></label>}
      {type !== "note" && <label>{type === "contact" ? "Contact name" : type === "rfi" ? "Subject" : "Task title"}<input autoFocus={type==="contact"} value={title} onChange={(event)=>setTitle(event.target.value)} required /></label>}
      {type === "rfi" && <label>RFI number<input value={secondary} onChange={(event)=>setSecondary(event.target.value)} required /></label>}
      {type === "contact" && <label>Company <span className="optional">(optional)</span><input value={secondary} onChange={(event)=>setSecondary(event.target.value)} /></label>}
      <label>{type === "note" ? "Note" : type === "rfi" ? "Question" : type === "contact" ? "Email (optional)" : "Description (optional)"}<textarea value={detail} onChange={(event)=>setDetail(event.target.value)} required={type === "note" || type === "rfi"} rows={type === "contact" ? 2 : 4} /></label>
      {(type === "task" || type === "rfi") && <div className="capture-row">{type === "task" && <label>Priority<select value={priority} onChange={(event)=>setPriority(event.target.value)}><option value="low">Low</option><option value="medium">Medium</option><option value="high">High</option><option value="urgent">Urgent</option></select></label>}<label>{type === "rfi" ? "Response due" : "Due date"} <span className="optional">(optional)</span><input type="date" value={date} onChange={(event)=>setDate(event.target.value)} /></label></div>}
      {error && <p className="status error" role="alert">{error}</p>}<div className="actions"><button type="submit" disabled={saving||!valid}>{saving?"Saving…":`Save ${type === "rfi" ? "RFI draft" : type}`}</button><button type="button" className="secondary" onClick={onClose}>Cancel</button></div>
    </form>
  </section></div>;
}

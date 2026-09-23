import { useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { describeAppError } from "./error";
import { useProjectContext } from "./projectContext";

type Project = { id:string; number:string; name:string; status:string; phase:string; customPhaseName:string|null };
type Task = { id:string; projectId:string; projectNumber:string; projectName:string; title:string; status:string; priority:string; dueDate:string|null; followUpDate:string|null; waitingOn:string|null };
type Rfi = { id:string; projectId:string; number:string; subject:string; status:string; responseDueDate:string|null };
type Submittal = { id:string; projectId:string; number:string; name:string; status:string; submittedDate:string|null };
type Note = { id:string; projectId:string|null; body:string; createdAtUtc:string };

const activeTask = (task:Task) => !["completed","cancelled"].includes(task.status);
const label = (value:string) => value.replace(/_/g," ").replace(/\b\w/g,(letter:string)=>letter.toUpperCase());
const localToday = () => { const date=new Date(); return `${date.getFullYear()}-${String(date.getMonth()+1).padStart(2,"0")}-${String(date.getDate()).padStart(2,"0")}` };

export function Overview({ onNavigate }: { onNavigate:(screen:string)=>void }) {
  const context = useProjectContext();
  const [projects,setProjects]=useState<Project[]>([]); const [tasks,setTasks]=useState<Task[]>([]); const [rfis,setRfis]=useState<Rfi[]>([]); const [submittals,setSubmittals]=useState<Submittal[]>([]); const [notes,setNotes]=useState<Note[]>([]);
  const [loading,setLoading]=useState(true); const [error,setError]=useState("");
  useEffect(()=>{setLoading(true);void Promise.all([invoke<Project[]>("list_projects",{includeArchived:false}),invoke<Task[]>("list_tasks"),invoke<Rfi[]>("list_rfis"),invoke<Submittal[]>("list_submittals"),invoke<Note[]>("list_notes")]).then(([p,t,r,s,n])=>{setProjects(p);setTasks(t);setRfis(r);setSubmittals(s);setNotes(n);setError("")}).catch((caught)=>setError(describeAppError(caught))).finally(()=>setLoading(false))},[]);
  const scope = <T extends {projectId:string}>(items:T[]) => context ? items.filter((item)=>item.projectId===context) : items;
  const scopedTasks=useMemo(()=>scope(tasks),[tasks,context]);
  const scopedRfis=useMemo(()=>scope(rfis),[rfis,context]);
  const scopedSubmittals=useMemo(()=>scope(submittals),[submittals,context]);
  const scopedNotes=useMemo(()=>notes.filter((note)=>context?note.projectId===context:Boolean(note.projectId)),[notes,context]);
  const today=localToday();
  const dueTasks=scopedTasks.filter((task)=>activeTask(task)&&((task.dueDate&&task.dueDate<=today)||(task.followUpDate&&task.followUpDate<=today))).sort((a,b)=>(a.dueDate??a.followUpDate??"").localeCompare(b.dueDate??b.followUpDate??""));
  const openRfis=scopedRfis.filter((rfi)=>["draft","open"].includes(rfi.status));
  const pendingSubmittals=scopedSubmittals.filter((item)=>["submitted","under_review","revise_and_resubmit"].includes(item.status));
  const selected=projects.find((project)=>project.id===context);
  const scopeName=selected?`${selected.number} — ${selected.name}`:"All active projects";
  if(loading)return <section className="projects-page"><p className="empty-state" role="status">Loading project overview…</p></section>;
  return <section className="projects-page overview-page" aria-labelledby="overview-title">
    <div className="page-toolbar"><div><p className="eyebrow">Overview</p><h1 id="overview-title">{scopeName}</h1>{selected&&<p className="page-context">{label(selected.status)} · {selected.phase==="custom"?selected.customPhaseName:label(selected.phase)}</p>}</div></div>
    {error&&<p className="status error" role="alert">{error}</p>}
    <div className="overview-summary" aria-label="Project workload summary"><button type="button" onClick={()=>onNavigate("tasks")}><strong>{scopedTasks.filter(activeTask).length}</strong><span>Active tasks</span></button><button type="button" onClick={()=>onNavigate("attention")}><strong>{dueTasks.length}</strong><span>Due or follow-up</span></button><button type="button" onClick={()=>onNavigate("rfis")}><strong>{openRfis.length}</strong><span>Open RFIs</span></button><button type="button" onClick={()=>onNavigate("submittals")}><strong>{pendingSubmittals.length}</strong><span>Pending submittals</span></button></div>
    <div className="overview-columns"><section className="overview-register"><div className="detail-heading"><h2>Immediate work</h2><button type="button" className="quiet" onClick={()=>onNavigate("attention")}>Open Attention</button></div>{dueTasks.length?<table><thead><tr><th>Task</th><th>Project</th><th>Due</th><th>Status</th></tr></thead><tbody>{dueTasks.slice(0,8).map((task)=><tr key={task.id}><td>{task.title}</td><td>{task.projectNumber} — {task.projectName}</td><td>{task.dueDate??task.followUpDate}</td><td>{label(task.status)}</td></tr>)}</tbody></table>:<p className="empty-state">No overdue, due-today, or follow-up work in this context.</p>}</section>
    <section className="overview-register"><div className="detail-heading"><h2>Open responses</h2><div className="actions"><button type="button" className="quiet" onClick={()=>onNavigate("rfis")}>RFIs</button><button type="button" className="quiet" onClick={()=>onNavigate("submittals")}>Submittals</button></div></div>{openRfis.length||pendingSubmittals.length?<ul className="overview-list">{openRfis.slice(0,5).map((rfi)=><li key={rfi.id}><strong>{rfi.number} — {rfi.subject}</strong><span>RFI · {label(rfi.status)}{rfi.responseDueDate?` · Due ${rfi.responseDueDate}`:""}</span></li>)}{pendingSubmittals.slice(0,5).map((item)=><li key={item.id}><strong>{item.number} — {item.name}</strong><span>Submittal · {label(item.status)}{item.submittedDate?` · Submitted ${item.submittedDate}`:""}</span></li>)}</ul>:<p className="empty-state">No RFIs or submittals currently require a response.</p>}</section>
    <section className="overview-register overview-notes"><div className="detail-heading"><h2>Recent project notes</h2><button type="button" className="quiet" onClick={()=>onNavigate("notes")}>Open notes</button></div>{scopedNotes.length?<ul className="overview-list">{scopedNotes.slice(0,6).map((note)=><li key={note.id}><strong>{note.body}</strong><span>{new Date(note.createdAtUtc).toLocaleString()}</span></li>)}</ul>:<p className="empty-state">No notes have been recorded in this context.</p>}</section></div>
  </section>;
}

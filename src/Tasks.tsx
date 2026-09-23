import { Fragment, useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { describeAppError } from "./error";
import { takeFocus } from "./focus";
import { useProjectContext } from "./projectContext";
import { summarizeTaskProgress } from "./taskProgress";
import { createTaskView, readTaskViews, writeTaskViews, type SavedTaskView } from "./savedViews";

type Project = { id: string; number: string; name: string };
type Task = { id: string; projectId: string; projectNumber: string; projectName: string; title: string; priority: string; status: string; dueDate: string | null; followUpDate: string | null; waitingOn: string | null };
type AttentionSection = { key: string; label: string; tasks: Task[] };
type RfiAttention = { id: string; projectId: string; projectNumber: string; projectName: string; number: string; subject: string; recipient: string | null; responseDueDate: string | null; status: string };
type SubmittalAttention = { id: string; projectId: string; projectNumber: string; projectName: string; number: string; name: string; recipient: string | null; submittedDate: string | null; status: string };
const statuses = ["open", "in_progress", "waiting", "blocked", "completed", "cancelled"];
const priorities = ["low", "medium", "high", "urgent"];
const labels: Record<string, string> = { open: "Open", in_progress: "In Progress", waiting: "Waiting", blocked: "Blocked", completed: "Completed", cancelled: "Cancelled", low: "Low", medium: "Medium", high: "High", urgent: "Urgent" };
function localDate(date: Date) { return date.getFullYear() + "-" + String(date.getMonth() + 1).padStart(2, "0") + "-" + String(date.getDate()).padStart(2, "0"); }
function dateWindow() { const today = new Date(); const through = new Date(today.getFullYear(), today.getMonth(), today.getDate()); through.setDate(through.getDate() + 14); return { today: localDate(today), through: localDate(through) }; }

export function Tasks({ attention = false, onOpenRfis, onOpenSubmittals }: { attention?: boolean; onOpenRfis?: () => void; onOpenSubmittals?: () => void }) {
  const projectContext = useProjectContext();
  const [tasks, setTasks] = useState<Task[]>([]); const [allTasks, setAllTasks] = useState<Task[]>([]); const [sections, setSections] = useState<AttentionSection[]>([]); const [projects, setProjects] = useState<Project[]>([]);
  const [title, setTitle] = useState(""); const [projectId, setProjectId] = useState(""); const [priority, setPriority] = useState("medium"); const [dueDate, setDueDate] = useState(""); const [description, setDescription] = useState(""); const [category, setCategory] = useState(""); const [createStatus, setCreateStatus] = useState("open"); const [createWaitingOn, setCreateWaitingOn] = useState(""); const [createFollowUpDate, setCreateFollowUpDate] = useState("");
  const [query, setQuery] = useState(""); const [projectFilter, setProjectFilter] = useState(""); const [statusFilter, setStatusFilter] = useState(""); const [priorityFilter, setPriorityFilter] = useState("");
  const [sortBy, setSortBy] = useState("due");
  const [savedViews, setSavedViews] = useState<SavedTaskView[]>(() => readTaskViews());
  const [selectedView, setSelectedView] = useState("");
  const [viewName, setViewName] = useState("");
  const [namingView, setNamingView] = useState(false);
  const [focusedTaskId, setFocusedTaskId] = useState<string | null>(null);
  const [showCreate, setShowCreate] = useState(false);
  const [error, setError] = useState(""); const [loading, setLoading] = useState(true); const [saving, setSaving] = useState(false);
  const [waitingTask, setWaitingTask] = useState<Task | null>(null); const [waitingOn, setWaitingOn] = useState(""); const [followUpDate, setFollowUpDate] = useState("");
  const [rfiAttention, setRfiAttention] = useState<RfiAttention[]>([]);
  const [submittalAttention, setSubmittalAttention] = useState<SubmittalAttention[]>([]);
  useEffect(()=>{if(attention)return;try{const v=JSON.parse(localStorage.getItem("tasks.filters")??"null");if(v){setQuery(v.query??"");setProjectFilter(v.project??"");setStatusFilter(v.status??"");setPriorityFilter(v.priority??"")}}catch{/* ignore invalid local preference */}},[attention]);useEffect(()=>{if(!attention)localStorage.setItem("tasks.filters",JSON.stringify({query,project:projectFilter,status:statusFilter,priority:priorityFilter}))},[attention,query,projectFilter,statusFilter,priorityFilter]);
  useEffect(()=>{if(!selectedView)return;const view=savedViews.find((item)=>item.id===selectedView);if(!view||view.query!==query||view.project!==projectFilter||view.status!==statusFilter||view.priority!==priorityFilter||view.sort!==sortBy)setSelectedView("")},[query,projectFilter,statusFilter,priorityFilter,sortBy,selectedView,savedViews]);
  useEffect(()=>setProjectFilter(projectContext),[projectContext]);
  useEffect(()=>{const handler=()=>{if(!attention){setShowCreate(true);setTimeout(()=>(document.querySelector(".create-panel input") as HTMLInputElement|null)?.focus(),0)}};window.addEventListener("workspace:new",handler);return()=>window.removeEventListener("workspace:new",handler)},[attention]);
  useEffect(()=>{if(attention||!tasks.length)return;const id=takeFocus("tasks");const focused=tasks.find(task=>task.id===id);if(focused){setFocusedTaskId(focused.id);setQuery(focused.title)}},[tasks,attention]);

  async function load() {
    setLoading(true);
    try {
      const projectList = await invoke<Project[]>("list_projects", { includeArchived: false }); setProjects(projectList);
      if (attention) { const range = dateWindow(); const [result, rfiResult, submittalResult, taskList] = await Promise.all([invoke<AttentionSection[]>("list_attention", range), invoke<RfiAttention[]>("list_rfi_attention", range), invoke<SubmittalAttention[]>("list_submittal_attention"), invoke<Task[]>("list_tasks")]); setSections(result); setTasks(result.flatMap((section) => section.tasks)); setAllTasks(taskList); setRfiAttention(rfiResult); setSubmittalAttention(submittalResult); }
      else { const taskList=await invoke<Task[]>("list_tasks"); setTasks(taskList); setAllTasks(taskList); }
      setError("");
    } catch (caught) { setError(describeAppError(caught)); } finally { setLoading(false); }
  }
  useEffect(() => { void load(); }, [attention]);

  async function addTask(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault(); if (!title.trim() || !projectId) return; setSaving(true);
    try {
      await invoke("create_task", { input: { projectId, title, priority, status: createStatus, dueDate: dueDate || null, followUpDate: createStatus === "waiting" ? createFollowUpDate || null : null, waitingOn: createStatus === "waiting" ? createWaitingOn.trim() : null, description: description.trim() || null, category: category.trim() || null } });
      setTitle(""); setDueDate(""); setDescription(""); setCategory(""); setCreateStatus("open"); setCreateWaitingOn(""); setCreateFollowUpDate(""); setShowCreate(false); await load();
    } catch (caught) { setError(describeAppError(caught)); } finally { setSaving(false); }
  }
  async function saveStatus(task: Task, status: string, waiting: string | null = null, followUp: string | null = null) {
    setSaving(true);
    try { await invoke("set_task_status", { id: task.id, status, waitingOn: waiting, followUpDate: followUp }); setWaitingTask(null); setError(""); await load(); }
    catch (caught) { setError(describeAppError(caught)); } finally { setSaving(false); }
  }
  function changeStatus(task: Task, status: string) {
    if (status === "waiting") { setWaitingTask(task); setWaitingOn(task.waitingOn ?? ""); setFollowUpDate(task.followUpDate ?? ""); }
    else void saveStatus(task, status);
  }
  function applySavedView(id: string) {
    setSelectedView(id);
    const view = savedViews.find((item) => item.id === id);
    if (!view) return;
    setFocusedTaskId(null); setQuery(view.query); setProjectFilter(view.project); setStatusFilter(view.status); setPriorityFilter(view.priority); setSortBy(view.sort);
  }
  function saveView(event: React.FormEvent) {
    event.preventDefault();
    if (!viewName.trim()) return;
    const view = createTaskView(viewName, { query, project: projectFilter, status: statusFilter, priority: priorityFilter, sort: sortBy });
    const next = [...savedViews, view]; writeTaskViews(next); setSavedViews(next); setSelectedView(view.id); setViewName(""); setNamingView(false);
  }
  function deleteView() {
    if (!selectedView) return;
    const next = savedViews.filter((item) => item.id !== selectedView); writeTaskViews(next); setSavedViews(next); setSelectedView("");
  }
  const visibleTasks = useMemo(() => tasks.filter((task) => {
    if (focusedTaskId) return task.id === focusedTaskId;
    const needle = query.trim().toLocaleLowerCase();
    return (!needle || (task.title + " " + task.projectNumber + " " + task.projectName).toLocaleLowerCase().includes(needle)) && (!projectFilter || task.projectId === projectFilter) && (!statusFilter || task.status === statusFilter) && (!priorityFilter || task.priority === priorityFilter);
  }).sort((a,b)=>sortBy==="title"?a.title.localeCompare(b.title):sortBy==="priority"?["urgent","high","medium","low"].indexOf(a.priority)-["urgent","high","medium","low"].indexOf(b.priority):(a.dueDate??"9999-12-31").localeCompare(b.dueDate??"9999-12-31")), [tasks, query, projectFilter, statusFilter, priorityFilter, focusedTaskId, sortBy]);
  const visibleSections = useMemo(() => sections.map((section) => ({ ...section, tasks: section.tasks.filter((task) => visibleTasks.some((visible) => visible.id === task.id)) })), [sections, visibleTasks]);
  const progressTasks = useMemo(() => allTasks.filter((task) => !projectFilter || task.projectId === projectFilter), [allTasks, projectFilter]);
  const progressScope = projectFilter ? projects.find((project) => project.id === projectFilter)?.name ?? "Selected project" : "All projects";
  const visibleRfiAttention = useMemo(() => rfiAttention.filter((item) => !projectFilter || item.projectId === projectFilter), [rfiAttention, projectFilter]);
  const visibleSubmittalAttention = useMemo(() => submittalAttention.filter((item) => !projectFilter || item.projectId === projectFilter), [submittalAttention, projectFilter]);
  const tableProps = { onStatusChange: changeStatus, waitingTask, waitingOn, followUpDate, setWaitingOn, setFollowUpDate, onSaveWaiting: () => waitingTask && void saveStatus(waitingTask, "waiting", waitingOn, followUpDate || null), onCancelWaiting: () => setWaitingTask(null), saving };

  return <section className="projects-page" aria-labelledby="tasks-title">
    <div className="page-toolbar"><div><p className="eyebrow">{attention ? "Attention" : "Tasks"}</p><h1 id="tasks-title">{attention ? "What needs attention" : "Tasks"}</h1></div>{!attention&&<button type="button" onClick={()=>setShowCreate(value=>!value)}>{showCreate?"Close create":"New task"}</button>}</div>
    {(!loading || allTasks.length > 0) && <TaskProgressGauge tasks={progressTasks} scope={progressScope} loading={loading} />}
    {!attention && showCreate && <form className="create-panel" onSubmit={(event) => void addTask(event)}>
      <h2>Quick task</h2><div className="form-grid task-create-grid">
        <label>Task title<input autoComplete="off" value={title} onChange={(event) => setTitle(event.target.value)} required /></label>
        <label>Project<select value={projectId} onChange={(event) => setProjectId(event.target.value)} required><option value="">Choose project</option>{projects.map((project) => <option value={project.id} key={project.id}>{project.number} — {project.name}</option>)}</select></label>
        <label>Priority<select value={priority} onChange={(event) => setPriority(event.target.value)}>{priorities.map((item) => <option value={item} key={item}>{labels[item]}</option>)}</select></label>
        <label>Status<select value={createStatus} onChange={(event) => setCreateStatus(event.target.value)}>{statuses.map((status) => <option value={status} key={status}>{labels[status]}</option>)}</select></label>
        <label>Due date<input type="date" value={dueDate} onChange={(event) => setDueDate(event.target.value)} /></label>
        {createStatus === "waiting" && <><label>Waiting on<input value={createWaitingOn} onChange={(event) => setCreateWaitingOn(event.target.value)} maxLength={160} required /></label><label>Follow-up date <span className="optional">(optional)</span><input type="date" value={createFollowUpDate} onChange={(event) => setCreateFollowUpDate(event.target.value)} /></label></>}
      </div><details className="optional-details"><summary>Additional details</summary><div className="form-grid"><label>Category <span className="optional">(optional)</span><input value={category} onChange={(event) => setCategory(event.target.value)} maxLength={80} /></label><label className="task-description">Description <span className="optional">(optional)</span><input value={description} onChange={(event) => setDescription(event.target.value)} maxLength={1000} /></label></div></details><div className="actions"><button type="submit" disabled={saving || !title.trim() || !projectId || (createStatus === "waiting" && !createWaitingOn.trim())}>{saving ? "Saving…" : "Add task"}</button><button type="button" className="secondary" onClick={()=>setShowCreate(false)}>Cancel</button></div>
    </form>}
    {!attention && <div className="saved-view-bar" aria-label="Saved task views"><label>Saved view<select value={selectedView} onChange={(event)=>applySavedView(event.target.value)}><option value="">Current filters</option>{savedViews.map((view)=><option key={view.id} value={view.id}>{view.name}</option>)}</select></label>{namingView?<form onSubmit={saveView}><label>View name<input autoFocus value={viewName} onChange={(event)=>setViewName(event.target.value)} maxLength={60} required /></label><button type="submit" disabled={!viewName.trim()}>Save</button><button type="button" className="quiet" onClick={()=>{setNamingView(false);setViewName("")}}>Cancel</button></form>:<button type="button" className="secondary" onClick={()=>setNamingView(true)}>Save current view</button>}{selectedView&&<button type="button" className="quiet destructive" onClick={deleteView}>Delete saved view</button>}</div>}
    <div className="list-tools task-filters" aria-label="Task filters">
      <label className="search-filter">Search<input type="search" value={query} onChange={(event) => { setFocusedTaskId(null); setQuery(event.target.value); }} placeholder="Task or project" /></label>
      <label>Project<select value={projectFilter} onChange={(event) => setProjectFilter(event.target.value)}><option value="">All projects</option>{projects.map((project) => <option key={project.id} value={project.id}>{project.number} — {project.name}</option>)}</select></label>
      <label>Status<select value={statusFilter} onChange={(event) => setStatusFilter(event.target.value)}><option value="">All statuses</option>{statuses.map((status) => <option key={status} value={status}>{labels[status]}</option>)}</select></label>
      <label>Priority<select value={priorityFilter} onChange={(event) => setPriorityFilter(event.target.value)}><option value="">All priorities</option>{priorities.map((item) => <option key={item} value={item}>{labels[item]}</option>)}</select></label>
      <label>Sort<select value={sortBy} onChange={(event)=>setSortBy(event.target.value)}><option value="due">Due date</option><option value="priority">Priority</option><option value="title">Task title</option></select></label>
      {(query || projectFilter || statusFilter || priorityFilter) && <button type="button" className="quiet clear-filters" onClick={() => { setFocusedTaskId(null); setSelectedView(""); setQuery(""); setProjectFilter(""); setStatusFilter(""); setPriorityFilter(""); }}>Clear filters</button>}
    </div>
    {error && <p className="status error" role="alert">{error}</p>}
    {loading ? <p className="empty-state" role="status">Loading tasks…</p> : attention ? <div className="attention-sections">{visibleSections.map((section) => <section key={section.key} className="attention-section"><h2 className="group-heading">{section.label}<span>{section.tasks.length}</span></h2><TaskTable tasks={section.tasks} {...tableProps} /></section>)}<section className="attention-section"><div className="detail-heading"><h2 className="group-heading">RFIs awaiting response<span>{visibleRfiAttention.length}</span></h2>{onOpenRfis && <button type="button" className="secondary" onClick={onOpenRfis}>Open RFIs</button>}</div>{visibleRfiAttention.length ? <div className="table-scroll"><table><thead><tr><th>RFI</th><th>Project</th><th>Recipient</th><th>Response due</th></tr></thead><tbody>{visibleRfiAttention.map((rfi) => <tr key={rfi.id}><td><strong>{rfi.number}</strong><span className="task-subline">{rfi.subject}</span></td><td>{rfi.projectNumber} — {rfi.projectName}</td><td>{rfi.recipient ?? "—"}</td><td>{rfi.responseDueDate ?? "No due date"}</td></tr>)}</tbody></table></div> : <p className="empty-state">No open RFIs are due in this Attention window.</p>}</section><section className="attention-section"><div className="detail-heading"><h2 className="group-heading">Submittals awaiting disposition<span>{visibleSubmittalAttention.length}</span></h2>{onOpenSubmittals && <button type="button" className="secondary" onClick={onOpenSubmittals}>Open submittals</button>}</div>{visibleSubmittalAttention.length ? <div className="table-scroll"><table><thead><tr><th>Submittal</th><th>Project</th><th>Recipient</th><th>Submitted</th></tr></thead><tbody>{visibleSubmittalAttention.map((item) => <tr key={item.id}><td><strong>{item.number}</strong><span className="task-subline">{item.name}</span></td><td>{item.projectNumber} — {item.projectName}</td><td>{item.recipient ?? "—"}</td><td>{item.submittedDate ?? "No submitted date"}</td></tr>)}</tbody></table></div> : <p className="empty-state">No submittals are awaiting disposition.</p>}</section></div> : <TaskTable tasks={visibleTasks} {...tableProps} />}
  </section>;
}

function TaskProgressGauge({ tasks, scope, loading }: { tasks: Task[]; scope: string; loading: boolean }) {
  const progress = summarizeTaskProgress(tasks);
  const statuses = [
    ["In progress", progress.inProgress, "in-progress"],
    ["Open", progress.open, "open"],
    ["Waiting", progress.waiting, "waiting"],
    ["Blocked", progress.blocked, "blocked"],
  ] as const;
  return <section className="task-progress" aria-label={`Task completion for ${scope}`} aria-busy={loading}>
    <div className="task-progress-heading"><div><h2>Task completion</h2><p>{scope} · Cancelled tasks excluded</p></div><strong>{progress.percent}%</strong></div>
    <progress value={progress.completed} max={progress.total || 1} aria-label={`${progress.completed} of ${progress.total} tasks completed`} />
    <div className="task-progress-detail"><p><strong>{progress.completed}</strong> of <strong>{progress.total}</strong> completed</p>{progress.total > 0 ? <ul>{statuses.map(([label, count, key]) => <li key={key} className={`progress-${key}`}><i aria-hidden="true"/><span>{label}</span><strong>{count}</strong></li>)}</ul> : <span>No active or completed tasks in this scope.</span>}</div>
  </section>;
}

type TableProps = { tasks: Task[]; onStatusChange: (task: Task, status: string) => void; waitingTask: Task | null; waitingOn: string; followUpDate: string; setWaitingOn: (value: string) => void; setFollowUpDate: (value: string) => void; onSaveWaiting: () => void; onCancelWaiting: () => void; saving: boolean };
function TaskTable({ tasks, onStatusChange, waitingTask, waitingOn, followUpDate, setWaitingOn, setFollowUpDate, onSaveWaiting, onCancelWaiting, saving }: TableProps) {
  if (!tasks.length) return <p className="empty-state">No tasks in this view.</p>;
  return <div className="table-scroll"><table><thead><tr><th>Task</th><th>Project</th><th>Priority</th><th>Due / follow-up</th><th>Status</th></tr></thead><tbody>{tasks.map((task) => <Fragment key={task.id}>
    <tr><td>{task.title}{task.waitingOn && task.status === "waiting" && <span className="task-subline">Waiting on {task.waitingOn}</span>}</td><td>{task.projectNumber} — {task.projectName}</td>
    <td><span className={"priority-text priority-" + task.priority}>{labels[task.priority]}</span></td><td>{task.status === "waiting" ? task.followUpDate ? "Follow up " + task.followUpDate : "No follow-up date" : task.dueDate ?? "—"}</td>
    <td><select aria-label={"Status for " + task.title} value={waitingTask?.id === task.id ? "waiting" : task.status} onChange={(event) => onStatusChange(task, event.target.value)} disabled={saving}>{statuses.map((status) => <option key={status} value={status}>{labels[status]}</option>)}</select></td></tr>
    {waitingTask?.id === task.id && <tr key={task.id + "-waiting"}><td colSpan={5} className="waiting-editor-cell"><div className="waiting-editor"><label>Waiting on<input autoFocus value={waitingOn} onChange={(event) => setWaitingOn(event.target.value)} maxLength={160} required /></label><label>Follow-up date <span className="optional">(optional)</span><input type="date" value={followUpDate} onChange={(event) => setFollowUpDate(event.target.value)} /></label><button type="button" onClick={onSaveWaiting} disabled={saving || !waitingOn.trim()}>Save Waiting</button><button type="button" className="secondary" onClick={onCancelWaiting}>Cancel</button></div></td></tr>}
  </Fragment>)}</tbody></table></div>;
}

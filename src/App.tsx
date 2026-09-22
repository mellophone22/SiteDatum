import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import "./App.css";
import { describeAppError } from "./error";
import { Projects } from "./Projects";
import { Tasks } from "./Tasks";
import { Rfis } from "./Rfis";
import { Submittals } from "./Submittals";
import { Files } from "./Files";
import { NotesContacts } from "./NotesContacts";
import { SearchPalette } from "./SearchPalette";

type ProjectRootSetting = { path: string | null };
type ProjectRootValidation = { canonicalPath: string; pathKind: "local" | "unc"; warning: string | null };
type Screen = "projects" | "tasks" | "rfis" | "submittals" | "files" | "notes" | "attention" | "settings";
type ProjectSummary = { id: string; number: string; name: string };

function App() {
  const [path, setPath] = useState("");
  const [savedPath, setSavedPath] = useState<string | null>(null);
  const [validation, setValidation] = useState<ProjectRootValidation | null>(null);
  const [message, setMessage] = useState("Loading saved setting…");
  const [error, setError] = useState("");
  const [working, setWorking] = useState(false);
  const [screen, setScreen] = useState<Screen>(() => (localStorage.getItem("workspace.screen") as Screen | null) || "attention");
  const [paletteOpen, setPaletteOpen] = useState(false);
  const [navigationRevision, setNavigationRevision] = useState(0);
  const searchButtonRef = useRef<HTMLButtonElement>(null);
  const [projectContext, setProjectContext] = useState(() => localStorage.getItem("workspace.projectContext") ?? "");
  const [projectOptions, setProjectOptions] = useState<ProjectSummary[]>([]);

  useEffect(() => {
    void loadSetting();
    void invoke<ProjectSummary[]>("list_projects", { includeArchived: false }).then(setProjectOptions).catch(() => setProjectOptions([]));
  }, []);
  useEffect(() => { localStorage.setItem("workspace.screen", screen); }, [screen]);
  useEffect(() => { const handler=(event:KeyboardEvent)=>{if(event.ctrlKey&&event.key.toLowerCase()==="k"){event.preventDefault();setPaletteOpen(true)}if(event.ctrlKey&&event.key.toLowerCase()==="n"){event.preventDefault();window.dispatchEvent(new CustomEvent("workspace:new"))}if(event.key==="Escape")setPaletteOpen(false)};window.addEventListener("keydown",handler);return()=>window.removeEventListener("keydown",handler)},[]);
  useEffect(() => {
    const listKeys = (event: KeyboardEvent) => {
      if (!["ArrowDown", "ArrowUp", "Home", "End"].includes(event.key)) return;
      const active = document.activeElement as HTMLElement | null;
      if (!active?.classList.contains("project-link")) return;
      const table = active.closest("table");
      const links = table ? Array.from(table.querySelectorAll<HTMLButtonElement>("tbody .project-link")) : [];
      const index = links.indexOf(active as HTMLButtonElement);
      if (index < 0) return;
      const next = event.key === "Home" ? 0 : event.key === "End" ? links.length - 1 : event.key === "ArrowDown" ? Math.min(index + 1, links.length - 1) : Math.max(index - 1, 0);
      event.preventDefault(); links[next]?.focus();
    };
    const rowContext = (event: MouseEvent) => {
      const target = event.target as HTMLElement;
      const action = target.closest("tr")?.querySelector<HTMLButtonElement>(".project-link");
      if (!action) return;
      event.preventDefault(); action.click();
    };
    window.addEventListener("keydown", listKeys);
    window.addEventListener("contextmenu", rowContext);
    return () => { window.removeEventListener("keydown", listKeys); window.removeEventListener("contextmenu", rowContext); };
  }, []);

  async function loadSetting() {
    try {
      const setting = await invoke<ProjectRootSetting>("get_project_root");
      setPath(setting.path ?? "");
      setSavedPath(setting.path);
      setMessage(setting.path ? "Saved project root loaded." : "Choose the folder where project workspaces will be created.");
    } catch (caught) {
      setError(describeAppError(caught));
      setMessage("");
    }
  }

  async function checkLocation() {
    setWorking(true);
    setError("");
    setValidation(null);
    try {
      const result = await invoke<ProjectRootValidation>("validate_project_root_command", { path });
      setValidation(result);
      setMessage(`${result.pathKind === "unc" ? "UNC share" : "Local folder"} is available.`);
    } catch (caught) {
      setError(describeAppError(caught));
      setMessage("");
    } finally {
      setWorking(false);
    }
  }

  async function browseForFolder() {
    const selected = await open({ directory: true, multiple: false, title: "Select project root" });
    if (typeof selected === "string") {
      setPath(selected);
      setValidation(null);
      setMessage("Folder selected. Check the location before saving.");
      setError("");
    }
  }

  async function save() {
    setWorking(true);
    setError("");
    try {
      const setting = await invoke<ProjectRootSetting>("save_project_root", { path });
      setPath(setting.path ?? path);
      setSavedPath(setting.path);
      setMessage("Project root saved and ready for project workspaces.");
    } catch (caught) {
      setError(describeAppError(caught));
      setMessage("");
    } finally {
      setWorking(false);
    }
  }
  async function backup() { setWorking(true); setError(""); try { const saved=await invoke<string>("create_local_backup"); setMessage(`Local backup created: ${saved}`); } catch (caught) { setError(describeAppError(caught)); } finally { setWorking(false); } }
  function changeProjectContext(value: string) { setProjectContext(value); localStorage.setItem("workspace.projectContext", value); window.dispatchEvent(new CustomEvent("workspace:project", { detail: value })); }
  function closePalette() { setPaletteOpen(false); requestAnimationFrame(() => searchButtonRef.current?.focus()); }
  const navigate = (destination: Screen) => setScreen(destination);
  const navButton = (destination: Screen, label: string) => <button type="button" className={`nav-button${screen === destination ? " active" : ""}`} aria-current={screen === destination ? "page" : undefined} onClick={() => navigate(destination)}>{label}</button>;

  return (
    <main className="application-shell">
      <header className="app-header">
        <div className="brand-row"><span className="app-name">Project Engineer Workspace</span><div className="utility-nav"><button ref={searchButtonRef} type="button" className="quiet" onClick={()=>setPaletteOpen(true)}>Search <kbd>Ctrl+K</kbd></button>{navButton("settings", "Settings")}</div></div>
        <nav className="main-navigation" aria-label="Main navigation"><div className="primary-nav">{navButton("attention", "Attention")}{navButton("projects", "Projects")}</div><div className="workspace-nav" aria-label="Workspace registers"><label className="project-context"><span>Project</span><select value={projectContext} onChange={(event) => changeProjectContext(event.target.value)}><option value="">All projects</option>{projectOptions.map((project) => <option key={project.id} value={project.id}>{project.number} — {project.name}</option>)}</select></label>{navButton("tasks", "Tasks")}{navButton("rfis", "RFIs")}{navButton("submittals", "Submittals")}{navButton("files", "Files")}{navButton("notes", "Notes & Contacts")}</div></nav>
      </header>
      <SearchPalette open={paletteOpen} onClose={closePalette} onNavigate={(value)=>{setScreen(value as typeof screen);setNavigationRevision((current)=>current+1)}} />
      {screen === "projects" ? <Projects key={navigationRevision} onOpenSettings={()=>setScreen("settings")} /> : screen === "tasks" ? <Tasks key={navigationRevision} /> : screen === "rfis" ? <Rfis key={navigationRevision} /> : screen === "submittals" ? <Submittals key={navigationRevision} /> : screen === "files" ? <Files key={navigationRevision} /> : screen === "notes" ? <NotesContacts key={navigationRevision} /> : screen === "attention" ? <Tasks key={navigationRevision} attention onOpenRfis={() => setScreen("rfis")} onOpenSubmittals={() => setScreen("submittals")} /> : <section className="settings" aria-labelledby="settings-title">
        <div className="section-heading"><div><p className="eyebrow">Settings</p><h1 id="settings-title">Project root</h1></div></div>
        <p className="intro">Select an existing local folder or UNC share. The app stores its data separately in local application storage.</p>
        <div className="form-row">
          <label htmlFor="project-root">Existing folder path</label>
          <div className="path-control">
            <input id="project-root" value={path} onChange={(event) => { setPath(event.target.value); setValidation(null); }} placeholder="C:\\Projects or \\server\\share\\Projects" spellCheck="false" aria-describedby="root-help root-status" />
            <button type="button" className="secondary" onClick={() => void browseForFolder()} disabled={working}>Browse…</button>
          </div>
          <p id="root-help" className="help">The folder must already exist and be readable. Reparse points are not accepted.</p>
        </div>
        <div className="actions"><button type="button" className="secondary" onClick={() => void checkLocation()} disabled={working || !path.trim()}>Check location</button><button type="button" onClick={() => void save()} disabled={working || !path.trim()}>Save project root</button><button type="button" className="secondary" onClick={() => void backup()} disabled={working}>Create local backup</button></div>
        <div id="root-status" className="status-area" aria-live="polite">
          {message && <p className="status success">{message}</p>}
          {validation?.warning && <p className="status warning">{validation.warning}</p>}
          {error && <p className="status error" role="alert">{error}</p>}
          {savedPath && <p className="saved-path"><span>Saved path</span>{savedPath}</p>}
        </div>
      </section>}
    </main>
  );
}

export default App;

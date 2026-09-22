import { useEffect, useRef, useState, type ReactNode } from "react";
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
import brandLogo from "./assets/anydesk-logo-transparent.png";
import appIcon from "./assets/anydesk-app-icon.jpg";

type ProjectRootSetting = { path: string | null };
type ProjectRootValidation = { canonicalPath: string; pathKind: "local" | "unc"; warning: string | null };
type Screen = "projects" | "tasks" | "rfis" | "submittals" | "files" | "notes" | "attention" | "settings";
type ProjectSummary = { id: string; number: string; name: string };

const navIcons: Record<Screen, ReactNode> = {
  attention: <><path d="M12 3.5 3.7 18h16.6L12 3.5Z"/><path d="M12 9v4.5M12 16.5h.01"/></>,
  projects: <><path d="M3.5 6.5h6l1.7 2h9.3v10H3.5z"/><path d="M3.5 8.5h17"/></>,
  tasks: <><path d="m4.5 7 1.8 1.8 3.2-3.3M11 7h8M4.5 13l1.8 1.8 3.2-3.3M11 13h8M4.5 19l1.8 1.8 3.2-3.3M11 19h8"/></>,
  rfis: <><path d="M5 4h14v16H5z"/><path d="M8 8h8M8 12h8M8 16h5"/></>,
  submittals: <><path d="M12 3v12M7.5 7.5 12 3l4.5 4.5"/><path d="M4 14v6h16v-6"/></>,
  files: <><path d="M6 3.5h8l4 4V20.5H6z"/><path d="M14 3.5v4h4M9 12h6M9 16h6"/></>,
  notes: <><path d="M4 4h16v13H8l-4 3z"/><path d="M8 8h8M8 12h6"/></>,
  settings: <><circle cx="12" cy="12" r="3"/><path d="M19.4 15a1.7 1.7 0 0 0 .34 1.88l.06.06-2.12 2.12-.06-.06a1.7 1.7 0 0 0-1.88-.34 1.7 1.7 0 0 0-1 1.55V20h-3v-.09a1.7 1.7 0 0 0-1.1-1.55 1.7 1.7 0 0 0-1.88.34l-.06.06-2.12-2.12.06-.06A1.7 1.7 0 0 0 7 14.7a1.7 1.7 0 0 0-1.55-1H5v-3h.45A1.7 1.7 0 0 0 7 9.6a1.7 1.7 0 0 0-.34-1.88l-.06-.06 2.12-2.12.06.06a1.7 1.7 0 0 0 1.88.34 1.7 1.7 0 0 0 1-1.55V4h3v.39a1.7 1.7 0 0 0 1 1.55 1.7 1.7 0 0 0 1.88-.34l.06-.06 2.12 2.12-.06.06a1.7 1.7 0 0 0-.34 1.88 1.7 1.7 0 0 0 1.55 1H21v3h-.05A1.7 1.7 0 0 0 19.4 15Z"/></>,
};

function NavIcon({ screen }: { screen: Screen }) {
  return <svg className="nav-icon" viewBox="0 0 24 24" aria-hidden="true" fill="none" stroke="currentColor" strokeWidth="1.7" strokeLinecap="round" strokeLinejoin="round">{navIcons[screen]}</svg>;
}

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
  const navButton = (destination: Screen, label: string) => <button type="button" className={`nav-button${screen === destination ? " active" : ""}`} title={label} aria-current={screen === destination ? "page" : undefined} onClick={() => navigate(destination)}><NavIcon screen={destination}/><span>{label}</span></button>;

  return (
    <main className="application-shell">
      <aside className="app-sidebar">
        <div className="brand-row"><span className="brand-lockup"><img className="brand-logo" src={brandLogo} alt="AnyDesk"/><img className="brand-logo brand-logo-contrast" src={brandLogo} alt="" aria-hidden="true"/></span><img className="brand-icon" src={appIcon} alt="" aria-hidden="true"/></div>
        <nav className="main-navigation" aria-label="Main navigation">
          <div className="primary-nav"><p className="nav-group-label">Command center</p>{navButton("attention", "Attention")}{navButton("projects", "Projects")}</div>
          <div className="workspace-nav" aria-label="Workspace registers"><p className="nav-group-label">Project registers</p>{navButton("tasks", "Tasks")}{navButton("rfis", "RFIs")}{navButton("submittals", "Submittals")}{navButton("files", "Files")}{navButton("notes", "Notes & Contacts")}</div>
        </nav>
        <div className="sidebar-footer">{navButton("settings", "Settings")}<span className="local-indicator"><i aria-hidden="true"/>Local workspace</span></div>
      </aside>
      <div className="workspace-shell">
        <header className="app-header">
          <label className="project-context"><span>Working context</span><select value={projectContext} onChange={(event) => changeProjectContext(event.target.value)}><option value="">All projects</option>{projectOptions.map((project) => <option key={project.id} value={project.id}>{project.number} — {project.name}</option>)}</select></label>
          <button ref={searchButtonRef} type="button" className="search-trigger" aria-label="Search workspace" onClick={()=>setPaletteOpen(true)}><svg viewBox="0 0 24 24" aria-hidden="true"><circle cx="11" cy="11" r="6.5"/><path d="m16 16 4 4"/></svg><span>Search workspace</span><kbd>Ctrl+K</kbd></button>
        </header>
        <div className="workspace-content">
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
        </div>
      </div>
    </main>
  );
}

export default App;

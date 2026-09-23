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
import { QuickCapture } from "./QuickCapture";
import { Overview } from "./Overview";
import { Operations } from "./Operations";
import { remindersEnabled, runReminderCheck, setRemindersEnabled } from "./reminders";
import { AuditRecovery } from "./AuditRecovery";
import brandLogo from "./assets/anydesk-logo-transparent.png";
import appIcon from "./assets/anydesk-app-icon.jpg";

type ProjectRootSetting = { path: string | null };
type ProjectRootValidation = { canonicalPath: string; pathKind: "local" | "unc"; warning: string | null };
type CloudAuthStatus = { connected: boolean; email: string | null };
type CloudSyncResult = { outcome: string; message: string; cloudVersion: number; conflictCount: number };
type CloudConflict = { id: string; createdAtUtc: string };
type Screen = "overview" | "projects" | "tasks" | "rfis" | "submittals" | "files" | "notes" | "operations" | "attention" | "recovery" | "settings";
type ProjectSummary = { id: string; number: string; name: string };

const navIcons: Record<Screen, ReactNode> = {
  overview: <><path d="M4 5h7v6H4zM13 5h7v10h-7zM4 13h7v6H4zM13 17h7v2h-7z"/></>,
  attention: <><path d="M12 3.5 3.7 18h16.6L12 3.5Z"/><path d="M12 9v4.5M12 16.5h.01"/></>,
  projects: <><path d="M3.5 6.5h6l1.7 2h9.3v10H3.5z"/><path d="M3.5 8.5h17"/></>,
  tasks: <><path d="m4.5 7 1.8 1.8 3.2-3.3M11 7h8M4.5 13l1.8 1.8 3.2-3.3M11 13h8M4.5 19l1.8 1.8 3.2-3.3M11 19h8"/></>,
  rfis: <><path d="M5 4h14v16H5z"/><path d="M8 8h8M8 12h8M8 16h5"/></>,
  submittals: <><path d="M12 3v12M7.5 7.5 12 3l4.5 4.5"/><path d="M4 14v6h16v-6"/></>,
  files: <><path d="M6 3.5h8l4 4V20.5H6z"/><path d="M14 3.5v4h4M9 12h6M9 16h6"/></>,
  notes: <><path d="M4 4h16v13H8l-4 3z"/><path d="M8 8h8M8 12h6"/></>,
  operations: <><path d="M4 5h16v14H4zM4 9h16M9 9v10"/><path d="m12 13 1.5 1.5L17 11"/></>,
  settings: <><circle cx="12" cy="12" r="3"/><path d="M19.4 15a1.7 1.7 0 0 0 .34 1.88l.06.06-2.12 2.12-.06-.06a1.7 1.7 0 0 0-1.88-.34 1.7 1.7 0 0 0-1 1.55V20h-3v-.09a1.7 1.7 0 0 0-1.1-1.55 1.7 1.7 0 0 0-1.88.34l-.06.06-2.12-2.12.06-.06A1.7 1.7 0 0 0 7 14.7a1.7 1.7 0 0 0-1.55-1H5v-3h.45A1.7 1.7 0 0 0 7 9.6a1.7 1.7 0 0 0-.34-1.88l-.06-.06 2.12-2.12.06.06a1.7 1.7 0 0 0 1.88.34 1.7 1.7 0 0 0 1-1.55V4h3v.39a1.7 1.7 0 0 0 1 1.55 1.7 1.7 0 0 0 1.88-.34l.06-.06 2.12 2.12-.06.06a1.7 1.7 0 0 0-.34 1.88 1.7 1.7 0 0 0 1.55 1H21v3h-.05A1.7 1.7 0 0 0 19.4 15Z"/></>,
  recovery: <><path d="M4 7h16v12H4zM7 4h10v3"/><path d="M9 12h6M12 9v6"/></>,
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
  const [captureOpen, setCaptureOpen] = useState(false);
  const [navigationRevision, setNavigationRevision] = useState(0);
  const searchButtonRef = useRef<HTMLButtonElement>(null);
  const [projectContext, setProjectContext] = useState(() => localStorage.getItem("workspace.projectContext") ?? "");
  const [projectOptions, setProjectOptions] = useState<ProjectSummary[]>([]);
  const [cloudStatus, setCloudStatus] = useState<CloudAuthStatus>({ connected: false, email: null });
  const [cloudEmail, setCloudEmail] = useState("");
  const [cloudPassword, setCloudPassword] = useState("");
  const [cloudMessage, setCloudMessage] = useState("");
  const [cloudError, setCloudError] = useState("");
  const [cloudWorking, setCloudWorking] = useState(false);
  const [cloudConflicts, setCloudConflicts] = useState<CloudConflict[]>([]);
  const [remindersOn, setRemindersOn] = useState(remindersEnabled);

  useEffect(() => {
    void loadSetting();
    void invoke<ProjectSummary[]>("list_projects", { includeArchived: false }).then(setProjectOptions).catch(() => setProjectOptions([]));
    void invoke<CloudAuthStatus>("get_cloud_auth_status").then((status) => { setCloudStatus(status); if (status.connected) void loadCloudConflicts(); }).catch((caught) => setCloudError(describeAppError(caught)));
  }, []);
  useEffect(() => { localStorage.setItem("workspace.screen", screen); }, [screen]);
  useEffect(()=>{if(!remindersOn)return;void runReminderCheck();const timer=window.setInterval(()=>void runReminderCheck(),300000);return()=>window.clearInterval(timer)},[remindersOn]);
  useEffect(()=>{const restored=()=>setNavigationRevision(value=>value+1);window.addEventListener("workspace:restored",restored);return()=>window.removeEventListener("workspace:restored",restored)},[]);
  useEffect(() => { const handler=(event:KeyboardEvent)=>{if(event.ctrlKey&&event.key.toLowerCase()==="k"){event.preventDefault();setPaletteOpen(true)}if(event.ctrlKey&&event.shiftKey&&event.key.toLowerCase()==="n"){event.preventDefault();setCaptureOpen(true)}else if(event.ctrlKey&&event.key.toLowerCase()==="n"){event.preventDefault();window.dispatchEvent(new CustomEvent("workspace:new"))}if(event.key==="Escape")setPaletteOpen(false)};window.addEventListener("keydown",handler);return()=>window.removeEventListener("keydown",handler)},[]);
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
  async function signInCloud() {
    setCloudWorking(true); setCloudError(""); setCloudMessage("");
    try { const status = await invoke<CloudAuthStatus>("sign_in_cloud_with_password", { email: cloudEmail, password: cloudPassword }); setCloudStatus(status); setCloudPassword(""); setCloudMessage("Cloud account connected securely on this computer."); await loadCloudConflicts(); }
    catch (caught) { setCloudError(describeAppError(caught)); }
    finally { setCloudWorking(false); }
  }
  async function disconnectCloud() {
    if (!confirm("Disconnect this computer from the AnyDesk cloud workspace? Local projects and files will remain on this computer.")) return;
    setCloudWorking(true); setCloudError("");
    try { await invoke("disconnect_cloud"); setCloudStatus({ connected: false, email: null }); setCloudConflicts([]); setCloudPassword(""); setCloudMessage("This computer is disconnected. Local workspace data remains available."); }
    catch (caught) { setCloudError(describeAppError(caught)); }
    finally { setCloudWorking(false); }
  }
  async function loadCloudConflicts() { try { setCloudConflicts(await invoke<CloudConflict[]>("list_cloud_conflicts")); } catch (caught) { setCloudError(describeAppError(caught)); } }
  async function syncCloud() {
    setCloudWorking(true); setCloudError(""); setCloudMessage("");
    try { const result = await invoke<CloudSyncResult>("sync_cloud_workspace"); setCloudMessage(result.message); await loadCloudConflicts(); if (result.outcome === "downloaded") { setNavigationRevision((value) => value + 1); void invoke<ProjectSummary[]>("list_projects", { includeArchived: false }).then(setProjectOptions); } }
    catch (caught) { setCloudError(describeAppError(caught)); }
    finally { setCloudWorking(false); }
  }
  async function resolveCloudConflict(id: string, choice: "local" | "cloud") {
    const description = choice === "local" ? "replace the cloud metadata with this computer's version" : "replace this computer's metadata with the cloud version";
    if (!confirm(`Resolve this conflict and ${description}? Project files in OneDrive will not be deleted or overwritten.`)) return;
    setCloudWorking(true); setCloudError("");
    try { const result = await invoke<CloudSyncResult>("resolve_cloud_conflict", { id, choice }); setCloudMessage(result.message); await loadCloudConflicts(); setNavigationRevision((value) => value + 1); void invoke<ProjectSummary[]>("list_projects", { includeArchived: false }).then(setProjectOptions); }
    catch (caught) { setCloudError(describeAppError(caught)); }
    finally { setCloudWorking(false); }
  }
  function changeProjectContext(value: string) { setProjectContext(value); localStorage.setItem("workspace.projectContext", value); window.dispatchEvent(new CustomEvent("workspace:project", { detail: value })); }
  function closePalette() { setPaletteOpen(false); requestAnimationFrame(() => searchButtonRef.current?.focus()); }
  async function changeReminders(enabled:boolean){try{const accepted=await setRemindersEnabled(enabled);setRemindersOn(accepted&&enabled);if(accepted&&enabled){const total=await runReminderCheck(true);setMessage(total?`Windows reminders enabled. ${total} items currently need attention.`:"Windows reminders enabled. Nothing is due now.")}else if(!enabled)setMessage("Windows reminders disabled.");else setError("Windows notification permission was not granted.")}catch(caught){setError(describeAppError(caught))}}
  const navigate = (destination: Screen) => setScreen(destination);
  const navButton = (destination: Screen, label: string) => <button type="button" className={`nav-button${screen === destination ? " active" : ""}`} title={label} aria-current={screen === destination ? "page" : undefined} onClick={() => navigate(destination)}><NavIcon screen={destination}/><span>{label}</span></button>;

  return (
    <main className="application-shell">
      <aside className="app-sidebar">
        <div className="brand-row"><span className="brand-lockup"><img className="brand-logo" src={brandLogo} alt="AnyDesk"/><img className="brand-logo brand-logo-contrast" src={brandLogo} alt="" aria-hidden="true"/></span><img className="brand-icon" src={appIcon} alt="" aria-hidden="true"/></div>
        <nav className="main-navigation" aria-label="Main navigation">
          <div className="primary-nav"><p className="nav-group-label">Command center</p>{navButton("overview", "Overview")}{navButton("attention", "Attention")}{navButton("projects", "Projects")}</div>
          <div className="workspace-nav" aria-label="Workspace registers"><p className="nav-group-label">Project registers</p>{navButton("tasks", "Tasks")}{navButton("rfis", "RFIs")}{navButton("submittals", "Submittals")}{navButton("operations", "Operations")}{navButton("files", "Files")}{navButton("notes", "Notes & Contacts")}</div>
        </nav>
        <div className="sidebar-footer">{navButton("recovery", "Recovery")}{navButton("settings", "Settings")}<span className="local-indicator"><i aria-hidden="true"/>Local workspace</span></div>
      </aside>
      <div className="workspace-shell">
        <header className="app-header">
          <label className="project-context"><span>Working context</span><select value={projectContext} onChange={(event) => changeProjectContext(event.target.value)}><option value="">All projects</option>{projectOptions.map((project) => <option key={project.id} value={project.id}>{project.number} — {project.name}</option>)}</select></label>
          <div className="header-actions"><button type="button" className="secondary quick-capture-trigger" onClick={()=>setCaptureOpen(true)}>Quick capture <kbd>Ctrl+Shift+N</kbd></button><button ref={searchButtonRef} type="button" className="search-trigger" aria-label="Search workspace" onClick={()=>setPaletteOpen(true)}><svg viewBox="0 0 24 24" aria-hidden="true"><circle cx="11" cy="11" r="6.5"/><path d="m16 16 4 4"/></svg><span>Search workspace</span><kbd>Ctrl+K</kbd></button></div>
        </header>
        <div className="workspace-content">
      <SearchPalette open={paletteOpen} onClose={closePalette} onNavigate={(value)=>{setScreen(value as typeof screen);setNavigationRevision((current)=>current+1)}} />
      <QuickCapture open={captureOpen} projects={projectOptions} projectContext={projectContext} onClose={()=>setCaptureOpen(false)} onSaved={(value)=>{setScreen(value as Screen);setNavigationRevision((current)=>current+1)}} />
      {screen === "overview" ? <Overview key={navigationRevision} onNavigate={(value)=>setScreen(value as Screen)} /> : screen === "projects" ? <Projects key={navigationRevision} onOpenSettings={()=>setScreen("settings")} /> : screen === "tasks" ? <Tasks key={navigationRevision} /> : screen === "rfis" ? <Rfis key={navigationRevision} /> : screen === "submittals" ? <Submittals key={navigationRevision} /> : screen === "operations" ? <Operations key={navigationRevision} /> : screen === "files" ? <Files key={navigationRevision} /> : screen === "notes" ? <NotesContacts key={navigationRevision} /> : screen === "attention" ? <Tasks key={navigationRevision} attention onOpenRfis={() => setScreen("rfis")} onOpenSubmittals={() => setScreen("submittals")} /> : screen === "recovery" ? <AuditRecovery key={navigationRevision} onOpenFiles={()=>setScreen("files")}/> : <section className="settings" aria-labelledby="settings-title">
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
        <section className="cloud-settings" aria-labelledby="cloud-title">
          <div className="section-heading"><div><h2 id="cloud-title">Cloud workspace</h2><p className="intro">Sign in with the same confirmed account on each computer. Project files stay in your dedicated OneDrive project root; AnyDesk never syncs its local database through OneDrive.</p></div></div>
          {cloudStatus.connected ? <><div className="cloud-connected"><p><strong>Connected</strong><span>{cloudStatus.email}</span></p><div className="actions"><button type="button" onClick={() => void syncCloud()} disabled={cloudWorking}>{cloudWorking ? "Synchronizing…" : "Sync now"}</button><button type="button" className="secondary" onClick={() => void disconnectCloud()} disabled={cloudWorking}>Disconnect this computer</button></div></div>{cloudConflicts.length > 0 && <section className="conflict-panel" aria-labelledby="conflicts-title"><h3 id="conflicts-title">Sync conflict</h3><p>Both computers changed workspace metadata since the last successful sync. Choose the version to keep.</p>{cloudConflicts.map((conflict) => <div className="conflict-row" key={conflict.id}><span>Detected {new Date(conflict.createdAtUtc).toLocaleString()}</span><div className="actions"><button type="button" onClick={() => void resolveCloudConflict(conflict.id, "local")} disabled={cloudWorking}>Keep this computer</button><button type="button" className="secondary" onClick={() => void resolveCloudConflict(conflict.id, "cloud")} disabled={cloudWorking}>Use cloud version</button></div></div>)}</section>}</> : <form className="cloud-auth" onSubmit={(event) => { event.preventDefault(); void signInCloud(); }}>
            <label htmlFor="cloud-email">Email address<input id="cloud-email" type="email" autoComplete="email" required value={cloudEmail} onChange={(event) => { setCloudEmail(event.target.value); setCloudError(""); }} placeholder="you@company.com" /></label>
            <label htmlFor="cloud-password">Password<input id="cloud-password" type="password" autoComplete="current-password" required minLength={8} value={cloudPassword} onChange={(event) => { setCloudPassword(event.target.value); setCloudError(""); }} /></label>
            <p className="help">Use the confirmed user created in Supabase Authentication → Users. Windows Credential Manager securely stores the session on this computer.</p>
            <div className="actions"><button type="submit" disabled={cloudWorking || !cloudEmail.trim() || cloudPassword.length < 8}>{cloudWorking ? "Signing in…" : "Sign in"}</button></div>
          </form>}
          <div className="status-area" aria-live="polite">{cloudMessage && <p className="status success">{cloudMessage}</p>}{cloudError && <p className="status error" role="alert">{cloudError}</p>}</div>
        </section>
        <section className="cloud-settings" aria-labelledby="reminders-title"><div className="section-heading"><div><h2 id="reminders-title">Local reminders</h2><p className="intro">Show a Windows notification for overdue and due-today tasks, follow-ups, and project-control records while AnyDesk is running.</p></div></div><label className="setting-toggle"><input type="checkbox" checked={remindersOn} onChange={(event)=>void changeReminders(event.target.checked)}/> Enable Windows reminders</label>{remindersOn&&<button type="button" className="secondary" onClick={()=>void runReminderCheck(true).then(total=>setMessage(total?`Reminder sent for ${total} current items.`:"No items currently need a reminder.")).catch(caught=>setError(describeAppError(caught)))}>Check reminders now</button>}</section>
      </section>}
        </div>
      </div>
    </main>
  );
}

export default App;

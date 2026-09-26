import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { describeAppError } from "./error";
import { StatusNotice } from "./Feedback";
import brandLogo from "./assets/branding/site-datum-wordmark.png";

type RootValidation = { canonicalPath: string; pathKind: "local" | "unc"; warning: string | null };
type Project = { id: string; number: string; name: string };
type FolderPreview = { projectPath: string };
type ProjectInput = { number: string; name: string; status: string; phase: string; customPhaseName: string; customer: string };

const initialProject = (): ProjectInput => ({ number: "", name: "", status: "active", phase: "engineering", customPhaseName: "", customer: "" });
const projectPayload = (project: ProjectInput) => ({ ...project, customPhaseName: project.customPhaseName || null, customer: project.customer || null, generalContractor: null, engineer: null, projectManager: null, superintendent: null, location: null, startDate: null, targetDate: null, description: null, importantNotes: null });

export function FirstRun({ onComplete, onSkip }: { onComplete: (project: Project) => void; onSkip: () => void }) {
  const [step, setStep] = useState<"root" | "project">("root");
  const [path, setPath] = useState("");
  const [project, setProject] = useState<ProjectInput>(initialProject);
  const [preview, setPreview] = useState("");
  const [warning, setWarning] = useState("");
  const [error, setError] = useState("");
  const [working, setWorking] = useState(false);

  async function browse() {
    const selected = await open({ directory: true, multiple: false, title: "Choose the project root" });
    if (typeof selected === "string") { setPath(selected); setError(""); setWarning(""); }
  }

  async function saveRoot(event: React.FormEvent) {
    event.preventDefault(); setWorking(true); setError("");
    try {
      const checked = await invoke<RootValidation>("validate_project_root_command", { path });
      await invoke("save_project_root", { path: checked.canonicalPath });
      setPath(checked.canonicalPath); setWarning(checked.warning ?? ""); setStep("project");
    } catch (caught) { setError(describeAppError(caught)); }
    finally { setWorking(false); }
  }

  function change(key: keyof ProjectInput, value: string) {
    setProject((current) => ({ ...current, [key]: value, ...(key === "phase" && value !== "custom" ? { customPhaseName: "" } : {}) }));
    setPreview(""); setError("");
  }

  async function previewFolder() {
    setWorking(true); setError("");
    try { const result = await invoke<FolderPreview>("preview_project_folder", { input: projectPayload(project) }); setPreview(result.projectPath); }
    catch (caught) { setError(describeAppError(caught)); }
    finally { setWorking(false); }
  }

  async function createProject(event: React.FormEvent) {
    event.preventDefault();
    if (!preview) { await previewFolder(); return; }
    setWorking(true); setError("");
    try { onComplete(await invoke<Project>("create_project", { input: projectPayload(project) })); }
    catch (caught) { setError(describeAppError(caught)); }
    finally { setWorking(false); }
  }

  const projectValid = project.number.trim() && project.name.trim() && (project.phase !== "custom" || project.customPhaseName.trim());

  return <main className="first-run-shell">
    <section className="first-run-panel" aria-labelledby="first-run-title">
      <header className="first-run-header"><img src={brandLogo} alt="SiteDatum"/><div><span>Local Project Engineer workspace</span><strong>Step {step === "root" ? "1" : "2"} of 2</strong></div></header>
      {step === "root" ? <form onSubmit={saveRoot}>
        <p className="eyebrow">Workspace location</p><h1 id="first-run-title">Choose your project root</h1>
        <p className="intro">SiteDatum creates project folders beneath this existing local folder or UNC share. Your documents remain normal Windows files.</p>
        <div className="first-run-field"><label htmlFor="first-run-root">Existing folder path</label><div className="path-control"><input id="first-run-root" required value={path} onChange={(event) => { setPath(event.target.value); setError(""); }} placeholder="C:\\Projects or \\server\\share\\Projects" spellCheck="false" autoFocus/><button type="button" className="secondary" onClick={() => void browse()} disabled={working}>Browse…</button></div></div>
        <p className="help">The folder must already exist and be readable. Existing project folders are never overwritten.</p>
        {error && <StatusNotice tone="error">{error}</StatusNotice>}
        <div className="first-run-actions"><button type="button" className="quiet" onClick={onSkip}>Set up later</button><button type="submit" disabled={working || !path.trim()}>{working ? "Checking location…" : "Check and continue"}</button></div>
      </form> : <form onSubmit={createProject}>
        <p className="eyebrow">First project</p><h1 id="first-run-title">Create your first workspace</h1>
        <p className="intro">Start with the project identity. Additional team, schedule, and project details can be added later.</p>
        {warning && <StatusNotice tone="warning">{warning}</StatusNotice>}
        <div className="form-grid first-run-project-fields"><label>Project number<input required autoFocus value={project.number} onChange={(event) => change("number", event.target.value)}/></label><label>Project name<input required value={project.name} onChange={(event) => change("name", event.target.value)}/></label><label>Phase<select value={project.phase} onChange={(event) => change("phase", event.target.value)}>{["preconstruction","engineering","submittals","procurement","construction","programming","startup","commissioning","closeout","custom"].map((phase) => <option key={phase} value={phase}>{phase.replace(/_/g," ").replace(/\b\w/g,(letter)=>letter.toUpperCase())}</option>)}</select></label>{project.phase === "custom" && <label>Custom phase name<input required value={project.customPhaseName} onChange={(event) => change("customPhaseName", event.target.value)}/></label>}<label>Customer <span className="optional">(optional)</span><input value={project.customer} onChange={(event) => change("customer", event.target.value)}/></label></div>
        {preview && <dl className="operation-preview"><div><dt>Project folder to create</dt><dd>{preview}</dd></div></dl>}
        {error && <StatusNotice tone="error">{error}</StatusNotice>}
        <p className="first-run-offline">Cloud sync is optional and can be configured later. Local use does not require a connection.</p>
        <div className="first-run-actions"><button type="button" className="secondary" onClick={() => { setStep("root"); setPreview(""); }} disabled={working}>Back</button><button type="submit" disabled={working || !projectValid}>{working ? "Working…" : preview ? "Create project and open workspace" : "Preview project folder"}</button></div>
      </form>}
    </section>
  </main>;
}

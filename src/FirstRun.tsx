import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { describeAppError } from "./error";
import { FieldError, StatusNotice } from "./Feedback";
import { Brand } from "./Brand";
import { displayWindowsPath } from "./windowsPath";
import { firstFieldError, requiredFieldErrors } from "./formGuidance";

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
  const [rootAttempted, setRootAttempted] = useState(false);
  const [projectAttempted, setProjectAttempted] = useState(false);

  const rootErrors = rootAttempted ? requiredFieldErrors([{ key: "first-run-root", label: "Project root", value: path }]) : {};
  const projectErrors = projectAttempted ? requiredFieldErrors([
    { key: "first-run-project-number", label: "Project number", value: project.number },
    { key: "first-run-project-name", label: "Project name", value: project.name },
    { key: "first-run-custom-phase", label: "Custom phase name", value: project.customPhaseName, required: project.phase === "custom" },
  ]) : {};

  async function browse() {
    const selected = await open({ directory: true, multiple: false, title: "Choose the project root" });
    if (typeof selected === "string") { setPath(displayWindowsPath(selected)); setError(""); setWarning(""); }
  }

  async function saveRoot(event: React.FormEvent) {
    event.preventDefault(); setRootAttempted(true); setError("");
    const errors = requiredFieldErrors([{ key: "first-run-root", label: "Project root", value: path }]);
    const firstError = firstFieldError(errors);
    if (firstError) { document.getElementById(firstError)?.focus(); return; }
    setWorking(true);
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
    setProjectAttempted(true); setError("");
    const errors = requiredFieldErrors([
      { key: "first-run-project-number", label: "Project number", value: project.number },
      { key: "first-run-project-name", label: "Project name", value: project.name },
      { key: "first-run-custom-phase", label: "Custom phase name", value: project.customPhaseName, required: project.phase === "custom" },
    ]);
    const firstError = firstFieldError(errors);
    if (firstError) { document.getElementById(firstError)?.focus(); return; }
    setWorking(true);
    try { const result = await invoke<FolderPreview>("preview_project_folder", { input: projectPayload(project) }); setPreview(result.projectPath); }
    catch (caught) { setError(describeAppError(caught)); }
    finally { setWorking(false); }
  }

  async function createProject(event: React.FormEvent) {
    event.preventDefault();
    setProjectAttempted(true);
    const errors = requiredFieldErrors([
      { key: "first-run-project-number", label: "Project number", value: project.number },
      { key: "first-run-project-name", label: "Project name", value: project.name },
      { key: "first-run-custom-phase", label: "Custom phase name", value: project.customPhaseName, required: project.phase === "custom" },
    ]);
    const firstError = firstFieldError(errors);
    if (firstError) { document.getElementById(firstError)?.focus(); return; }
    if (!preview) { await previewFolder(); return; }
    setWorking(true); setError("");
    try { onComplete(await invoke<Project>("create_project", { input: projectPayload(project) })); }
    catch (caught) { setError(describeAppError(caught)); }
    finally { setWorking(false); }
  }

  return <main className="first-run-shell">
    <section className="first-run-panel" aria-labelledby="first-run-title">
      <header className="first-run-header"><Brand /><div><span>Local Project Engineer workspace</span><strong>Step {step === "root" ? "1" : "2"} of 2</strong></div></header>
      {step === "root" ? <form onSubmit={saveRoot} noValidate>
        <p className="eyebrow">Workspace location</p><h1 id="first-run-title">Choose your project root</h1>
        <p className="intro">SiteDatum creates project folders beneath this existing local folder or UNC share. Your documents remain normal Windows files.</p>
        <div className="first-run-field"><label htmlFor="first-run-root">Existing folder path</label><div className="path-control"><input id="first-run-root" required value={path} onChange={(event) => { setPath(event.target.value); setError(""); }} placeholder="C:\\Projects or \\server\\share\\Projects" spellCheck="false" autoFocus aria-invalid={Boolean(rootErrors["first-run-root"])} aria-describedby={rootErrors["first-run-root"] ? "first-run-root-help first-run-root-error" : "first-run-root-help"}/><button type="button" className="secondary" onClick={() => void browse()} disabled={working}>Browse…</button></div><FieldError id="first-run-root-error">{rootErrors["first-run-root"]}</FieldError></div>
        <p id="first-run-root-help" className="help">The folder must already exist and be readable. Existing project folders are never overwritten.</p>
        {error && <StatusNotice tone="error">{error}</StatusNotice>}
        <div className="first-run-actions"><button type="button" className="quiet" onClick={onSkip}>Set up later</button><button type="submit" disabled={working}>{working ? "Checking location…" : "Check and continue"}</button></div>
      </form> : <form onSubmit={createProject} noValidate>
        <p className="eyebrow">First project</p><h1 id="first-run-title">Create your first workspace</h1>
        <p className="intro">Start with the project identity. Additional team, schedule, and project details can be added later.</p>
        {warning && <StatusNotice tone="warning">{warning}</StatusNotice>}
        <div className="form-grid first-run-project-fields"><label htmlFor="first-run-project-number">Project number<input id="first-run-project-number" required autoFocus value={project.number} onChange={(event) => change("number", event.target.value)} aria-invalid={Boolean(projectErrors["first-run-project-number"])} aria-describedby={projectErrors["first-run-project-number"] ? "first-run-project-number-error" : undefined}/><FieldError id="first-run-project-number-error">{projectErrors["first-run-project-number"]}</FieldError></label><label htmlFor="first-run-project-name">Project name<input id="first-run-project-name" required value={project.name} onChange={(event) => change("name", event.target.value)} aria-invalid={Boolean(projectErrors["first-run-project-name"])} aria-describedby={projectErrors["first-run-project-name"] ? "first-run-project-name-error" : undefined}/><FieldError id="first-run-project-name-error">{projectErrors["first-run-project-name"]}</FieldError></label><label>Phase<select value={project.phase} onChange={(event) => change("phase", event.target.value)}>{["preconstruction","engineering","submittals","procurement","construction","programming","startup","commissioning","closeout","custom"].map((phase) => <option key={phase} value={phase}>{phase.replace(/_/g," ").replace(/\b\w/g,(letter)=>letter.toUpperCase())}</option>)}</select></label>{project.phase === "custom" && <label htmlFor="first-run-custom-phase">Custom phase name<input id="first-run-custom-phase" required value={project.customPhaseName} onChange={(event) => change("customPhaseName", event.target.value)} aria-invalid={Boolean(projectErrors["first-run-custom-phase"])} aria-describedby={projectErrors["first-run-custom-phase"] ? "first-run-custom-phase-error" : undefined}/><FieldError id="first-run-custom-phase-error">{projectErrors["first-run-custom-phase"]}</FieldError></label>}<label>Customer <span className="optional">(optional)</span><input value={project.customer} onChange={(event) => change("customer", event.target.value)}/></label></div>
        {preview && <dl className="operation-preview"><div><dt>Project folder to create</dt><dd>{preview}</dd></div></dl>}
        {error && <StatusNotice tone="error">{error}</StatusNotice>}
        <p className="first-run-offline">Local use does not require an account or internet connection.</p>
        <div className="first-run-actions"><button type="button" className="secondary" onClick={() => { setStep("root"); setPreview(""); }} disabled={working}>Back</button><button type="submit" disabled={working}>{working ? "Working…" : preview ? "Create project and open workspace" : "Preview project folder"}</button></div>
      </form>}
    </section>
  </main>;
}

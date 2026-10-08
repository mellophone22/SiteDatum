import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { describeAppError } from "./error";
import { StatusNotice, useConfirmation } from "./Feedback";

export type StartupStatus = {
  ready: boolean;
  code: string | null;
  message: string | null;
  recovery: string | null;
  correlationId: string | null;
  databasePath: string;
  backupDirectory: string;
};
type Preview = { projects: number; tasks: number; rfis: number; submittals: number; workItems: number; notes: number; contacts: number };
type RecoveryResult = { restoredPath: string; preservedPath: string | null };

export function StartupRecovery({ status }: { status: StartupStatus }) {
  const [selectedPath, setSelectedPath] = useState("");
  const [preview, setPreview] = useState<Preview | null>(null);
  const [error, setError] = useState("");
  const [result, setResult] = useState<RecoveryResult | null>(null);
  const [working, setWorking] = useState(false);
  const { confirmAction, confirmationDialog } = useConfirmation();

  async function choose() {
    const selected = await open({ multiple: false, directory: false, filters: [{ name: "SiteDatum backup", extensions: ["sqlite3"] }], title: "Choose a SiteDatum backup" });
    if (typeof selected !== "string") return;
    setWorking(true); setError(""); setResult(null);
    try { setPreview(await invoke<Preview>("preview_backup_file", { path: selected })); setSelectedPath(selected); }
    catch (caught) { setPreview(null); setSelectedPath(""); setError(describeAppError(caught)); }
    finally { setWorking(false); }
  }
  async function recover() {
    if (!preview || !await confirmAction({ title: "Recover the workspace database?", description: <><p>The selected verified backup will become the active SiteDatum database.</p><p>The unavailable database is preserved in the backup folder and project documents are not changed.</p></>, confirmLabel: "Recover database", destructive: true })) return;
    setWorking(true); setError("");
    try { setResult(await invoke<RecoveryResult>("recover_startup_database", { path: selectedPath })); }
    catch (caught) { setError(describeAppError(caught)); }
    finally { setWorking(false); }
  }

  return <main className="startup-recovery-shell">
    {confirmationDialog}
    <section className="startup-recovery-panel" aria-labelledby="startup-recovery-title">
      <header><span className="startup-brand"><strong>Site</strong><em>Datum</em></span><span>Workspace recovery</span></header>
      <div className="startup-recovery-content">
        <p className="eyebrow">Safe startup</p>
        <h1 id="startup-recovery-title">SiteDatum could not open the local workspace.</h1>
        <p className="intro">{status.message} {status.recovery}</p>
        <StatusNotice tone="error">Code {status.code ?? "DATABASE_STARTUP_FAILED"}. Reference {status.correlationId ?? "unavailable"}.</StatusNotice>
        <dl className="recovery-locations"><div><dt>Workspace database</dt><dd>{status.databasePath}</dd></div><div><dt>Backup folder</dt><dd>{status.backupDirectory}</dd></div></dl>
        {!result && <><div className="actions"><button type="button" onClick={() => void choose()} disabled={working}>{working ? "Checking…" : "Choose backup file…"}</button></div>
        {preview && <div className="restore-preview"><h2>Verified backup</h2><p className="backup-path">{selectedPath}</p><p>{preview.projects} projects · {preview.tasks} tasks · {preview.rfis} RFIs · {preview.submittals} submittals · {preview.workItems} operations records · {preview.notes} notes · {preview.contacts} contacts</p><button type="button" className="destructive-button" disabled={working} onClick={() => void recover()}>{working ? "Recovering…" : "Recover from this backup"}</button></div>}</>}
        {result && <StatusNotice tone="success"><strong>Recovery completed.</strong> Close and reopen SiteDatum. The previous database was preserved at {result.preservedPath ?? status.backupDirectory}.</StatusNotice>}
        {error && <StatusNotice tone="error">{error}</StatusNotice>}
        <p className="help">SiteDatum does not modify project folders or documents during database recovery.</p>
      </div>
    </section>
  </main>;
}

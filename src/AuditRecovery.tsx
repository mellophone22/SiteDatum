import { useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { describeAppError } from "./error";
import { EmptyState, StatusNotice, useConfirmation } from "./Feedback";

type Activity = { eventType: string; entityType: string; summary: string; occurredAtUtc: string };
type FileRecord = { id: string; projectNumber: string; projectName: string; fileName: string; filePath: string; missing: boolean };
type Conflict = { id: string; createdAtUtc: string };
type Backup = { path: string; fileName: string; sizeBytes: number; modifiedAtUtc: string };
type Preview = { projects: number; tasks: number; rfis: number; submittals: number; workItems: number; notes: number; contacts: number };
type BackupSettings = { schedule: "off" | "daily" | "weekly"; destinationPath: string | null; lastSuccessUtc: number | null };
type PortableExport = { path: string; exportedAtUtc: number; files: { entity: string; fileName: string; rowCount: number }[] };

const backupDate = (value: string) => new Date(Number(value) * 1000).toLocaleString();

export function AuditRecovery({ onOpenFiles }: { onOpenFiles: () => void }) {
  const [activity, setActivity] = useState<Activity[]>([]);
  const [files, setFiles] = useState<FileRecord[]>([]);
  const [conflicts, setConflicts] = useState<Conflict[]>([]);
  const [backups, setBackups] = useState<Backup[]>([]);
  const [settings, setSettings] = useState<BackupSettings>({ schedule: "off", destinationPath: null, lastSuccessUtc: null });
  const [query, setQuery] = useState("");
  const [preview, setPreview] = useState<{ backup: Backup; counts: Preview } | null>(null);
  const [error, setError] = useState("");
  const [message, setMessage] = useState("");
  const [working, setWorking] = useState(false);
  const { confirmAction, confirmationDialog } = useConfirmation();

  const load = async () => {
    try {
      const [a, f, b, s] = await Promise.all([
        invoke<Activity[]>("list_activity"), invoke<FileRecord[]>("list_files"),
        invoke<Backup[]>("list_configured_backups"), invoke<BackupSettings>("get_backup_settings"),
      ]);
      setActivity(a); setFiles(f); setBackups(b); setSettings(s);
      try { setConflicts(await invoke<Conflict[]>("list_cloud_conflicts")); } catch { setConflicts([]); }
      setError("");
    } catch (caught) { setError(describeAppError(caught)); }
  };

  useEffect(() => { void load(); }, []);
  const visible = useMemo(() => activity.filter((value) => `${value.summary} ${value.entityType} ${value.eventType}`.toLowerCase().includes(query.toLowerCase())), [activity, query]);
  const missing = files.filter((value) => value.missing);

  async function inspect(backup: Backup) {
    try { setError(""); setPreview({ backup, counts: await invoke<Preview>("preview_backup_file", { path: backup.path }) }); }
    catch (caught) { setError(describeAppError(caught)); }
  }
  async function chooseBackupFile() {
    const selected = await open({ multiple: false, directory: false, filters: [{ name: "SiteDatum backup", extensions: ["sqlite3"] }], title: "Choose a SiteDatum backup" });
    if (typeof selected === "string") await inspect({ path: selected, fileName: selected.split(/[\\/]/).pop() ?? "Selected backup", sizeBytes: 0, modifiedAtUtc: "0" });
  }
  async function restore() {
    if (!preview || !await confirmAction({ title: "Restore this backup?", description: <><p><strong>{preview.backup.fileName}</strong> will replace the current application metadata.</p><p>A verified safety backup is created first. Project files will not be changed.</p></>, confirmLabel: "Restore backup", destructive: true })) return;
    setWorking(true); setError("");
    try { await invoke("restore_backup_file", { path: preview.backup.path }); setMessage("Backup restored. All workspace views have been reloaded from the selected snapshot."); setPreview(null); await load(); window.dispatchEvent(new CustomEvent("workspace:restored")); }
    catch (caught) { setError(describeAppError(caught)); }
    finally { setWorking(false); }
  }
  async function chooseDestination() {
    const selected = await open({ directory: true, multiple: false, title: "Choose external backup folder" });
    if (typeof selected === "string") setSettings((current) => ({ ...current, destinationPath: selected }));
  }
  async function saveBackupSettings() {
    setWorking(true); setError("");
    try { const saved = await invoke<BackupSettings>("save_backup_settings", { settings }); setSettings(saved); setMessage(saved.schedule === "off" ? "External backup location saved. Scheduled backups remain off." : `Scheduled ${saved.schedule} backups are enabled while SiteDatum is running.`); await load(); }
    catch (caught) { setError(describeAppError(caught)); }
    finally { setWorking(false); }
  }
  async function createExternalBackup() {
    if (!settings.destinationPath) return;
    setWorking(true); setError("");
    try { const backup = await invoke<Backup>("create_external_backup", { directory: settings.destinationPath }); setMessage(`Verified backup created: ${backup.path}`); await load(); }
    catch (caught) { setError(describeAppError(caught)); }
    finally { setWorking(false); }
  }
  async function exportCompleteCsv() {
    const selected = await open({ directory: true, multiple: false, title: "Choose a folder for the SiteDatum export" });
    if (typeof selected !== "string") return;
    setWorking(true); setError("");
    try {
      const result = await invoke<PortableExport>("export_complete_csv", { directory: selected });
      const rows = result.files.reduce((total, file) => total + file.rowCount, 0);
      setMessage(`Complete CSV export created with ${rows} records across ${result.files.length} files: ${result.path}`);
    } catch (caught) { setError(describeAppError(caught)); }
    finally { setWorking(false); }
  }

  return <section className="projects-page" aria-labelledby="audit-title">
    {confirmationDialog}
    <div className="page-toolbar"><div><p className="eyebrow">Recovery</p><h1 id="audit-title">Audit and recovery</h1></div><button className="secondary" onClick={() => void load()}>Refresh</button></div>
    {message && <StatusNotice tone="success">{message}</StatusNotice>}{error && <StatusNotice tone="error">{error}</StatusNotice>}
    <div className="recovery-summary">
      <section><h2>Missing files <span>{missing.length}</span></h2>{missing.length ? <><ul className="overview-list">{missing.slice(0, 8).map((value) => <li key={value.id}><strong>{value.fileName}</strong><span>{value.projectNumber} — {value.projectName}</span></li>)}</ul><button className="secondary" onClick={onOpenFiles}>Open file recovery</button></> : <p>No registered files are missing.</p>}</section>
      <section><h2>Sync conflicts <span>{conflicts.length}</span></h2>{conflicts.length ? <p>Resolve these conflicts from Settings before the next sync.</p> : <p>No unresolved cloud conflicts.</p>}</section>
    </div>
    <section className="recovery-section" aria-labelledby="portable-export-title">
      <div className="detail-heading"><div><h2 id="portable-export-title">Complete data export</h2><p>Export every core record to separate, machine-readable CSV files. This is always available on Free and does not require an account.</p></div><button type="button" className="secondary" onClick={() => void exportCompleteCsv()} disabled={working}>Export all CSV data…</button></div>
      <p className="help">The export includes active and archived metadata, relationships, attachment references, templates, and activity history. Referenced document files remain in their existing Windows folders and are not copied.</p>
    </section>
    <section className="recovery-section" aria-labelledby="backup-plan-title">
      <div className="detail-heading"><div><h2 id="backup-plan-title">External backup plan</h2><p>Keep a second copy outside SiteDatum's app storage. Scheduled backups run when SiteDatum is open.</p></div></div>
      <div className="backup-plan-grid">
        <label>Schedule<select value={settings.schedule} onChange={(event) => setSettings((current) => ({ ...current, schedule: event.target.value as BackupSettings["schedule"] }))}><option value="off">Off</option><option value="daily">Daily</option><option value="weekly">Weekly</option></select></label>
        <label className="backup-destination">Destination folder<div className="path-control"><input value={settings.destinationPath ?? ""} readOnly placeholder="Choose an external folder"/><button type="button" className="secondary" onClick={() => void chooseDestination()}>Browse…</button></div></label>
      </div>
      <p className="help">{settings.lastSuccessUtc ? `Last scheduled backup: ${new Date(settings.lastSuccessUtc * 1000).toLocaleString()}` : "No scheduled backup has completed yet."}</p>
      <div className="actions"><button type="button" onClick={() => void saveBackupSettings()} disabled={working || (settings.schedule !== "off" && !settings.destinationPath)}>Save backup plan</button><button type="button" className="secondary" onClick={() => void createExternalBackup()} disabled={working || !settings.destinationPath}>Back up now</button></div>
    </section>
    <section className="recovery-section">
      <div className="detail-heading"><div><h2>Available backups</h2><p>App-local and configured external backups are listed with their exact location.</p></div><button type="button" className="secondary" onClick={() => void chooseBackupFile()}>Restore from file…</button></div>
      {backups.length ? <table><thead><tr><th>Backup</th><th>Created</th><th>Size</th><th></th></tr></thead><tbody>{backups.map((value) => <tr key={value.path}><td><strong>{value.fileName}</strong><span className="backup-path">{value.path}</span></td><td>{backupDate(value.modifiedAtUtc)}</td><td>{(value.sizeBytes / 1024 / 1024).toFixed(1)} MB</td><td><button className="quiet" onClick={() => void inspect(value)}>Preview restore</button></td></tr>)}</tbody></table> : <EmptyState>No backups are available. Create one from Settings or choose an external destination above.</EmptyState>}
      {preview && <div className="restore-preview"><h3>Restore preview</h3><p className="backup-path">{preview.backup.path}</p><p>{preview.counts.projects} projects · {preview.counts.tasks} tasks · {preview.counts.rfis} RFIs · {preview.counts.submittals} submittals · {preview.counts.workItems} operations records · {preview.counts.notes} notes · {preview.counts.contacts} contacts</p><div className="actions"><button className="destructive-button" disabled={working} onClick={() => void restore()}>{working ? "Restoring…" : "Restore this backup"}</button><button className="secondary" onClick={() => setPreview(null)}>Cancel</button></div></div>}
    </section>
    <section className="recovery-section"><div className="detail-heading"><h2>Activity audit</h2><input aria-label="Search activity" placeholder="Search activity" value={query} onChange={(event) => setQuery(event.target.value)}/></div><table><thead><tr><th>Time</th><th>Area</th><th>Event</th></tr></thead><tbody>{visible.map((value, index) => <tr key={`${value.occurredAtUtc}-${index}`}><td>{new Date(value.occurredAtUtc).toLocaleString()}</td><td>{value.entityType}</td><td>{value.summary}</td></tr>)}</tbody></table></section>
  </section>;
}

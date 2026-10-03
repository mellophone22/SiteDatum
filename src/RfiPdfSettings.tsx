import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { describeAppError } from "./error";
import { StatusNotice } from "./Feedback";

type TemplateMode = "site_datum" | "custom";

type RfiPdfConfiguration = {
  templateMode: TemplateMode;
  companyName: string;
  companyDetails: string;
  accentColor: string;
  customTemplatePath: string | null;
};

type LicensingStatus = { isPro: boolean };

const defaults: RfiPdfConfiguration = {
  templateMode: "site_datum",
  companyName: "",
  companyDetails: "",
  accentColor: "#087F89",
  customTemplatePath: null,
};

export function RfiPdfSettings() {
  const [settings, setSettings] = useState<RfiPdfConfiguration>(defaults);
  const [isPro, setIsPro] = useState(false);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [message, setMessage] = useState("");
  const [error, setError] = useState("");

  useEffect(() => {
    void Promise.all([
      invoke<RfiPdfConfiguration>("get_rfi_pdf_settings"),
      invoke<LicensingStatus>("get_licensing_status"),
    ])
      .then(([configuration, licensing]) => {
        setSettings(configuration);
        setIsPro(licensing.isPro);
      })
      .catch((caught) => setError(describeAppError(caught)))
      .finally(() => setLoading(false));
  }, []);

  async function chooseTemplate() {
    const path = await open({
      multiple: false,
      directory: false,
      title: "Choose a custom RFI PDF template",
      filters: [{ name: "PDF document", extensions: ["pdf"] }],
    });
    if (typeof path === "string") {
      setSettings((current) => ({ ...current, customTemplatePath: path }));
      setError("");
      setMessage("");
    }
  }

  async function saveSettings(event: React.FormEvent) {
    event.preventDefault();
    setSaving(true);
    setMessage("");
    setError("");
    try {
      const saved = await invoke<RfiPdfConfiguration>("save_rfi_pdf_settings", { settings });
      setSettings(saved);
      setMessage(saved.templateMode === "custom"
        ? "Custom RFI template validated and saved. The original PDF was not copied or modified."
        : "SiteDatum RFI branding saved locally on this computer.");
    } catch (caught) {
      setError(describeAppError(caught));
    } finally {
      setSaving(false);
    }
  }

  return <section className="settings-section rfi-pdf-settings" aria-labelledby="rfi-pdf-settings-title">
    <div className="settings-section-heading">
      <h2 id="rfi-pdf-settings-title">RFI PDF</h2>
      <p>Choose the locally generated SiteDatum layout or reference a customer-owned PDF template. No branding or project information is uploaded.</p>
    </div>
    {loading ? <p className="help">Loading RFI PDF settings…</p> : <form onSubmit={(event) => void saveSettings(event)}>
      <fieldset className="template-mode-selector">
        <legend>PDF layout</legend>
        <label className={settings.templateMode === "site_datum" ? "selected" : undefined}>
          <input type="radio" name="rfi-template-mode" value="site_datum" checked={settings.templateMode === "site_datum"} onChange={() => setSettings((current) => ({ ...current, templateMode: "site_datum" }))} />
          <span><strong>SiteDatum layout</strong><small>Add your company name, contact line, and accent color to a neutral RFI form.</small></span>
        </label>
        <label className={`${settings.templateMode === "custom" ? "selected " : ""}${!isPro ? "locked" : ""}`}>
          <input type="radio" name="rfi-template-mode" value="custom" checked={settings.templateMode === "custom"} disabled={!isPro} onChange={() => setSettings((current) => ({ ...current, templateMode: "custom" }))} />
          <span><strong>Custom PDF template <em>Pro</em></strong><small>Reference your own one-page portrait Letter PDF. SiteDatum never copies or edits the source file.</small></span>
        </label>
      </fieldset>

      {settings.templateMode === "site_datum" ? <div className="rfi-brand-fields">
        <label>Company name<input value={settings.companyName} maxLength={120} placeholder="Your company" onChange={(event) => setSettings((current) => ({ ...current, companyName: event.target.value }))} /></label>
        <label className="wide-field">Company details <span className="optional">(optional)</span><textarea value={settings.companyDetails} maxLength={360} placeholder="Address · phone · email · website" onChange={(event) => setSettings((current) => ({ ...current, companyDetails: event.target.value }))} /></label>
        <label>Accent color<span className="color-input"><input type="color" value={settings.accentColor} aria-label="Choose RFI accent color" onChange={(event) => setSettings((current) => ({ ...current, accentColor: event.target.value.toUpperCase() }))} /><input value={settings.accentColor} pattern="#[0-9A-Fa-f]{6}" aria-label="RFI accent color hex value" onChange={(event) => setSettings((current) => ({ ...current, accentColor: event.target.value }))} /></span></label>
      </div> : <div className="custom-template-control">
        <div className="path-control"><input aria-label="Custom RFI template path" value={settings.customTemplatePath ?? ""} readOnly placeholder="Choose a one-page portrait Letter PDF" /><button type="button" className="secondary" onClick={() => void chooseTemplate()} disabled={!isPro || saving}>Browse…</button></div>
        <p className="help">The template must use SiteDatum’s RFI field positions. Put logos and permanent company artwork directly in that PDF. If its drive is unavailable, export stops and explains how to recover.</p>
      </div>}

      {!isPro && <p className="pro-template-note">Custom PDF templates are a Pro feature. Your SiteDatum branding settings remain local and editable.</p>}
      <div className="actions"><button type="submit" disabled={saving || (settings.templateMode === "custom" && !settings.customTemplatePath)}>{saving ? "Saving…" : "Save RFI PDF settings"}</button></div>
      <div className="status-area" aria-live="polite">{message && <StatusNotice tone="success">{message}</StatusNotice>}{error && <StatusNotice tone="error">{error}</StatusNotice>}</div>
    </form>}
  </section>;
}

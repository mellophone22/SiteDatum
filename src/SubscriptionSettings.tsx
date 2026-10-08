import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { openUrl } from "@tauri-apps/plugin-opener";
import { describeAppError } from "./error";
import { StatusNotice, useConfirmation } from "./Feedback";
import { continuityState } from "./subscriptionContinuity";

type Plan = "free" | "pro_monthly" | "pro_annual";
type SubscriptionStatus = "active" | "past_due" | "canceled" | "expired";
type Freshness = "free" | "verified" | "grace" | "expired";
type AccessKind = "paid" | "complimentary";

type LicensingStatus = {
  connected: boolean;
  email: string | null;
  plan: Plan;
  accessKind: AccessKind | null;
  subscriptionStatus: SubscriptionStatus | null;
  freshness: Freshness;
  paidThroughUtc: number | null;
  refreshAfterUtc: number | null;
  verifiedAtUtc: number | null;
  graceEndsAtUtc: number | null;
  deviceId: string | null;
  activeProjectLimit: number | null;
  isPro: boolean;
};

type AccountActionResult = { status: LicensingStatus; message: string };
type LicensingDevice = { deviceId: string; activatedAtUtc: string; lastSeenAtUtc: string; isCurrent: boolean };
type LicensingDeviceList = { devices: LicensingDevice[]; activeDeviceLimit: number };

const freeStatus: LicensingStatus = {
  connected: false,
  email: null,
  plan: "free",
  accessKind: null,
  subscriptionStatus: null,
  freshness: "free",
  paidThroughUtc: null,
  refreshAfterUtc: null,
  verifiedAtUtc: null,
  graceEndsAtUtc: null,
  deviceId: null,
  activeProjectLimit: 3,
  isPro: false,
};

function formatDate(value: number | null) {
  return value ? new Date(value * 1000).toLocaleDateString(undefined, { year: "numeric", month: "short", day: "numeric" }) : "—";
}

function formatServiceDate(value: string) {
  return new Date(value).toLocaleDateString(undefined, { year: "numeric", month: "short", day: "numeric" });
}

function planLabel(plan: Plan) {
  if (plan === "pro_monthly") return "Pro Monthly";
  if (plan === "pro_annual") return "Pro Annual";
  return "Free";
}

function accessLabel(status: LicensingStatus) {
  return status.accessKind === "complimentary" ? "Complimentary Pro" : planLabel(status.plan);
}

function statusSummary(status: LicensingStatus) {
  if (status.freshness === "grace") {
    const continuity = continuityState(status.verifiedAtUtc, status.graceEndsAtUtc);
    return continuity.daysRemaining === null
      ? "Pro is available from the last verified entitlement."
      : `Pro is available from the last verified entitlement for ${continuity.daysRemaining} more day${continuity.daysRemaining === 1 ? "" : "s"}.`;
  }
  if (status.freshness === "expired") return "SiteDatum is using the Free policy. Existing projects and records remain editable.";
  if (status.accessKind === "complimentary" && status.isPro) return `Complimentary Pro is verified on this computer. Next online verification is due by ${formatDate(status.refreshAfterUtc)}.`;
  if (status.subscriptionStatus === "past_due") return `Payment needs attention. Pro remains available through ${formatDate(status.paidThroughUtc)}.`;
  if (status.subscriptionStatus === "canceled") return `Canceled. Pro remains available through ${formatDate(status.paidThroughUtc)}.`;
  if (status.isPro) return `Verified on this computer. Next online verification is due by ${formatDate(status.refreshAfterUtc)}.`;
  return "Free works without an account and supports up to three active projects.";
}

export function SubscriptionSettings() {
  const [status, setStatus] = useState<LicensingStatus>(freeStatus);
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [working, setWorking] = useState("");
  const [message, setMessage] = useState("");
  const [error, setError] = useState("");
  const [devicesOpen, setDevicesOpen] = useState(false);
  const [devices, setDevices] = useState<LicensingDeviceList | null>(null);
  const { confirmAction, confirmationDialog } = useConfirmation();

  useEffect(() => {
    void invoke<LicensingStatus>("get_licensing_status")
      .then(async (loaded) => {
        setStatus(loaded);
        if (loaded.connected && loaded.refreshAfterUtc && loaded.refreshAfterUtc <= Math.floor(Date.now() / 1000)) {
          try { setStatus((await invoke<AccountActionResult>("refresh_licensing_entitlement")).status); }
          catch { /* Cached access remains authoritative; staged recovery guidance appears below. */ }
        }
      })
      .catch((caught) => setError(describeAppError(caught)));
  }, []);

  async function run<T>(name: string, action: () => Promise<T>, onSuccess: (value: T) => void) {
    setWorking(name);
    setMessage("");
    setError("");
    try { onSuccess(await action()); }
    catch (caught) {
      setError(describeAppError(caught));
      if (typeof caught === "object" && caught !== null && "code" in caught && caught.code === "LICENSING_SESSION_EXPIRED") {
        try {
          setStatus(await invoke<LicensingStatus>("get_licensing_status"));
          setDevicesOpen(false);
          setDevices(null);
        } catch { /* Preserve the original recovery message. */ }
      }
    }
    finally { setWorking(""); }
  }

  function signIn() {
    void run("sign-in", () => invoke<AccountActionResult>("sign_in_licensing", { email, password }), (result) => {
      setStatus(result.status);
      setPassword("");
      setMessage(result.message);
    });
  }

  function createAccount() {
    void run("create-account", () => invoke<string>("create_licensing_account", { email, password }), (result) => {
      setPassword("");
      setMessage(result);
    });
  }

  function refreshEntitlement() {
    void run("refresh", () => invoke<AccountActionResult>("refresh_licensing_entitlement"), (result) => {
      setStatus(result.status);
      setMessage(result.message);
    });
  }

  function openCheckout(plan: "pro_monthly" | "pro_annual") {
    void run(plan, async () => {
      const url = await invoke<string>("get_checkout_url", { plan });
      await openUrl(url);
    }, () => {
      setMessage("Secure Stripe checkout opened in your browser. Return here and refresh after checkout.");
    });
  }

  function openPortal() {
    void run("portal", async () => {
      const url = await invoke<string>("get_billing_portal_url");
      await openUrl(url);
    }, () => {
      setMessage("The Stripe billing portal opened in your browser.");
    });
  }

  function signOut() {
    void run("sign-out", () => invoke<LicensingStatus>("sign_out_licensing"), (result) => {
      setStatus(result);
      setDevicesOpen(false);
      setDevices(null);
      setMessage("Account removed from this computer. Local workspace data was not changed.");
    });
  }

  function toggleDevices() {
    if (devicesOpen) { setDevicesOpen(false); return; }
    setDevicesOpen(true);
    void run("devices", () => invoke<LicensingDeviceList>("list_licensing_devices"), setDevices);
  }

  async function deactivateDevice(device: LicensingDevice) {
    const confirmed = await confirmAction({
      title: device.isCurrent ? "Deactivate this computer?" : "Deactivate this Windows computer?",
      description: device.isCurrent
        ? "This computer will return to Free immediately. Your local projects, records, files, backups, and exports will remain available."
        : "This activation will be retired. The other computer's local data will not be changed, and any cached Pro access ends at its existing verification or paid-through boundary.",
      confirmLabel: "Deactivate computer",
      destructive: true,
    });
    if (!confirmed) return;
    void run(`deactivate-${device.deviceId}`, () => invoke<AccountActionResult>("deactivate_licensing_device", { deviceId: device.deviceId }), async (result) => {
      setStatus(result.status);
      setMessage(result.message);
      try { setDevices(await invoke<LicensingDeviceList>("list_licensing_devices")); }
      catch { setDevices(null); setDevicesOpen(false); }
    });
  }

  const continuity = continuityState(status.verifiedAtUtc, status.graceEndsAtUtc);
  const isComplimentary = status.accessKind === "complimentary";
  const tone = status.freshness === "expired" || status.subscriptionStatus === "past_due" || continuity.stage === "urgent" ? "warning" : "success";
  const showStatusNotice = status.freshness !== "grace"
    ? status.isPro || status.freshness === "expired" || status.subscriptionStatus === "past_due"
    : continuity.stage !== "none";

  return <section className="settings-section subscription-settings" aria-labelledby="subscription-settings-title">
    {confirmationDialog}
    <div className="settings-section-heading">
      <h2 id="subscription-settings-title">Account &amp; Subscription</h2>
      <p>Free stays accountless. Sign in only to buy, verify, or manage SiteDatum Pro.</p>
    </div>

    <div className="subscription-current" aria-live="polite">
      <div>
        <span className="field-label">Current access</span>
        <strong>{accessLabel(status)}</strong>
        <p>{statusSummary(status)}</p>
      </div>
      <dl>
        <div><dt>Account</dt><dd>{status.email ?? "Not connected"}</dd></div>
        <div><dt>Active projects</dt><dd>{status.activeProjectLimit ?? "Unlimited"}</dd></div>
        <div><dt>{isComplimentary ? "Billing" : "Paid through"}</dt><dd>{isComplimentary ? "Not required" : formatDate(status.paidThroughUtc)}</dd></div>
        <div><dt>Verification</dt><dd>{status.freshness === "grace" ? "Offline grace" : status.freshness === "verified" ? "Verified" : status.freshness === "expired" ? "Expired" : "Not required"}</dd></div>
      </dl>
    </div>

    {showStatusNotice &&
      <StatusNotice tone={tone}>{statusSummary(status)}</StatusNotice>}

    {!status.connected ? <form className="licensing-auth" onSubmit={(event) => { event.preventDefault(); signIn(); }}>
      <div className="licensing-auth-fields">
        <label htmlFor="licensing-email">Email address<input id="licensing-email" type="email" autoComplete="email" required value={email} onChange={(event) => setEmail(event.target.value)} /></label>
        <label htmlFor="licensing-password">Password<input id="licensing-password" type="password" autoComplete="current-password" required minLength={12} value={password} onChange={(event) => setPassword(event.target.value)} /></label>
      </div>
      <p className="help">Licensing credentials are stored in Windows Credential Manager and are used only for account and subscription access.</p>
      <div className="actions">
        <button type="submit" disabled={!!working || !email.trim() || password.length < 12}>{working === "sign-in" ? "Signing in…" : "Sign in"}</button>
        <button type="button" className="secondary" onClick={createAccount} disabled={!!working || !email.trim() || password.length < 12}>{working === "create-account" ? "Creating account…" : "Create account"}</button>
      </div>
    </form> : <div className="subscription-actions actions">
      <button type="button" onClick={refreshEntitlement} disabled={!!working}>{working === "refresh" ? "Verifying…" : "Refresh entitlement"}</button>
      {!isComplimentary && (status.isPro || status.subscriptionStatus) && <button type="button" className="secondary" onClick={openPortal} disabled={!!working}>{working === "portal" ? "Opening…" : "Manage billing"}</button>}
      <button type="button" className="secondary" aria-expanded={devicesOpen} aria-controls="licensing-devices" onClick={toggleDevices} disabled={!!working}>{working === "devices" ? "Loading…" : devicesOpen ? "Hide computers" : "Manage computers"}</button>
      <button type="button" className="quiet" onClick={signOut} disabled={!!working}>{working === "sign-out" ? "Signing out…" : "Sign out on this computer"}</button>
    </div>}

    {devicesOpen && <section id="licensing-devices" className="licensing-devices" aria-labelledby="licensing-devices-title">
      <div className="licensing-devices-heading">
        <div><h3 id="licensing-devices-title">Active computers</h3><p>SiteDatum Pro supports two active Windows computers. Device names and workspace information are never sent.</p></div>
        <strong>{devices ? `${devices.devices.length} of ${devices.activeDeviceLimit}` : "—"}</strong>
      </div>
      {devices && (devices.devices.length ? <div className="licensing-devices-table"><table>
        <thead><tr><th>Computer</th><th>Activated</th><th>Last verified</th><th aria-label="Computer action" /></tr></thead>
        <tbody>{devices.devices.map((device) => <tr key={device.deviceId}>
          <th scope="row">{device.isCurrent ? "This computer" : "Windows computer"}</th>
          <td>{formatServiceDate(device.activatedAtUtc)}</td><td>{formatServiceDate(device.lastSeenAtUtc)}</td>
          <td><button type="button" className="quiet danger-text" disabled={!!working} onClick={() => void deactivateDevice(device)}>{working === `deactivate-${device.deviceId}` ? "Deactivating…" : "Deactivate"}</button></td>
        </tr>)}</tbody>
      </table></div> : <p className="empty-state">No active Pro computers are registered to this account.</p>)}
    </section>}

    <div className="status-area" aria-live="polite">{message && <StatusNotice tone="success">{message}</StatusNotice>}{error && <StatusNotice tone="error">{error}</StatusNotice>}</div>

    <div className="plan-comparison">
      <table>
        <thead><tr><th>Plan</th><th>Active projects</th><th>Included workflow</th><th>Price</th><th aria-label="Plan action" /></tr></thead>
        <tbody>
          <tr><th scope="row">Free</th><td>3</td><td>Complete manual workflow, backup, and CSV export</td><td>$0</td><td>{!status.isPro && <span className="current-plan-label">Current</span>}</td></tr>
          <tr><th scope="row">Pro Monthly</th><td>Unlimited</td><td>Templates, bulk operations, Excel interchange, professional reports</td><td>$15/month</td><td>{isComplimentary ? <span className="current-plan-label">Included</span> : <button type="button" className="secondary" disabled={!status.connected || !!working || status.plan === "pro_monthly"} onClick={() => openCheckout("pro_monthly")}>{working === "pro_monthly" ? "Opening…" : status.plan === "pro_monthly" ? "Current" : "Choose monthly"}</button>}</td></tr>
          <tr><th scope="row">Pro Annual</th><td>Unlimited</td><td>Same Pro capabilities; two months saved</td><td>$150/year</td><td>{isComplimentary ? <span className="current-plan-label">Included</span> : <button type="button" className="secondary" disabled={!status.connected || !!working || status.plan === "pro_annual"} onClick={() => openCheckout("pro_annual")}>{working === "pro_annual" ? "Opening…" : status.plan === "pro_annual" ? "Current" : "Choose annual"}</button>}</td></tr>
        </tbody>
      </table>
      {!status.connected && <p className="help">Sign in or create an account to open secure checkout. Ordinary Free use does not require either.</p>}
    </div>

    <p className="subscription-assurance">Expiration never deletes, hides, or locks existing project records. Local backup and essential export remain available.</p>
  </section>;
}

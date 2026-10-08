import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { openUrl } from "@tauri-apps/plugin-opener";
import { describeAppError } from "./error";
import { StatusNotice } from "./Feedback";

type Plan = "free" | "pro_monthly" | "pro_annual";
type SubscriptionStatus = "active" | "past_due" | "canceled" | "expired";
type Freshness = "free" | "verified" | "grace" | "expired";

type LicensingStatus = {
  connected: boolean;
  email: string | null;
  plan: Plan;
  subscriptionStatus: SubscriptionStatus | null;
  freshness: Freshness;
  paidThroughUtc: number | null;
  refreshAfterUtc: number | null;
  deviceId: string | null;
  activeProjectLimit: number | null;
  isPro: boolean;
};

type AccountActionResult = { status: LicensingStatus; message: string };

const freeStatus: LicensingStatus = {
  connected: false,
  email: null,
  plan: "free",
  subscriptionStatus: null,
  freshness: "free",
  paidThroughUtc: null,
  refreshAfterUtc: null,
  deviceId: null,
  activeProjectLimit: 3,
  isPro: false,
};

function formatDate(value: number | null) {
  return value ? new Date(value * 1000).toLocaleDateString(undefined, { year: "numeric", month: "short", day: "numeric" }) : "—";
}

function planLabel(plan: Plan) {
  if (plan === "pro_monthly") return "Pro Monthly";
  if (plan === "pro_annual") return "Pro Annual";
  return "Free";
}

function statusSummary(status: LicensingStatus) {
  if (status.freshness === "grace") return "Pro is available from the last verified entitlement. Connect before the 21-day verification window ends.";
  if (status.freshness === "expired") return "SiteDatum is using the Free policy. Existing projects and records remain editable.";
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

  useEffect(() => {
    void invoke<LicensingStatus>("get_licensing_status")
      .then(setStatus)
      .catch((caught) => setError(describeAppError(caught)));
  }, []);

  async function run<T>(name: string, action: () => Promise<T>, onSuccess: (value: T) => void) {
    setWorking(name);
    setMessage("");
    setError("");
    try { onSuccess(await action()); }
    catch (caught) { setError(describeAppError(caught)); }
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
      setMessage("Account removed from this computer. Local workspace data was not changed.");
    });
  }

  const tone = status.freshness === "expired" || status.subscriptionStatus === "past_due" ? "warning" : "success";

  return <section className="settings-section subscription-settings" aria-labelledby="subscription-settings-title">
    <div className="settings-section-heading">
      <h2 id="subscription-settings-title">Account &amp; Subscription</h2>
      <p>Free stays accountless. Sign in only to buy, verify, or manage SiteDatum Pro.</p>
    </div>

    <div className="subscription-current" aria-live="polite">
      <div>
        <span className="field-label">Current access</span>
        <strong>{planLabel(status.plan)}</strong>
        <p>{statusSummary(status)}</p>
      </div>
      <dl>
        <div><dt>Account</dt><dd>{status.email ?? "Not connected"}</dd></div>
        <div><dt>Active projects</dt><dd>{status.activeProjectLimit ?? "Unlimited"}</dd></div>
        <div><dt>Paid through</dt><dd>{formatDate(status.paidThroughUtc)}</dd></div>
        <div><dt>Verification</dt><dd>{status.freshness === "grace" ? "Offline grace" : status.freshness === "verified" ? "Verified" : status.freshness === "expired" ? "Expired" : "Not required"}</dd></div>
      </dl>
    </div>

    {(status.isPro || status.freshness === "grace" || status.freshness === "expired" || status.subscriptionStatus === "past_due") &&
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
      {(status.isPro || status.subscriptionStatus) && <button type="button" className="secondary" onClick={openPortal} disabled={!!working}>{working === "portal" ? "Opening…" : "Manage billing"}</button>}
      <button type="button" className="quiet" onClick={signOut} disabled={!!working}>{working === "sign-out" ? "Signing out…" : "Sign out on this computer"}</button>
    </div>}

    <div className="plan-comparison">
      <table>
        <thead><tr><th>Plan</th><th>Active projects</th><th>Included workflow</th><th>Price</th><th aria-label="Plan action" /></tr></thead>
        <tbody>
          <tr><th scope="row">Free</th><td>3</td><td>Complete manual workflow, backup, and CSV export</td><td>$0</td><td>{!status.isPro && <span className="current-plan-label">Current</span>}</td></tr>
          <tr><th scope="row">Pro Monthly</th><td>Unlimited</td><td>Templates, bulk operations, Excel interchange, professional reports</td><td>$15/month</td><td><button type="button" className="secondary" disabled={!status.connected || !!working || status.plan === "pro_monthly"} onClick={() => openCheckout("pro_monthly")}>{working === "pro_monthly" ? "Opening…" : status.plan === "pro_monthly" ? "Current" : "Choose monthly"}</button></td></tr>
          <tr><th scope="row">Pro Annual</th><td>Unlimited</td><td>Same Pro capabilities; two months saved</td><td>$150/year</td><td><button type="button" className="secondary" disabled={!status.connected || !!working || status.plan === "pro_annual"} onClick={() => openCheckout("pro_annual")}>{working === "pro_annual" ? "Opening…" : status.plan === "pro_annual" ? "Current" : "Choose annual"}</button></td></tr>
        </tbody>
      </table>
      {!status.connected && <p className="help">Sign in or create an account to open secure checkout. Ordinary Free use does not require either.</p>}
    </div>

    <p className="subscription-assurance">Expiration never deletes, hides, or locks existing project records. Local backup and essential export remain available.</p>
    <div className="status-area" aria-live="polite">{message && <StatusNotice tone="success">{message}</StatusNotice>}{error && <StatusNotice tone="error">{error}</StatusNotice>}</div>
  </section>;
}

import appIcon from "./assets/branding/site-datum-app-icon-v2.png";

/** One lockup for navigation, onboarding, and About; no clipped bitmap text. */
export function Brand({ className = "" }: { className?: string }) {
  return <span className={`brand-lockup ${className}`} role="img" aria-label="SiteDatum">
    <img className="brand-mark" src={appIcon} alt="" aria-hidden="true" />
    <span className="brand-name" aria-hidden="true"><span className="brand-name-site">Site</span><span className="brand-name-datum">Datum</span></span>
  </span>;
}

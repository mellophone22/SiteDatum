import type { Screen } from "./workspaceState";

type HelpProps = { onNavigate: (screen: Screen) => void };

export function Help({ onNavigate }: HelpProps) {
  return <section className="projects-page help-page" aria-labelledby="help-title">
    <div className="page-toolbar"><div><p className="eyebrow">Local workspace guide</p><h1 id="help-title">Using SiteDatum</h1><p className="intro">A concise guide to where work belongs, how records relate, and what stays on this computer.</p></div></div>

    <nav className="help-jump-links" aria-label="Help topics">
      <a href="#help-start">Start</a><a href="#help-daily">Daily work</a><a href="#help-files">Files</a><a href="#help-safety">Data safety</a><a href="#help-plans">Free &amp; Pro</a>
    </nav>

    <div className="help-sections">
      <section id="help-start"><p className="eyebrow">01 · Establish context</p><h2>Start with a project workspace</h2><p>Choose an existing project root in Settings, then create a project. SiteDatum creates a normal Windows folder beneath that root and keeps the selected project visible while you move between registers.</p><div className="actions"><button type="button" onClick={() => onNavigate("projects")}>Open Projects</button><button type="button" className="secondary" onClick={() => onNavigate("settings")}>Workspace settings</button></div></section>
      <section id="help-daily"><p className="eyebrow">02 · Preserve decisions</p><h2>Use the register that matches the work</h2><dl className="help-register-guide"><div><dt>Tasks</dt><dd>Personal commitments, dates, waiting items, and follow-ups.</dd></div><div><dt>RFIs</dt><dd>Questions that require a documented answer and response history.</dd></div><div><dt>Submittals</dt><dd>Packages, revisions, review status, and dispositions.</dd></div><div><dt>Project Controls</dt><dd>Meetings, changes, procurement, transmittals, milestones, field work, and commissioning.</dd></div></dl><div className="actions"><button type="button" onClick={() => onNavigate("attention")}>Open Attention</button></div></section>
      <section id="help-files"><p className="eyebrow">03 · Keep documents normal</p><h2>Files remain Windows files</h2><p><strong>Register</strong> keeps a reference to the existing file. <strong>Copy</strong> places a new copy in the project. <strong>Move</strong> relocates the original only after confirmation. SiteDatum never silently overwrites a destination.</p><div className="actions"><button type="button" onClick={() => onNavigate("files")}>Open Files</button></div></section>
      <section id="help-safety"><p className="eyebrow">04 · Recover deliberately</p><h2>Backups and exports are separate safeguards</h2><p>A database backup protects the SiteDatum record. A complete CSV export provides portable metadata. Referenced documents stay in their Windows folders and must be protected by your normal file-backup process.</p><div className="actions"><button type="button" onClick={() => onNavigate("recovery")}>Open Recovery</button></div></section>
      <section id="help-plans"><p className="eyebrow">05 · Commercial boundary</p><h2>Free stays accountless; Pro accelerates work</h2><p>Free supports up to three active projects without an account. Pro adds unlimited projects, templates, bulk operations, Excel interchange, professional reports, and customer-owned RFI PDF templates. Billing and licensing receive no project names, records, documents, contacts, or file paths.</p><div className="actions"><button type="button" onClick={() => onNavigate("settings")}>Account &amp; Subscription</button></div></section>
    </div>

    <aside className="help-support" aria-labelledby="help-support-title"><div><p className="eyebrow">Support</p><h2 id="help-support-title">When something does not work</h2></div><p>Keep the on-screen reference number and describe what you were doing. Do not send project documents or database contents unless you deliberately choose to.</p><p><strong>supportsitedatum@protonmail.com</strong></p></aside>
  </section>;
}

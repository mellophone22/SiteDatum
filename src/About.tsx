import { useEffect, useState } from "react";
import { Brand } from "./Brand";
import { readAppVersion } from "./appVersion";

export function About() {
  const [version, setVersion] = useState("Reading version…");
  useEffect(() => {
    let active = true;
    void readAppVersion().then(value => { if (active) setVersion(`Version ${value}`); })
      .catch(() => { if (active) setVersion("Version unavailable"); });
    return () => { active = false; };
  }, []);

  return <section className="projects-page about-page" aria-labelledby="about-title">
    <div className="page-toolbar"><div><p className="eyebrow">Behind the workspace</p><h1 id="about-title">About SiteDatum</h1></div></div>
    <div className="about-sheet">
      <div className="about-identity">
        <span className="about-label">Project Engineer workspace</span>
        <Brand className="about-brand" />
        <p className="about-statement">The project record<br />you control</p>
        <div className="about-registration" aria-hidden="true"><span>SD</span><i /><span>LOCAL / FIRST</span></div>
      </div>
      <div className="about-details">
        <p className="about-purpose">Built for the work<br />between the drawings.</p>
        <p className="about-description">Projects, decisions, documents, and follow-ups in one deliberate workspace. A clear record from the first question to final closeout.</p>
        <dl className="about-facts">
          <div><dt>Creator</dt><dd>Created by Francisco Cabrera</dd></div>
          <div><dt>Software</dt><dd aria-live="polite">{version}</dd></div>
          <div><dt>Foundation</dt><dd>Local-first. Windows-native.</dd></div>
        </dl>
        <p className="about-ownership">Your documents remain normal Windows files.<br />Your project record stays in your control.</p>
      </div>
    </div>
  </section>;
}

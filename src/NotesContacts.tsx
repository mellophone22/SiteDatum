import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { describeAppError } from "./error";
import { takeFocus } from "./focus";
import { useProjectContext } from "./projectContext";

type Project = { id: string; number: string; name: string };
type Note = { id: string; projectId: string | null; projectNumber: string | null; projectName: string | null; body: string; createdAtUtc: string };
type Contact = { id: string; name: string; company: string | null; email: string | null; phone: string | null; role: string | null };
type Event = { eventType: string; summary: string; occurredAtUtc: string };

export function NotesContacts() {
  const projectContext = useProjectContext();
  const [notes, setNotes] = useState<Note[]>([]);
  const [contacts, setContacts] = useState<Contact[]>([]);
  const [events, setEvents] = useState<Event[]>([]);
  const [projects, setProjects] = useState<Project[]>([]);
  const [noteId, setNoteId] = useState<string | null>(null);
  const [contactId, setContactId] = useState<string | null>(null);
  const [body, setBody] = useState("");
  const [projectId, setProjectId] = useState("");
  const [name, setName] = useState("");
  const [company, setCompany] = useState("");
  const [email, setEmail] = useState("");
  const [phone, setPhone] = useState("");
  const [role, setRole] = useState("");
  const [error, setError] = useState("");

  const load = async () => {
    try {
      const [noteList, contactList, activityList, projectList] = await Promise.all([
        invoke<Note[]>("list_notes"), invoke<Contact[]>("list_contacts"), invoke<Event[]>("list_activity"), invoke<Project[]>("list_projects", { includeArchived: false }),
      ]);
      setNotes(noteList); setContacts(contactList); setEvents(activityList); setProjects(projectList); setError("");
    } catch (caught) { setError(describeAppError(caught)); }
  };

  useEffect(() => { void load(); }, []);
  useEffect(() => { if (!noteId) setProjectId(projectContext); }, [projectContext, noteId]);
  useEffect(() => {
    const handler = () => { setNoteId(null); setBody(""); setTimeout(() => document.querySelector<HTMLTextAreaElement>("#note-body")?.focus(), 0); };
    window.addEventListener("workspace:new", handler);
    return () => window.removeEventListener("workspace:new", handler);
  }, []);
  useEffect(() => {
    if (!notes.length && !contacts.length) return;
    const id = takeFocus("notes");
    const note = notes.find((value) => value.id === id);
    if (note) { setNoteId(note.id); setBody(note.body); setProjectId(note.projectId ?? ""); return; }
    const contact = contacts.find((value) => value.id === id);
    if (contact) { setContactId(contact.id); setName(contact.name); setCompany(contact.company ?? ""); setEmail(contact.email ?? ""); setPhone(contact.phone ?? ""); setRole(contact.role ?? ""); }
  }, [notes, contacts]);

  const clearNote = () => { setNoteId(null); setBody(""); setProjectId(""); };
  const clearContact = () => { setContactId(null); setName(""); setCompany(""); setEmail(""); setPhone(""); setRole(""); };

  async function saveNote(event: React.FormEvent) {
    event.preventDefault(); const input = { projectId: projectId || null, body };
    try { if (noteId) await invoke("update_note", { id: noteId, input }); else await invoke("create_note", { input }); clearNote(); await load(); }
    catch (caught) { setError(describeAppError(caught)); }
  }
  async function saveContact(event: React.FormEvent) {
    event.preventDefault(); const input = { name, company: company || null, email: email || null, phone: phone || null, role: role || null };
    try { if (contactId) await invoke("update_contact", { id: contactId, input }); else await invoke("create_contact", { input }); clearContact(); await load(); }
    catch (caught) { setError(describeAppError(caught)); }
  }

  return <section className="projects-page" aria-labelledby="notes-title">
    <div className="page-toolbar"><div><h1 id="notes-title">Notes, contacts, and activity</h1><p className="page-description">Keep project context and the people connected to it in one place.</p></div></div>
    {error && <p className="status error" role="alert">{error}</p>}
    <div className="context-layout">
      <div className="context-main">
        <form className="create-panel note-form" onSubmit={saveNote}>
          <h2>{noteId ? "Edit note" : "New note"}</h2>
          <div className="form-stack">
            <label htmlFor="note-project">Project <span className="optional">(optional)</span></label>
            <select id="note-project" value={projectId} onChange={(event) => setProjectId(event.target.value)}><option value="">General note</option>{projects.map((project) => <option key={project.id} value={project.id}>{project.number} — {project.name}</option>)}</select>
            <label htmlFor="note-body">Note</label>
            <textarea id="note-body" required value={body} onChange={(event) => setBody(event.target.value)} />
          </div>
          <div className="actions"><button>{noteId ? "Save changes" : "Save note"}</button>{noteId && <button type="button" className="secondary" onClick={clearNote}>Cancel</button>}</div>
        </form>
        <section aria-labelledby="saved-notes-title">
          <h2 id="saved-notes-title" className="group-heading">Saved notes <span>{notes.length}</span></h2>
          <div className="table-scroll"><table><thead><tr><th>Note</th><th>Project</th><th>Actions</th></tr></thead><tbody>{notes.filter((note)=>!projectContext||note.projectId===projectContext).map((note) => <tr key={note.id}><td className="note-body-cell">{note.body}</td><td>{note.projectNumber ? `${note.projectNumber} — ${note.projectName}` : "General"}</td><td className="row-actions"><button className="quiet" onClick={() => { setNoteId(note.id); setBody(note.body); setProjectId(note.projectId ?? ""); }}>Edit</button><button className="quiet" onClick={() => { if (confirm("Remove this note permanently?")) void invoke("delete_note", { id: note.id }).then(load); }}>Remove</button></td></tr>)}{!notes.filter((note)=>!projectContext||note.projectId===projectContext).length && <tr><td colSpan={3} className="empty">No notes in this project context.</td></tr>}</tbody></table></div>
        </section>
        <section className="attention-section" aria-labelledby="activity-title"><h2 id="activity-title" className="group-heading">Recent activity</h2><ul className="attachment-list">{events.map((event, index) => <li key={index}><span>{event.summary}<small>{new Date(event.occurredAtUtc).toLocaleString()}</small></span></li>)}</ul></section>
      </div>
      <aside className="rfi-detail contacts-panel" aria-labelledby="contacts-title">
        <form className="detail-form" onSubmit={saveContact}><h2 id="contacts-title">{contactId ? "Edit contact" : "New contact"}</h2><div className="form-grid"><label className="wide-field">Name<input required value={name} onChange={(event) => setName(event.target.value)} /></label><label>Company<input value={company} onChange={(event) => setCompany(event.target.value)} /></label><label>Role<input value={role} onChange={(event) => setRole(event.target.value)} /></label><label>Email<input type="email" value={email} onChange={(event) => setEmail(event.target.value)} /></label><label>Phone<input value={phone} onChange={(event) => setPhone(event.target.value)} /></label></div><div className="actions"><button>{contactId ? "Save changes" : "Save contact"}</button>{contactId && <button type="button" className="secondary" onClick={clearContact}>Cancel</button>}</div></form>
        <h3 className="group-heading contacts-heading">Contacts <span>{contacts.length}</span></h3>
        {contacts.length ? <ul className="attachment-list">{contacts.map((contact) => <li key={contact.id}><span><strong>{contact.name}</strong><small>{[contact.company, contact.role, contact.email, contact.phone].filter(Boolean).join(" · ")}</small></span><span className="attachment-actions"><button className="quiet" onClick={() => { setContactId(contact.id); setName(contact.name); setCompany(contact.company ?? ""); setEmail(contact.email ?? ""); setPhone(contact.phone ?? ""); setRole(contact.role ?? ""); }}>Edit</button><button className="quiet" onClick={() => { if (confirm(`Remove contact ${contact.name} permanently?`)) void invoke("delete_contact", { id: contact.id }).then(load); }}>Remove</button></span></li>)}</ul> : <p className="empty-state">No contacts yet.</p>}
      </aside>
    </div>
  </section>;
}

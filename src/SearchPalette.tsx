import { useEffect, useMemo, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

type Raw = { id: string; number?: string; name?: string; title?: string; subject?: string; fileName?: string; body?: string; company?: string | null };
type Item = { id: string; label: string; detail: string; screen: string };

export function SearchPalette({ open, onClose, onNavigate }: { open: boolean; onClose: () => void; onNavigate: (screen: string) => void }) {
  const [query, setQuery] = useState("");
  const [items, setItems] = useState<Item[]>([]);
  const [active, setActive] = useState(0);
  const [loading, setLoading] = useState(false);
  const [loadError, setLoadError] = useState("");
  const dialogRef = useRef<HTMLElement>(null);
  const previousFocus = useRef<HTMLElement | null>(null);

  useEffect(() => {
    if (!open) return;
    previousFocus.current = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    setQuery(""); setActive(0); setLoading(true); setLoadError("");
    void Promise.all([invoke<Raw[]>("list_projects", { includeArchived: true }), invoke<Raw[]>("list_tasks"), invoke<Raw[]>("list_rfis"), invoke<Raw[]>("list_submittals"), invoke<Raw[]>("list_files"), invoke<Raw[]>("list_notes"), invoke<Raw[]>("list_contacts")])
      .then(([projects, tasks, rfis, submittals, files, notes, contacts]) => setItems([
        ...projects.map((item) => ({ id: item.id, label: `${item.number} — ${item.name}`, detail: "Project", screen: "projects" })),
        ...tasks.map((item) => ({ id: item.id, label: item.title ?? "Task", detail: "Task", screen: "tasks" })),
        ...rfis.map((item) => ({ id: item.id, label: `${item.number} — ${item.subject}`, detail: "RFI", screen: "rfis" })),
        ...submittals.map((item) => ({ id: item.id, label: `${item.number} — ${item.name}`, detail: "Submittal", screen: "submittals" })),
        ...files.map((item) => ({ id: item.id, label: item.fileName ?? "File", detail: "File", screen: "files" })),
        ...notes.map((item) => ({ id: item.id, label: item.body ?? "Note", detail: "Note", screen: "notes" })),
        ...contacts.map((item) => ({ id: item.id, label: item.name ?? "Contact", detail: item.company ?? "Contact", screen: "notes" })),
      ])).catch(() => { setItems([]); setLoadError("Search is unavailable. Close this window and try again."); }).finally(() => setLoading(false));
    return () => { previousFocus.current?.focus(); };
  }, [open]);

  const results = useMemo(() => items.filter((item) => `${item.label} ${item.detail}`.toLocaleLowerCase().includes(query.toLocaleLowerCase())).slice(0, 30), [items, query]);
  const choose = (item: Item) => { localStorage.setItem("workspace.focus", JSON.stringify({ screen: item.screen, id: item.id })); onNavigate(item.screen); onClose(); };
  const trapFocus = (event: React.KeyboardEvent) => {
    if (event.key !== "Tab" || !dialogRef.current) return;
    const controls = Array.from(dialogRef.current.querySelectorAll<HTMLElement>("button:not(:disabled), input:not(:disabled)"));
    if (!controls.length) return;
    const first = controls[0]; const last = controls[controls.length - 1];
    if (event.shiftKey && document.activeElement === first) { event.preventDefault(); last.focus(); }
    else if (!event.shiftKey && document.activeElement === last) { event.preventDefault(); first.focus(); }
  };

  if (!open) return null;
  return <div className="palette-backdrop" onMouseDown={onClose}>
    <section ref={dialogRef} className="palette" role="dialog" aria-modal="true" aria-labelledby="search-title" onMouseDown={(event) => event.stopPropagation()} onKeyDown={trapFocus}>
      <div className="palette-heading"><div><h2 id="search-title">Search workspace</h2><p>Open a project record without losing its context.</p></div><button type="button" className="quiet" onClick={onClose}>Close</button></div>
      <input autoFocus aria-label="Search local records" value={query} onChange={(event) => { setQuery(event.target.value); setActive(0); }} onKeyDown={(event) => { if (event.key === "Escape") onClose(); if (event.key === "ArrowDown") { event.preventDefault(); setActive((value) => Math.min(value + 1, results.length - 1)); } if (event.key === "ArrowUp") { event.preventDefault(); setActive((value) => Math.max(value - 1, 0)); } if (event.key === "Enter" && results[active]) choose(results[active]); }} placeholder="Project, task, RFI, file, note, or contact" aria-controls="search-results" aria-activedescendant={results[active] ? `search-result-${active}` : undefined} />
      <p className="palette-help"><kbd>↑</kbd><kbd>↓</kbd> select · <kbd>Enter</kbd> open · <kbd>Esc</kbd> close</p>
      {loadError ? <p className="status error" role="alert">{loadError}</p> : null}<ul id="search-results" role="listbox" aria-label="Search results">{loading ? <li className="empty" role="status">Searching local records…</li> : results.map((item, index) => <li key={`${item.screen}-${item.id}`} id={`search-result-${index}`} role="option" aria-selected={index === active}><button className={index === active ? "quiet selected-search" : "quiet"} onMouseEnter={() => setActive(index)} onClick={() => choose(item)}><strong>{item.label}</strong><small>{item.detail}</small></button></li>)}{!loading && !loadError && query && !results.length ? <li className="empty">No matching local records.</li> : null}</ul>
    </section>
  </div>;
}

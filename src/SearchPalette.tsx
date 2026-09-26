import { useEffect,useMemo,useRef,useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { paletteItemKey,paletteResults,type PaletteItem } from "./searchPaletteState";

type Raw={id:string;projectId?:string|null;projectNumber?:string;projectName?:string;number?:string;name?:string;title?:string;subject?:string;fileName?:string;body?:string;company?:string|null};
export type SearchItem=PaletteItem;

const projectDetail=(kind:string,item:Raw)=>item.projectNumber&&item.projectName?`${kind} · ${item.projectNumber} — ${item.projectName}`:kind;
const readList=(key:string)=>{try{const value=JSON.parse(localStorage.getItem(key)??"[]");return Array.isArray(value)?value.filter((item):item is string=>typeof item==="string"):[]}catch{return []}};

export function SearchPalette({open,onClose,onOpen}:{open:boolean;onClose:()=>void;onOpen:(item:SearchItem)=>void}){
 const [query,setQuery]=useState(""); const [items,setItems]=useState<SearchItem[]>([]); const [recentKeys,setRecentKeys]=useState<string[]>([]); const [recentProjectIds,setRecentProjectIds]=useState<string[]>([]); const [active,setActive]=useState(0); const [loading,setLoading]=useState(false); const [loadError,setLoadError]=useState("");
 const dialogRef=useRef<HTMLElement>(null); const previousFocus=useRef<HTMLElement|null>(null);
 useEffect(()=>{if(!open)return;previousFocus.current=document.activeElement instanceof HTMLElement?document.activeElement:null;setQuery("");setActive(0);setLoading(true);setLoadError("");setRecentKeys(readList("search.recent"));setRecentProjectIds(readList("projects.recent"));
  void Promise.all([invoke<Raw[]>("list_projects",{includeArchived:true}),invoke<Raw[]>("list_tasks"),invoke<Raw[]>("list_rfis"),invoke<Raw[]>("list_submittals"),invoke<Raw[]>("list_files"),invoke<Raw[]>("list_notes"),invoke<Raw[]>("list_contacts")]).then(([projects,tasks,rfis,submittals,files,notes,contacts])=>setItems([
   ...projects.map(item=>({id:item.id,projectId:item.id,label:`${item.number} — ${item.name}`,detail:"Project workspace",group:"Projects",type:"project",screen:"projects" as const,kind:"project" as const})),
   ...tasks.map(item=>({id:item.id,projectId:item.projectId,label:item.title??"Task",detail:projectDetail("Task",item),group:"Tasks",type:"task",screen:"tasks" as const,kind:"record" as const})),
   ...rfis.map(item=>({id:item.id,projectId:item.projectId,label:`${item.number} — ${item.subject}`,detail:projectDetail("RFI",item),group:"RFIs",type:"rfi",screen:"rfis" as const,kind:"record" as const})),
   ...submittals.map(item=>({id:item.id,projectId:item.projectId,label:`${item.number} — ${item.name}`,detail:projectDetail("Submittal",item),group:"Submittals",type:"submittal",screen:"submittals" as const,kind:"record" as const})),
   ...files.map(item=>({id:item.id,projectId:item.projectId,label:item.fileName??"File",detail:projectDetail("File",item),group:"Files",type:"file",screen:"files" as const,kind:"record" as const})),
   ...notes.map(item=>({id:item.id,projectId:item.projectId,label:item.body??"Note",detail:projectDetail("Note",item),group:"Notes",type:"note",screen:"notes" as const,kind:"record" as const})),
   ...contacts.map(item=>({id:item.id,label:item.name??"Contact",detail:item.company?`Contact · ${item.company}`:"Contact",group:"Contacts",type:"contact",screen:"notes" as const,kind:"record" as const})),
  ])).catch(()=>{setItems([]);setLoadError("Search is unavailable. Close this window and try again.")}).finally(()=>setLoading(false));
  return()=>{previousFocus.current?.focus()};
 },[open]);
 const results=useMemo(()=>paletteResults(items,query,recentKeys,recentProjectIds),[items,query,recentKeys,recentProjectIds]);
 const choose=(item:SearchItem)=>{if(item.kind==="record"){const next=[paletteItemKey(item),...recentKeys.filter(key=>key!==paletteItemKey(item))].slice(0,12);localStorage.setItem("search.recent",JSON.stringify(next));setRecentKeys(next)}else if(item.kind==="project"){const next=[item.id,...recentProjectIds.filter(id=>id!==item.id)].slice(0,8);localStorage.setItem("projects.recent",JSON.stringify(next));setRecentProjectIds(next)}onOpen(item);onClose()};
 const trapFocus=(event:React.KeyboardEvent)=>{if(event.key!=="Tab"||!dialogRef.current)return;const controls=Array.from(dialogRef.current.querySelectorAll<HTMLElement>("button:not(:disabled), input:not(:disabled)"));if(!controls.length)return;const first=controls[0],last=controls[controls.length-1];if(event.shiftKey&&document.activeElement===first){event.preventDefault();last.focus()}else if(!event.shiftKey&&document.activeElement===last){event.preventDefault();first.focus()}};
 if(!open)return null;
 return <div className="palette-backdrop" onMouseDown={onClose}><section ref={dialogRef} className="palette" role="dialog" aria-modal="true" aria-labelledby="search-title" onMouseDown={event=>event.stopPropagation()} onKeyDown={trapFocus}>
  <div className="palette-heading"><div><h2 id="search-title">Search and open</h2><p>{query?"Results include their record type and project context.":"Recent work and workspace actions."}</p></div><button type="button" className="quiet" onClick={onClose}>Close</button></div>
  <input autoFocus aria-label="Search local records" value={query} onChange={event=>{setQuery(event.target.value);setActive(0)}} onKeyDown={event=>{if(event.key==="Escape")onClose();if(event.key==="ArrowDown"){event.preventDefault();setActive(value=>results.length?Math.min(value+1,results.length-1):0)}if(event.key==="ArrowUp"){event.preventDefault();setActive(value=>Math.max(value-1,0))}if(event.key==="Enter"&&results[active])choose(results[active])}} placeholder="Project, task, RFI, submittal, file, note, or contact" aria-controls="search-results" aria-activedescendant={results[active]?`search-result-${active}`:undefined}/>
  <p className="palette-help"><kbd>↑</kbd><kbd>↓</kbd> select · <kbd>Enter</kbd> open · <kbd>Esc</kbd> close</p>
  {loadError&&<p className="status error" role="alert">{loadError}</p>}<ul id="search-results" role="listbox" aria-label={query?"Search results":"Recent items and actions"}>{loading?<li className="empty" role="status">Loading local workspace…</li>:results.map((item,index)=><li key={`${item.kind}-${item.screen}-${item.id}`} role="presentation">{index===0||results[index-1].group!==item.group?<span className="palette-group">{item.group}</span>:null}<button id={`search-result-${index}`} role="option" aria-selected={index===active} className={index===active?"quiet selected-search":"quiet"} onMouseEnter={()=>setActive(index)} onClick={()=>choose(item)}><strong>{item.label}</strong><small>{item.detail}</small></button></li>)}{!loading&&!loadError&&query&&!results.length?<li className="empty">No matching local records.</li>:null}</ul>
 </section></div>;
}

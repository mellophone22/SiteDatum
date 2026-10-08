import type { Screen } from "./workspaceState";

export type PaletteItem = { id:string; projectId?:string|null; label:string; detail:string; group:string; type:string; screen:Screen; kind:"record"|"project"|"action" };

export const paletteActions:PaletteItem[]=[
  {id:"action-home",label:"Open Home",detail:"Workspace overview",group:"Actions",type:"action",screen:"overview",kind:"action"},
  {id:"action-attention",label:"Open Attention",detail:"Due, waiting, and follow-up work",group:"Actions",type:"action",screen:"attention",kind:"action"},
  {id:"action-projects",label:"Open Projects",detail:"Project workspaces",group:"Actions",type:"action",screen:"projects",kind:"action"},
  {id:"action-controls",label:"Open Project Controls",detail:"Operational registers",group:"Actions",type:"action",screen:"operations",kind:"action"},
  {id:"action-help",label:"Open Help",detail:"Workflow, files, backups, and plans",group:"Actions",type:"action",screen:"help",kind:"action"},
  {id:"action-about",label:"About SiteDatum",detail:"Software version and creator",group:"Actions",type:"action",screen:"about",kind:"action"},
];

export const paletteItemKey=(item:PaletteItem)=>`${item.screen}:${item.id}`;

export function paletteResults(items:PaletteItem[],query:string,recentKeys:string[],recentProjectIds:string[]):PaletteItem[]{
  const term=query.trim().toLocaleLowerCase();
  if(term)return [...paletteActions,...items].filter(item=>`${item.label} ${item.detail} ${item.group}`.toLocaleLowerCase().includes(term)).slice(0,30);
  const recentRecords=recentKeys.map(key=>items.find(item=>item.kind==="record"&&paletteItemKey(item)===key)).filter((item):item is PaletteItem=>Boolean(item)).slice(0,8).map(item=>({...item,group:"Recent records"}));
  const recentProjects=recentProjectIds.map(id=>items.find(item=>item.kind==="project"&&item.id===id)).filter((item):item is PaletteItem=>Boolean(item)).slice(0,6).map(item=>({...item,group:"Recent projects"}));
  const fallbackProjects=recentProjects.length?[]:items.filter(item=>item.kind==="project").slice(0,5).map(item=>({...item,group:"Projects"}));
  return [...paletteActions,...recentRecords,...recentProjects,...fallbackProjects];
}

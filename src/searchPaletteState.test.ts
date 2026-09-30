import { describe,expect,it } from "vitest";
import { paletteItemKey,paletteResults,type PaletteItem } from "./searchPaletteState";

const items:PaletteItem[]=[
 {id:"p1",projectId:"p1",label:"101 — School",detail:"Project",group:"Projects",type:"project",screen:"projects",kind:"project"},
 {id:"t1",projectId:"p1",label:"Review controls",detail:"Task · 101 — School",group:"Tasks",type:"task",screen:"tasks",kind:"record"},
 {id:"r1",projectId:"p1",label:"RFI-1 — Sequence",detail:"RFI · 101 — School",group:"RFIs",type:"rfi",screen:"rfis",kind:"record"},
];

describe("command palette state",()=>{
 it("finds About by product name and software version",()=>{expect(paletteResults(items,"sitedatum",[],[]).map(item=>item.screen)).toEqual(["about"]);expect(paletteResults(items,"version",[],[]).map(item=>item.screen)).toEqual(["about"])});
 it("shows implemented actions and real recent content for a blank query",()=>{const results=paletteResults(items,"",[paletteItemKey(items[1])],["p1"]);expect(results.some(item=>item.group==="Actions")).toBe(true);expect(results.find(item=>item.id==="t1")?.group).toBe("Recent records");expect(results.find(item=>item.id==="p1")?.group).toBe("Recent projects")});
 it("searches labels, types, and project context",()=>{expect(paletteResults(items,"school",[],[]).map(item=>item.id)).toEqual(["p1","t1","r1"]);expect(paletteResults(items,"rfi",[],[]).map(item=>item.id)).toEqual(["r1"])});
 it("falls back to actual projects when no project recency exists",()=>{expect(paletteResults(items,"",[],[]).some(item=>item.id==="p1"&&item.group==="Projects")).toBe(true)});
});

import { invoke } from "@tauri-apps/api/core";
import { isPermissionGranted, requestPermission, sendNotification } from "@tauri-apps/plugin-notification";

type Task={status:string;dueDate:string|null;followUpDate:string|null};
type WorkItem={status:string;dueDate:string|null;title:string};
const key="workspace.reminders.enabled";
const sentKey="workspace.reminders.lastSent";
function today(){const d=new Date();return `${d.getFullYear()}-${String(d.getMonth()+1).padStart(2,"0")}-${String(d.getDate()).padStart(2,"0")}`}
export function remindersEnabled(){return localStorage.getItem(key)==="true"}
export async function setRemindersEnabled(enabled:boolean){if(enabled){let granted=await isPermissionGranted();if(!granted)granted=(await requestPermission())==="granted";if(!granted)return false;}localStorage.setItem(key,String(enabled));return true}
export async function runReminderCheck(force=false){if(!remindersEnabled())return 0;const date=today();if(!force&&localStorage.getItem(sentKey)===date)return 0;const [tasks,items]=await Promise.all([invoke<Task[]>("list_tasks"),invoke<WorkItem[]>("list_work_items")]);const taskCount=tasks.filter(v=>!["completed","cancelled"].includes(v.status)&&((v.dueDate&&v.dueDate<=date)||(v.followUpDate&&v.followUpDate<=date))).length;const itemCount=items.filter(v=>!["completed","cancelled"].includes(v.status)&&v.dueDate&&v.dueDate<=date).length;const total=taskCount+itemCount;if(total>0)sendNotification({title:"AnyDesk attention",body:`${total} item${total===1?"":"s"} need attention today (${taskCount} tasks, ${itemCount} project controls).`});localStorage.setItem(sentKey,date);return total}

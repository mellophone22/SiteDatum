export const projectControlGroups = [
  { label: "Coordination", registers: ["meeting_minute", "transmittal"] },
  { label: "Commercial", registers: ["procurement", "change_event"] },
  { label: "Schedule / Field", registers: ["milestone", "daily_report", "punch_item"] },
  { label: "Startup & Commissioning", registers: ["startup_check", "commissioning_check", "commissioning_issue"] },
] as const;

export type ProjectControlRegister = typeof projectControlGroups[number]["registers"][number];

export const projectControlRegisters = projectControlGroups.flatMap(group => group.registers) as ProjectControlRegister[];
